use std::sync::{Arc, Mutex};

use base64::Engine;
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{
    CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo,
};
use rmcp::{
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::Deserialize;
use qaly_core::device::keycode;
use qaly_core::testing::Step;
use qaly_core::{AdbDevice, Session, SimError};

#[derive(Default)]
struct ActiveRecording {
    steps: Vec<Step>,
    goal: String,
    test_file: String,
    app: String,
}

#[derive(Clone)]
struct SimServer {
    tool_router: ToolRouter<SimServer>,
    session: Arc<Mutex<Session>>,
    recording: Arc<Mutex<Option<ActiveRecording>>>,
}

fn to_mcp_err(e: SimError) -> McpError {
    match &e {
        SimError::ElementNotFound { .. } | SimError::AmbiguousLabel { .. } => {
            McpError::invalid_params(e.to_string(), None)
        }
        _ => McpError::internal_error(e.to_string(), None),
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LaunchParams {
    /// Android package id, e.g. com.google.android.deskclock
    package: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TargetParams {
    /// Element id (e.g. "e3"), resource-id ("com.pkg:id/foo" or "@foo_tail"), accessibility label, or visible label
    target: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TextParams {
    /// Text to type into the focused field
    text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FillParams {
    /// Element id (e.g. "e3"), resource-id ("com.pkg:id/foo" or "@foo_tail"), accessibility label, or visible label to tap before typing
    target: String,
    /// Text to type
    text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SwipeParams {
    /// up | down | left | right
    direction: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct KeyParams {
    /// back | home | enter | recent
    key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WaitParams {
    /// Label text to wait for
    label: String,
    /// Timeout in milliseconds (default 5000)
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LabelParams {
    /// Label text to assert is visible
    label: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ShellParams {
    /// Raw command to run via `adb shell`, e.g. "dumpsys battery"
    command: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ScreenshotParams {
    /// If true, draw element bboxes + ids (heavier; like perceive). Default false = raw capture.
    #[serde(default)]
    annotated: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct BeginTestParams {
    /// Exact goal text from the .test file
    goal: String,
    /// Path to the .qaly.test file (e.g. "tests/youtube.qaly.test")
    test_file: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct EndTestParams {
    /// Whether the test goal was achieved
    passed: bool,
    /// Optional note (e.g. which element was used when label was ambiguous)
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RunTestParams {
    /// Path to the .qaly.test file
    test_file: String,
    /// Exact goal text of the test to run
    goal: String,
    /// When true, fuzzy-match failing selectors and patch the recording automatically.
    /// Use during development only — never in CI.
    #[serde(default)]
    auto_heal: bool,
    /// Capture debug screenshot on failure; generate HTML report. Defaults to true.
    #[serde(default = "default_true")]
    debug: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RunTestsParams {
    /// Path to the .qaly.test file
    test_file: String,
    /// When true, unrecorded tests are skipped (old behaviour).
    /// Default false — unrecorded tests are recorded automatically.
    #[serde(default)]
    skip_unrecorded: bool,
    /// Number of parallel devices. Default 1 (sequential, existing behaviour).
    #[serde(default)]
    workers: Option<usize>,
    /// Run emulators without a window (headless). Default false.
    #[serde(default)]
    headless: Option<bool>,
    /// AVD name to use when starting new emulators. Required if workers > running devices.
    #[serde(default)]
    avd_name: Option<String>,
    /// When true, fuzzy-match failing selectors and patch the recording automatically.
    /// Use during development only — never in CI.
    #[serde(default)]
    auto_heal: bool,
    /// When true, capture debug screenshots during replay. Artifacts written per failing test
    /// to .qaly/<stem>/last-failure/<goal_slug>/. No images in response (use run_test to investigate).
    /// Defaults to true.
    #[serde(default = "default_true")]
    debug: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SnapshotNameParams {
    /// Snapshot name, e.g. "checkpoint_login"
    name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct HealStepParams {
    /// Path to the .qaly.test file, e.g. "tests/checkout.qaly.test"
    test_file: String,
    /// Exact goal text as it appears in the .qaly.test file
    goal: String,
    /// 0-based index of the step to replace (matches "step N" in failure output minus 1)
    step_index: usize,
    /// Action type: tap | fill | type_text | swipe | key | wait_for | assert_visible | launch | shell
    action: String,
    /// For tap, fill, wait_for, assert_visible: element id (e.g. "e5") or visible label
    #[serde(default)]
    target: Option<String>,
    /// For fill, type_text: text to type
    #[serde(default)]
    text: Option<String>,
    /// For swipe: up | down | left | right
    #[serde(default)]
    direction: Option<String>,
    /// For key: back | home | enter | recent
    #[serde(default)]
    key: Option<String>,
    /// For launch: Android package id, e.g. com.example.app
    #[serde(default)]
    package: Option<String>,
    /// For shell: raw adb shell command
    #[serde(default)]
    command: Option<String>,
    /// For wait_for: timeout in milliseconds (default 5000)
    #[serde(default)]
    timeout_ms: Option<u64>,
}

fn default_true() -> bool { true }

fn swipe_vector(direction: &str) -> Result<(i32, i32, i32, i32), McpError> {
    qaly_core::testing::swipe_coords(direction).map_err(to_mcp_err)
}

fn debug_dir_for_test(test_file: &str) -> std::path::PathBuf {
    qaly_core::testing::default_debug_dir(std::path::Path::new(test_file))
}

/// If auto_start is configured and no emulator is running, start one now.
/// Called automatically before run_test / run_tests.
fn ensure_device() -> Result<(), McpError> {
    let cfg = qaly_core::Config::load();
    if cfg.emulator.auto_start && find_running_emulator().is_none() {
        eprintln!("qaly-mcp: no emulator running — starting '{}'...", cfg.emulator.avd);
        let em_cfg = qaly_core::EmulatorConfig {
            avd_name: cfg.emulator.avd.clone(),
            pool_size: 1,
            headless: cfg.emulator.headless,
            ..qaly_core::EmulatorConfig::default()
        };
        qaly_core::start_pool(&em_cfg)
            .map_err(|e| McpError::internal_error(format!("failed to start emulator: {e}"), None))?;
        eprintln!("qaly-mcp: emulator ready.");
    }
    Ok(())
}

fn params_to_step(p: &HealStepParams) -> Result<qaly_core::testing::Step, McpError> {
    use qaly_core::testing::Step;
    let require_target = || {
        p.target.clone().ok_or_else(|| {
            McpError::invalid_params(
                format!("'target' is required for action '{}'", p.action),
                None,
            )
        })
    };
    let require_text = || {
        p.text.clone().ok_or_else(|| {
            McpError::invalid_params(
                format!("'text' is required for action '{}'", p.action),
                None,
            )
        })
    };
    match p.action.as_str() {
        "tap" => Ok(Step::Tap { target: require_target()? }),
        "fill" => Ok(Step::Fill { target: require_target()?, text: require_text()? }),
        "type_text" => Ok(Step::TypeText { text: require_text()? }),
        "swipe" => Ok(Step::Swipe {
            direction: p.direction.clone().ok_or_else(|| {
                McpError::invalid_params("'direction' is required for action 'swipe'", None)
            })?,
        }),
        "key" => Ok(Step::Key {
            name: p.key.clone().ok_or_else(|| {
                McpError::invalid_params("'key' is required for action 'key'", None)
            })?,
        }),
        "wait_for" => Ok(Step::WaitFor { label: require_target()?, timeout_ms: p.timeout_ms }),
        "assert_visible" => Ok(Step::AssertVisible { label: require_target()? }),
        "launch" => Ok(Step::Launch {
            package: p.package.clone().ok_or_else(|| {
                McpError::invalid_params("'package' is required for action 'launch'", None)
            })?,
        }),
        "shell" => Ok(Step::Shell {
            command: p.command.clone().ok_or_else(|| {
                McpError::invalid_params("'command' is required for action 'shell'", None)
            })?,
        }),
        other => Err(McpError::invalid_params(
            format!("unknown action '{other}'. Valid: tap|fill|type_text|swipe|key|wait_for|assert_visible|launch|shell"),
            None,
        )),
    }
}

#[tool_router]
impl SimServer {
    fn new() -> Self {
        let device: Box<dyn qaly_core::DeviceController> = Self::make_device();
        let mut session = Session::new(device);
        if let Ok(dir) = std::env::var("QALY_RUN_DIR") {
            if let Err(e) = session.enable_logging(std::path::Path::new(&dir)) {
                eprintln!("qaly-mcp: run logging disabled: {e}");
            }
        }
        Self {
            tool_router: Self::tool_router(),
            session: Arc::new(Mutex::new(session)),
            recording: Arc::new(Mutex::new(None)),
        }
    }

    /// Build the device controller: HybridController when `grpc` feature is on,
    /// plain AdbDevice otherwise. Falls back to AdbDevice on any connection error.
    fn make_device() -> Box<dyn qaly_core::DeviceController> {
        #[cfg(feature = "grpc")]
        {
            let adb_bin = std::env::var("ADB_BINARY").unwrap_or_else(|_| "adb".into());
            let grpc_port: u16 = std::env::var("QALY_GRPC_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8554);
            let ws_port: u16 = std::env::var("QALY_WS_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7777);

            let adb = AdbDevice::new();
            let rt = tokio::runtime::Handle::current();

            // block_in_place lets tokio know we're blocking so it can move other
            // tasks off this thread — avoids the "block_on inside runtime" panic.
            match tokio::task::block_in_place(|| rt.block_on(qaly_core::agent_apk::ensure_agent_running(&adb_bin))) {
                Ok(()) => {
                    match tokio::task::block_in_place(|| rt.block_on(qaly_core::try_connect_hybrid(
                        grpc_port,
                        ws_port,
                        adb.clone(),
                    ))) {
                        Some(hybrid) => {
                            eprintln!(
                                "qaly-mcp: HybridController active (gRPC:{grpc_port} WS:{ws_port})"
                            );
                            Box::new(hybrid) as Box<dyn qaly_core::DeviceController>
                        }
                        None => {
                            eprintln!("qaly-mcp: hybrid connect failed, using ADB");
                            Box::new(adb) as Box<dyn qaly_core::DeviceController>
                        }
                    }
                }
                Err(e) => {
                    eprintln!("qaly-mcp: hybrid unavailable ({e}), using ADB");
                    Box::new(adb) as Box<dyn qaly_core::DeviceController>
                }
            }
        }

        #[cfg(not(feature = "grpc"))]
        {
            Box::new(AdbDevice::new()) as Box<dyn qaly_core::DeviceController>
        }
    }

    /// When element resolution fails (ambiguous or not found), capture a screenshot and
    /// return it alongside the error text so the agent can visually pick the right element.
    fn error_with_screenshot(&self, e: SimError) -> CallToolResult {
        let maybe_png = self.session.lock().unwrap().screenshot_raw().ok();
        let mut content = vec![Content::text(e.to_string())];
        if let Some(png) = maybe_png {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
            content.push(Content::image(b64, "image/png".to_string()));
        }
        CallToolResult::success(content)
    }

    #[tool(description = "Capture the current screen. Returns structured elements as JSON (id, role, label, bbox, tappable). \
                       Call screenshot(annotated=true) if you need to see the screen visually.")]
    async fn perceive(&self) -> Result<CallToolResult, McpError> {
        let mut s = self.session.lock().unwrap();
        let (screen, _annotated) = s.perceive().map_err(to_mcp_err)?;
        let mut json = serde_json::to_string_pretty(&screen)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        // Append warnings as a clearly visible block so agents don't miss them.
        if !screen.warnings.is_empty() {
            json.push_str("\n\n⚠️ duplicate actionable label warnings:\n");
            for w in &screen.warnings {
                json.push_str(&format!("  - {}\n", w));
            }
        }
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Capture the screen as PNG. Pass annotated=true to draw element bounding boxes and ids — \
                       use this when you need to see the screen visually (perceive() gives text-only by default).")]
    async fn screenshot(
        &self,
        Parameters(p): Parameters<ScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut s = self.session.lock().unwrap();
        let png = if p.annotated.unwrap_or(false) {
            let (_screen, annotated) = s.perceive().map_err(to_mcp_err)?;
            annotated
        } else {
            s.screenshot_raw().map_err(to_mcp_err)?
        };
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
        Ok(CallToolResult::success(vec![Content::image(
            b64,
            "image/png".to_string(),
        )]))
    }

    #[tool(description = "Launch an app by package id.")]
    async fn launch_app(
        &self,
        Parameters(p): Parameters<LaunchParams>,
    ) -> Result<CallToolResult, McpError> {
        self.session.lock().unwrap().launch(&p.package).map_err(to_mcp_err)?;
        if let Some(rec) = self.recording.lock().unwrap().as_mut() {
            rec.steps.push(Step::Launch { package: p.package.clone() });
        }
        Ok(CallToolResult::success(vec![Content::text(format!("launched {}", p.package))]))
    }

    #[tool(description = "Tap an element. Target can be an element id (e3), resource-id (com.pkg:id/foo or @foo_tail), accessibility label, or visible label.")]
    async fn tap(&self, Parameters(p): Parameters<TargetParams>) -> Result<CallToolResult, McpError> {
        // Resolve a stable selector BEFORE the tap while last_screen is still valid.
        // This converts ephemeral ids like "e10" → "@alarm_fab" so recordings
        // don't break when element order changes between app versions.
        let stable = self.session.lock().unwrap().stable_target(&p.target);
        // Capture result before match so the MutexGuard drops before we re-acquire for screenshot.
        let result = self.session.lock().unwrap().tap(&p.target);
        match result {
            Ok(_) => {
                if let Some(rec) = self.recording.lock().unwrap().as_mut() {
                    rec.steps.push(Step::Tap { target: stable });
                }
                Ok(CallToolResult::success(vec![Content::text(format!("tapped {}", p.target))]))
            }
            Err(e) if matches!(&e, SimError::AmbiguousLabel { .. } | SimError::ElementNotFound { .. }) => {
                Ok(self.error_with_screenshot(e))
            }
            Err(e) => Err(to_mcp_err(e)),
        }
    }

    #[tool(description = "Type text into the currently focused field.")]
    async fn type_text(&self, Parameters(p): Parameters<TextParams>) -> Result<CallToolResult, McpError> {
        self.session.lock().unwrap().type_text(&p.text).map_err(to_mcp_err)?;
        if let Some(rec) = self.recording.lock().unwrap().as_mut() {
            rec.steps.push(Step::TypeText { text: p.text.clone() });
        }
        Ok(CallToolResult::success(vec![Content::text("typed")]))
    }

    #[tool(description = "Tap a target then type text into it. Target can be an element id (e3), resource-id (com.pkg:id/foo or @foo_tail), accessibility label, or visible label.")]
    async fn fill(&self, Parameters(p): Parameters<FillParams>) -> Result<CallToolResult, McpError> {
        // Same stabilization as tap — resolve eN → stable before the action mutates state.
        let stable = self.session.lock().unwrap().stable_target(&p.target);
        let result = self.session.lock().unwrap().fill(&p.target, &p.text);
        match result {
            Ok(_) => {
                if let Some(rec) = self.recording.lock().unwrap().as_mut() {
                    rec.steps.push(Step::Fill { target: stable, text: p.text.clone() });
                }
                Ok(CallToolResult::success(vec![Content::text(format!("filled {}", p.target))]))
            }
            Err(e) if matches!(&e, SimError::AmbiguousLabel { .. } | SimError::ElementNotFound { .. }) => {
                Ok(self.error_with_screenshot(e))
            }
            Err(e) => Err(to_mcp_err(e)),
        }
    }

    #[tool(description = "Swipe in a direction (up|down|left|right) using generic screen-center coordinates.")]
    async fn swipe(&self, Parameters(p): Parameters<SwipeParams>) -> Result<CallToolResult, McpError> {
        let (x1, y1, x2, y2) = swipe_vector(&p.direction)?;
        self.session.lock().unwrap().swipe(x1, y1, x2, y2, 300).map_err(to_mcp_err)?;
        if let Some(rec) = self.recording.lock().unwrap().as_mut() {
            rec.steps.push(Step::Swipe { direction: p.direction.clone() });
        }
        Ok(CallToolResult::success(vec![Content::text(format!("swiped {}", p.direction))]))
    }

    #[tool(description = "Press a hardware/navigation key: back|home|enter|recent.")]
    async fn press_key(&self, Parameters(p): Parameters<KeyParams>) -> Result<CallToolResult, McpError> {
        let code = keycode(&p.key)
            .ok_or_else(|| McpError::invalid_params(format!("unknown key '{}'", p.key), None))?;
        self.session.lock().unwrap().key(code).map_err(to_mcp_err)?;
        if let Some(rec) = self.recording.lock().unwrap().as_mut() {
            rec.steps.push(Step::Key { name: p.key.clone() });
        }
        Ok(CallToolResult::success(vec![Content::text(format!("pressed {}", p.key))]))
    }

    #[tool(description = "Block until a label appears, then return. Default timeout 5000ms.")]
    async fn wait_for(
        &self,
        Parameters(p): Parameters<WaitParams>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = p.timeout_ms.unwrap_or(5000);
        self.session
            .lock()
            .unwrap()
            .wait_for(&p.label, timeout)
            .map_err(to_mcp_err)?;
        if let Some(rec) = self.recording.lock().unwrap().as_mut() {
            rec.steps.push(Step::WaitFor { label: p.label.clone(), timeout_ms: p.timeout_ms });
        }
        Ok(CallToolResult::success(vec![Content::text(format!(
            "'{}' is visible",
            p.label
        ))]))
    }

    #[tool(description = "Assert a label is visible; errors if not found.")]
    async fn assert_visible(
        &self,
        Parameters(p): Parameters<LabelParams>,
    ) -> Result<CallToolResult, McpError> {
        let result = self.session.lock().unwrap().assert_visible(&p.label);
        match result {
            Ok(_) => {
                if let Some(rec) = self.recording.lock().unwrap().as_mut() {
                    rec.steps.push(Step::AssertVisible { label: p.label.clone() });
                }
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "'{}' is visible",
                    p.label
                ))]))
            }
            Err(e @ SimError::ElementNotFound { .. }) => Ok(self.error_with_screenshot(e)),
            Err(e) => Err(to_mcp_err(e)),
        }
    }

    #[tool(description = "Escape hatch: run a raw `adb shell` command and return its stdout.")]
    async fn shell(&self, Parameters(p): Parameters<ShellParams>) -> Result<CallToolResult, McpError> {
        let out = self.session.lock().unwrap().shell(&p.command).map_err(to_mcp_err)?;
        if let Some(rec) = self.recording.lock().unwrap().as_mut() {
            rec.steps.push(Step::Shell { command: p.command.clone() });
        }
        Ok(CallToolResult::success(vec![Content::text(out)]))
    }

    #[tool(description = "Save the current emulator state as a named snapshot. Call before a branch point to enable restore later.")]
    async fn snapshot_save(
        &self,
        Parameters(p): Parameters<SnapshotNameParams>,
    ) -> Result<CallToolResult, McpError> {
        self.session.lock().unwrap().snapshot_save(&p.name).map_err(to_mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(format!("Snapshot '{}' saved.", p.name))]))
    }

    #[tool(description = "Restore the emulator to a previously saved snapshot by name. When a recording is active (begin_test called), this action is recorded as a snapshot_restore step so the test can set its own starting state explicitly.")]
    async fn snapshot_restore(
        &self,
        Parameters(p): Parameters<SnapshotNameParams>,
    ) -> Result<CallToolResult, McpError> {
        self.session.lock().unwrap().snapshot_restore(&p.name).map_err(to_mcp_err)?;
        if let Some(rec) = self.recording.lock().unwrap().as_mut() {
            rec.steps.push(Step::SnapshotRestore { name: p.name.clone() });
        }
        Ok(CallToolResult::success(vec![Content::text(format!("Snapshot '{}' restored.", p.name))]))
    }

    #[tool(description = "List all available snapshots on the current emulator.")]
    async fn snapshot_list(&self) -> Result<CallToolResult, McpError> {
        let names = self.session.lock().unwrap().snapshot_list().map_err(to_mcp_err)?;
        let text = if names.is_empty() {
            "No snapshots.".to_string()
        } else {
            names.join("\n")
        };
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Delete a named snapshot from the current emulator to free space.")]
    async fn snapshot_delete(
        &self,
        Parameters(p): Parameters<SnapshotNameParams>,
    ) -> Result<CallToolResult, McpError> {
        self.session.lock().unwrap().snapshot_delete(&p.name).map_err(to_mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(format!("Snapshot '{}' deleted.", p.name))]))
    }

    #[tool(description = "Replace a failing step in a recording with a corrected action. Call this after run_test fails, manually perform the correct action on the device, then call this to persist the fix. After healing, call run_test again to verify.")]
    async fn heal_step(
        &self,
        Parameters(p): Parameters<HealStepParams>,
    ) -> Result<CallToolResult, McpError> {
        let step = params_to_step(&p)?;
        qaly_core::testing::heal_step(
            std::path::Path::new(&p.test_file),
            &p.goal,
            p.step_index,
            step,
        )
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Step {} of '{}' healed. Run run_test to verify the fix.",
            p.step_index + 1,
            p.goal
        ))]))
    }

    #[tool(description = "Start recording a test. All subsequent action calls are recorded until end_test() is called.")]
    async fn begin_test(
        &self,
        Parameters(p): Parameters<BeginTestParams>,
    ) -> Result<CallToolResult, McpError> {
        let tf = qaly_core::testing::parse_test_file(std::path::Path::new(&p.test_file))
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        *self.recording.lock().unwrap() = Some(ActiveRecording {
            steps: Vec::new(),
            goal: p.goal.clone(),
            test_file: p.test_file.clone(),
            app: tf.app,
        });
        // Duplicates are always fatal during recording: any ambiguous step will
        // fail on replay, so we stop immediately rather than produce a broken recording.
        self.session.lock().unwrap().set_duplicate_policy(
            qaly_core::actions::DuplicatePolicy::Error
        );
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Recording started for '{}'. Call action tools normally, then end_test().", p.goal
        ))]))
    }

    #[tool(description = "Finish recording and save the recording file to .qaly/.")]
    async fn end_test(
        &self,
        Parameters(p): Parameters<EndTestParams>,
    ) -> Result<CallToolResult, McpError> {
        let active = self.recording.lock().unwrap().take()
            .ok_or_else(|| McpError::invalid_params("no active recording — call begin_test first", None))?;
        // Restore default policy now that recording is finished.
        self.session.lock().unwrap().set_duplicate_policy(
            qaly_core::actions::DuplicatePolicy::Warn
        );
        let test_path = std::path::Path::new(&active.test_file);
        let tf = qaly_core::testing::parse_test_file(test_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let index = tf.entries.iter().position(|e| e.goal == active.goal)
            .ok_or_else(|| McpError::internal_error(
                format!("goal '{}' not found in {}", active.goal, active.test_file), None
            ))?;
        let rec_path = qaly_core::testing::recording_path(test_path, index, &active.goal);
        let recording = qaly_core::testing::Recording {
            goal: active.goal.clone(),
            app: active.app.clone(),
            recorded_at: qaly_core::testing::Recording::now_unix(),
            sim_version: env!("CARGO_PKG_VERSION").to_string(),
            steps: active.steps,
        };
        recording.save(&rec_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let status = if p.passed { "✓ passed" } else { "✗ failed" };
        let note = p.message.as_deref().map(|m| format!(" — {m}")).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(format!(
            "{status}{note} · {} steps saved to {}",
            recording.steps.len(),
            rec_path.display()
        ))]))
    }

    #[tool(description = "Replay a recorded test deterministically. Returns pass/fail with details. On failure, attaches the current screen and a heal hint so you can fix the recording without extra round trips.")]
    async fn run_test(
        &self,
        Parameters(p): Parameters<RunTestParams>,
    ) -> Result<CallToolResult, McpError> {
        ensure_device()?;
        let test_path = std::path::Path::new(&p.test_file);
        let tf = qaly_core::testing::parse_test_file(test_path)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let index = tf.entries.iter().position(|e| e.goal == p.goal)
            .ok_or_else(|| McpError::invalid_params(
                format!("goal '{}' not found in {}", p.goal, p.test_file), None
            ))?;
        let rec_path = qaly_core::testing::recording_path(test_path, index, &p.goal);
        if !rec_path.exists() {
            return Ok(CallToolResult::success(vec![Content::text(
                format!("SKIP '{}' — no recording at {}", p.goal, rec_path.display())
            )]));
        }
        let recording = qaly_core::testing::Recording::load(&rec_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let debug_dir: Option<std::path::PathBuf> = if p.debug {
            Some(
                debug_dir_for_test(&p.test_file)
                    .join(qaly_core::testing::goal_to_slug(&p.goal))
            )
        } else {
            None
        };
        let opts = qaly_core::testing::ReplayOpts {
            auto_heal: p.auto_heal,
            heal_test_file: if p.auto_heal { Some(test_path.to_path_buf()) } else { None },
            heal_goal: if p.auto_heal { Some(p.goal.clone()) } else { None },
            debug_artifacts: debug_dir,
        };
        let result = {
            let mut s = self.session.lock().unwrap();
            qaly_core::testing::replay_opts(&recording, &mut s, opts.clone())
        };
        if result.status == qaly_core::testing::TestStatus::Failed {
            let mut text = format_result_with_heal_hint(&result, &p.test_file, &p.goal);
            // Append artifact paths so the agent can request images if needed.
            if let Some(ref report) = result.report_path {
                text.push_str(&format!("\n  report:     {}", report.display()));
            }
            if let Some(trace) = result.step_traces.iter()
                .find(|t| t.status == qaly_core::testing::StepStatus::Failed)
            {
                if let Some(ref png) = trace.screenshot_path {
                    text.push_str(&format!("\n  screenshot: {}", png.display()));
                }
            }
            // Attach a fresh perceive screenshot so the agent sees the current screen state.
            let mut contents = vec![Content::text(text)];
            if let Ok((_, png)) = self.session.lock().unwrap().perceive() {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
                contents.push(Content::image(b64, "image/png".to_string()));
            }
            return Ok(CallToolResult::success(contents));
        }
        Ok(CallToolResult::success(vec![Content::text(qaly_core::testing::format_result(&result))]))
    }

    #[tool(description = "Run all tests in a .qaly.test file. Replays recorded tests and automatically starts recording unrecorded ones (set skip_unrecorded=true to skip them instead).")]
    async fn run_tests(
        &self,
        Parameters(p): Parameters<RunTestsParams>,
    ) -> Result<CallToolResult, McpError> {
        ensure_device()?;
        if self.recording.lock().unwrap().is_some() {
            return Err(McpError::invalid_params(
                "a recording is already active — call end_test() first",
                None,
            ));
        }
        let test_path = std::path::Path::new(&p.test_file);
        let tf = qaly_core::testing::parse_test_file(test_path)
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        qaly_core::testing::cleanup_orphaned_recordings(test_path, &tf.goals())
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let replay_opts = qaly_core::testing::ReplayOpts {
            auto_heal: p.auto_heal,
            heal_test_file: if p.auto_heal { Some(test_path.to_path_buf()) } else { None },
            heal_goal: None,  // overridden per-test inside the runner
            debug_artifacts: if p.debug {
                Some(debug_dir_for_test(&p.test_file))
            } else {
                None
            },
        };

        let workers = p.workers.unwrap_or(1);
        if workers > 1 {
            let avd_name = p.avd_name.clone().unwrap_or_else(|| {
                std::env::var("AVD_NAME").unwrap_or_else(|_| "Medium_Phone_API_36.1".into())
            });
            let headless = p.headless.unwrap_or(false);
            let adb_bin = std::env::var("ADB_BINARY").unwrap_or_else(|_| "adb".into());

            let par_results: Vec<qaly_core::testing::TestResult> = if tf.fixtures.is_empty() {
                // ── No fixtures: all tests parallel with a common base snapshot ──────
                // Use a single shared pool (reuse-first, no -read-only needed).
                if matches!(tf.clean_state, qaly_core::testing::CleanStateConfig::AppData) {
                    eprintln!("qaly: warning: clean_state 'app_data' is not supported with \
                               --workers > 1 in no-fixture parallel mode (base snapshot will be \
                               restored instead). Remove --workers or use a fixture-based layout.");
                }
                let config = qaly_core::EmulatorConfig {
                    avd_name: avd_name.clone(),
                    pool_size: workers,
                    headless,
                    ..qaly_core::EmulatorConfig::default()
                };
                let (pool, managed) = qaly_core::start_pool(&config)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                let base_snapshot = format!("qaly_base_{}", qaly_core::testing::Recording::now_unix());
                let pool_size = pool.size();
                let mut snap_devices = Vec::new();
                for _ in 0..pool_size {
                    let device = pool.acquire();
                    device.snapshot_save(&base_snapshot)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    snap_devices.push(device);
                }
                for device in snap_devices { pool.release(device); }

                let mut recorded: Vec<(usize, qaly_core::testing::Recording)> = Vec::new();
                let mut unrecorded_r: Vec<(usize, qaly_core::testing::TestResult)> = Vec::new();
                for (i, entry) in tf.entries.iter().enumerate() {
                    let goal = &entry.goal;
                    let rec_path = qaly_core::testing::recording_path(test_path, i, goal);
                    if rec_path.exists() {
                        match qaly_core::testing::Recording::load(&rec_path) {
                            Ok(rec) => recorded.push((i, rec)),
                            Err(e) => unrecorded_r.push((i, qaly_core::testing::TestResult::load_error(goal, &e))),
                        }
                    } else {
                        unrecorded_r.push((i, qaly_core::testing::TestResult::unrecorded(goal)));
                    }
                }
                let refs: Vec<(usize, &qaly_core::testing::Recording)> =
                    recorded.iter().map(|(i, r)| (*i, r)).collect();
                let mut par = qaly_core::testing::run_tests_parallel(&refs, &pool, &base_snapshot, replay_opts.clone());
                par.extend(unrecorded_r);
                par.sort_by_key(|(i, _)| *i);
                qaly_core::stop_pool(managed)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                par.into_iter().map(|(_, r)| r).collect()

            } else {
                // ── Fixtures: Phase 1 sequential + Phase 2 parallel per fixture ──────
                //
                // All emulators run as -read-only (Android requires this for multiple
                // instances of the same AVD).  Phase 1 boots from qaly_clean_state so
                // tests start from a known state without a runtime snapshot_restore
                // (which -read-only blocks).  Phase 2 boots directly from each fixture
                // snapshot.  We kill any running non-read-only emulator first.

                // Check on disk — no running emulator required.
                let clean_snap_exists = qaly_core::snapshot_exists_on_disk(
                    &avd_name, qaly_core::CLEAN_STATE_SNAPSHOT,
                );

                if !clean_snap_exists {
                    eprintln!(
                        "qaly: warning — '{}' snapshot not found; Phase 1 will \
                         cold-boot (tests may start from wrong state).",
                        qaly_core::CLEAN_STATE_SNAPSHOT
                    );
                }

                // Kill any running emulator so our -read-only managed ones can start.
                qaly_core::kill_all_running_emulators(&adb_bin);

                // ── Phase 1: one -read-only device, sequential ────────────────────
                let p1_config = qaly_core::EmulatorConfig {
                    avd_name: avd_name.clone(),
                    pool_size: 1,
                    headless,
                    boot_snapshot: if clean_snap_exists {
                        Some(qaly_core::CLEAN_STATE_SNAPSHOT.into())
                    } else {
                        None
                    },
                    ..qaly_core::EmulatorConfig::default()
                };
                let (p1_pool, p1_managed) = qaly_core::start_pool(&p1_config)
                    .map_err(|e| McpError::internal_error(
                        format!("failed to start Phase 1 emulator: {e}"), None,
                    ))?;

                let p1_device = p1_pool.acquire();
                let mut p1_session = qaly_core::Session::new(p1_device);

                let mut p1_items: Vec<(usize, Option<qaly_core::testing::Recording>, &qaly_core::testing::TestEntry)> = Vec::new();
                let mut p1_errors: std::collections::HashMap<usize, qaly_core::testing::TestResult> = std::collections::HashMap::new();

                for (i, entry) in tf.entries.iter().enumerate() {
                    if entry.fixture.is_some() { continue; }
                    let goal = &entry.goal;
                    let rec_path = qaly_core::testing::recording_path(test_path, i, goal);
                    if rec_path.exists() {
                        match qaly_core::testing::Recording::load(&rec_path) {
                            Ok(rec) => p1_items.push((i, Some(rec), entry)),
                            Err(e) => { p1_errors.insert(i, qaly_core::testing::TestResult::load_error(goal, &e)); }
                        }
                    } else {
                        p1_items.push((i, None, entry));
                    }
                }

                let p1_pairs = qaly_core::testing::run_entries_sequential(
                    &p1_items, &tf, &mut p1_session, replay_opts.clone(),
                );
                let p1_device = p1_session.into_device();
                p1_pool.release(p1_device);
                qaly_core::stop_pool(p1_managed)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;

                // ── Phase 2: N workers per fixture group, booted from snapshot ─────
                // BTreeMap gives deterministic group-processing order across runs.
                let mut fixture_groups: std::collections::BTreeMap<
                    String, Vec<(usize, qaly_core::testing::Recording)>
                > = std::collections::BTreeMap::new();
                let mut p2_errors: Vec<(usize, qaly_core::testing::TestResult)> = Vec::new();

                for (i, entry) in tf.entries.iter().enumerate() {
                    let Some(fixture_name) = &entry.fixture else { continue };
                    let goal = &entry.goal;
                    let rec_path = qaly_core::testing::recording_path(test_path, i, goal);
                    if rec_path.exists() {
                        match qaly_core::testing::Recording::load(&rec_path) {
                            Ok(rec) => {
                                fixture_groups
                                    .entry(fixture_name.clone())
                                    .or_default()
                                    .push((i, rec));
                            }
                            Err(e) => p2_errors.push((i, qaly_core::testing::TestResult::load_error(goal, &e))),
                        }
                    } else {
                        p2_errors.push((i, qaly_core::testing::TestResult::unrecorded(goal)));
                    }
                }

                let mut p2_pairs: Vec<(usize, qaly_core::testing::TestResult)> = p2_errors;

                for (fixture_name, group) in &fixture_groups {
                    let snap_name = format!("qaly_fixture_{fixture_name}");
                    let workers_for_group = group.len().min(workers);
                    let p2_config = qaly_core::EmulatorConfig {
                        avd_name: avd_name.clone(),
                        pool_size: workers_for_group,
                        headless,
                        boot_snapshot: Some(snap_name.clone()),
                        ..qaly_core::EmulatorConfig::default()
                    };
                    let (p2_pool, p2_managed) = qaly_core::start_pool(&p2_config)
                        .map_err(|e| McpError::internal_error(
                            format!("failed to start Phase 2 workers for fixture '{fixture_name}': {e}"),
                            None,
                        ))?;

                    let refs: Vec<(usize, &qaly_core::testing::Recording)> =
                        group.iter().map(|(i, r)| (*i, r)).collect();
                    let mut group_results =
                        qaly_core::testing::run_preloaded_tests_parallel(&refs, &p2_pool, replay_opts.clone());
                    p2_pairs.append(&mut group_results);

                    qaly_core::stop_pool(p2_managed)
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                }

                // Merge results
                let mut all: std::collections::HashMap<usize, qaly_core::testing::TestResult> = p1_errors;
                for (idx, r) in p1_pairs { all.insert(idx, r); }
                for (idx, r) in p2_pairs { all.insert(idx, r); }
                let mut sorted: Vec<(usize, qaly_core::testing::TestResult)> = all.into_iter().collect();
                sorted.sort_by_key(|(i, _)| *i);
                sorted.into_iter().map(|(_, r)| r).collect()
            };

            let report = qaly_core::testing::format_report(&par_results);
            return Ok(CallToolResult::success(vec![Content::text(report)]));
        }

        // Build items for run_entries_sequential, handling auto-record intercept.
        let mut items: Vec<(usize, Option<qaly_core::testing::Recording>, &qaly_core::testing::TestEntry)> = Vec::new();
        let mut pre_results: std::collections::HashMap<usize, qaly_core::testing::TestResult> = std::collections::HashMap::new();

        for (i, entry) in tf.entries.iter().enumerate() {
            let goal = &entry.goal;
            let rec_path = qaly_core::testing::recording_path(test_path, i, goal);
            if !rec_path.exists() {
                if p.skip_unrecorded {
                    items.push((i, None, entry));
                    continue;
                }
                // Auto-record: pause and ask agent to complete this goal then call end_test().
                let recorded_so_far: Vec<qaly_core::testing::TestResult> = {
                    let mut v: Vec<(usize, qaly_core::testing::TestResult)> = pre_results
                        .iter().map(|(k, r)| (*k, r.clone())).collect();
                    v.sort_by_key(|(k, _)| *k);
                    v.into_iter().map(|(_, r)| r).collect()
                };
                let partial = format_results_table(&recorded_so_far);
                let prompt = format!(
                    "{}Recording started for '{}'. Complete the goal on the device \
                     using the available tools (perceive, tap, fill, etc.), then call \
                     end_test(). After that, call run_tests(test_file=\"{}\") to continue.",
                    partial, goal, p.test_file
                );
                *self.recording.lock().unwrap() = Some(ActiveRecording {
                    steps: Vec::new(),
                    goal: goal.clone(),
                    test_file: p.test_file.clone(),
                    app: tf.app.clone(),
                });
                return Ok(CallToolResult::success(vec![Content::text(prompt)]));
            }
            match qaly_core::testing::Recording::load(&rec_path) {
                Ok(rec) => items.push((i, Some(rec), entry)),
                Err(e) => {
                    pre_results.insert(i, qaly_core::testing::TestResult::load_error(goal, &e));
                }
            }
        }

        // Run with fixture save/restore.
        let seq_pairs = qaly_core::testing::run_entries_sequential(
            &items,
            &tf,
            &mut self.session.lock().unwrap(),
            replay_opts,
        );
        let mut all: std::collections::HashMap<usize, qaly_core::testing::TestResult> = pre_results;
        for (idx, r) in seq_pairs { all.insert(idx, r); }
        let mut sorted: Vec<(usize, qaly_core::testing::TestResult)> = all.into_iter().collect();
        sorted.sort_by_key(|(i, _)| *i);
        let results: Vec<qaly_core::testing::TestResult> = sorted.into_iter().map(|(_, r)| r).collect();
        let report = qaly_core::testing::format_report(&results);
        Ok(CallToolResult::success(vec![Content::text(report)]))
    }
}

fn format_result_with_heal_hint(
    r: &qaly_core::testing::TestResult,
    test_file: &str,
    goal: &str,
) -> String {
    // Reuse the shared formatter for the base text, then append the heal hint.
    let mut s = qaly_core::testing::format_result(r);
    if let Some(fs) = &r.failed_step {
        s.push_str(&format!(
            "\n\nTo heal: call heal_step(test_file=\"{test_file}\", goal=\"{goal}\", \
             step_index={}, action=\"{}\", <corrected params>)\n\
             Current screen attached below.",
            fs.index, fs.action
        ));
    }
    s
}

fn format_results_table(results: &[qaly_core::testing::TestResult]) -> String {
    if results.is_empty() {
        return String::new();
    }
    let bar = "━".repeat(60);
    let mut lines = vec!["Progress so far:".to_string(), bar.clone()];
    for r in results {
        lines.push(qaly_core::testing::format_result(r));
    }
    lines.push(bar);
    lines.push(String::new()); // blank line before the recording message
    lines.join("\n")
}

#[tool_handler]
impl ServerHandler for SimServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::V_2025_06_18,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Drive Android devices with Qaly. Call perceive() to see the screen, then tap/fill/etc."
                    .to_string(),
            ),
        }
    }
}

/// Returns the serial of a running emulator, or None if no emulators are running.
fn find_running_emulator() -> Option<String> {
    let adb = std::env::var("ADB_BINARY").unwrap_or_else(|_| "adb".into());
    qaly_core::list_emulator_serials(&adb).ok()?.into_iter().next()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = qaly_core::Config::load();
    cfg.apply_env();
    let service = SimServer::new().serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;

    Ok(())
}
