mod client;

use std::sync::Arc;

use base64::Engine;
use client::proto;
use client::DaemonClient;
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler, ServiceExt};
use serde::Deserialize;
use tokio::sync::Mutex;

#[derive(Clone)]
struct SimServer {
    tool_router: ToolRouter<SimServer>,
    client: Arc<Mutex<DaemonClient>>,
}

/// Map a gRPC `Status` into an MCP error. NotFound / InvalidArgument become
/// invalid_params; everything else becomes an internal error.
fn grpc_to_mcp_err(s: tonic::Status) -> McpError {
    match s.code() {
        tonic::Code::NotFound | tonic::Code::InvalidArgument => {
            McpError::invalid_params(s.message().to_string(), None)
        }
        _ => McpError::internal_error(s.message().to_string(), None),
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
    /// Currently ignored — the daemon returns a raw capture. Reserved for future annotation support.
    #[serde(default)]
    #[allow(dead_code)]
    annotated: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct BeginTestParams {
    /// Exact goal text from the .test file
    goal: String,
    /// Path to the .qaly.test file (e.g. "tests/youtube.qaly.test")
    test_file: String,
    /// App package under test (recorded into the test recording)
    #[serde(default)]
    app: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct EndTestParams {
    /// Whether the test goal was achieved (informational; recording is always saved)
    #[serde(default)]
    passed: Option<bool>,
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
    /// Capture debug screenshot on failure; generate HTML report. Defaults to true.
    #[serde(default = "default_true")]
    debug: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RunTestsParams {
    /// Path to the .qaly.test file
    test_file: String,
    /// Only run tests whose goal contains this substring.
    #[serde(default)]
    filter: Option<String>,
    /// Number of parallel devices. Default 1 (sequential).
    #[serde(default)]
    workers: Option<u32>,
    /// Run emulators without a window (headless). Default false.
    #[serde(default)]
    headless: Option<bool>,
    /// AVD name to use when starting new emulators.
    #[serde(default)]
    avd_name: Option<String>,
    /// When true, fuzzy-match failing selectors and patch the recording automatically.
    /// Use during development only — never in CI.
    #[serde(default)]
    auto_heal: bool,
    /// When true, capture debug screenshots during replay. Defaults to true.
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
    step_index: u32,
    /// Action type: tap | fill | type_text | swipe | key | wait_for | assert_visible | launch | shell
    action: String,
    /// For tap, fill, wait_for, assert_visible: element id (e.g. "e5") or visible label
    #[serde(default)]
    target: Option<String>,
    /// For fill, type_text: text to type
    #[serde(default)]
    text: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Map a direction name to the proto SwipeDirection enum value.
fn swipe_direction_value(direction: &str) -> Result<i32, McpError> {
    match direction {
        "up" => Ok(proto::SwipeDirection::Up as i32),
        "down" => Ok(proto::SwipeDirection::Down as i32),
        "left" => Ok(proto::SwipeDirection::Left as i32),
        "right" => Ok(proto::SwipeDirection::Right as i32),
        other => Err(McpError::invalid_params(
            format!("unknown swipe direction '{other}'. Valid: up|down|left|right"),
            None,
        )),
    }
}

/// Render a proto TestResult into human-readable text similar to the old
/// core formatter.
fn format_test_result(r: &proto::TestResult) -> String {
    let status = match proto::TestStatus::try_from(r.status) {
        Ok(proto::TestStatus::Pass) => "PASS",
        Ok(proto::TestStatus::Fail) => "FAIL",
        Ok(proto::TestStatus::Skip) => "SKIP",
        Err(_) => "????",
    };
    let mut s = format!("{status}  {}", r.test_name);
    if let Some(fs) = &r.failed_step {
        s.push_str(&format!("\n  step {}: {}", fs.index, fs.reason));
    }
    if let Some(ref report) = r.debug_report_path {
        if !report.is_empty() {
            s.push_str(&format!("\n  report: {report}"));
        }
    }
    s
}

#[tool_router]
impl SimServer {
    fn new(client: DaemonClient) -> Self {
        Self {
            tool_router: Self::tool_router(),
            client: Arc::new(Mutex::new(client)),
        }
    }

    #[tool(description = "Capture the current screen. Returns structured elements as JSON (id, label, resource_id, accessibility_label, bounds, clickable, editable). \
                       Call screenshot() if you need to see the screen visually.")]
    async fn perceive(&self) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .perceive(proto::PerceiveRequest {
                session_id,
                text_only: true,
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        let json = serde_json::to_string_pretty(&resp.elements)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Capture the screen as PNG. Returns a raw screen capture image.")]
    async fn screenshot(
        &self,
        Parameters(_p): Parameters<ScreenshotParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .screenshot(proto::ScreenshotRequest { session_id })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&resp.png);
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
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .launch_app(proto::LaunchRequest {
                session_id,
                package: p.package.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(resp.message)]))
    }

    #[tool(description = "Tap an element. Target can be an element id (e3), resource-id (com.pkg:id/foo or @foo_tail), accessibility label, or visible label.")]
    async fn tap(&self, Parameters(p): Parameters<TargetParams>) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .tap(proto::TapRequest {
                session_id,
                target: p.target.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(resp.message)]))
    }

    #[tool(description = "Type text into the currently focused field.")]
    async fn type_text(
        &self,
        Parameters(p): Parameters<TextParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .type_text(proto::TypeTextRequest {
                session_id,
                text: p.text.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(resp.message)]))
    }

    #[tool(description = "Tap a target then type text into it. Target can be an element id (e3), resource-id (com.pkg:id/foo or @foo_tail), accessibility label, or visible label.")]
    async fn fill(&self, Parameters(p): Parameters<FillParams>) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .fill(proto::FillRequest {
                session_id,
                target: p.target.clone(),
                text: p.text.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(resp.message)]))
    }

    #[tool(description = "Swipe in a direction (up|down|left|right) using generic screen-center coordinates.")]
    async fn swipe(
        &self,
        Parameters(p): Parameters<SwipeParams>,
    ) -> Result<CallToolResult, McpError> {
        let direction = swipe_direction_value(&p.direction)?;
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .swipe(proto::SwipeRequest {
                session_id,
                direction,
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(resp.message)]))
    }

    #[tool(description = "Press a hardware/navigation key: back|home|enter|recent.")]
    async fn press_key(
        &self,
        Parameters(p): Parameters<KeyParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .press_key(proto::PressKeyRequest {
                session_id,
                keycode: p.key.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(resp.message)]))
    }

    #[tool(description = "Block until a label appears, then return. Default timeout 5000ms.")]
    async fn wait_for(
        &self,
        Parameters(p): Parameters<WaitParams>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = p.timeout_ms.unwrap_or(5000) as u32;
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .wait_for(proto::WaitForRequest {
                session_id,
                target: p.label.clone(),
                timeout_ms: timeout,
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(resp.message)]))
    }

    #[tool(description = "Assert a label is visible; errors if not found.")]
    async fn assert_visible(
        &self,
        Parameters(p): Parameters<LabelParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .assert_visible(proto::AssertRequest {
                session_id,
                target: p.label.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(resp.message)]))
    }

    #[tool(description = "Escape hatch: run a raw `adb shell` command and return its stdout.")]
    async fn shell(
        &self,
        Parameters(p): Parameters<ShellParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .device
            .shell(proto::ShellRequest {
                session_id,
                command: p.command.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(resp.stdout)]))
    }

    #[tool(description = "Save the current emulator state as a named snapshot. Call before a branch point to enable restore later.")]
    async fn snapshot_save(
        &self,
        Parameters(p): Parameters<SnapshotNameParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        c.snapshot
            .save(proto::SnapshotRequest {
                session_id,
                name: p.name.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Snapshot '{}' saved.",
            p.name
        ))]))
    }

    #[tool(description = "Restore the emulator to a previously saved snapshot by name.")]
    async fn snapshot_restore(
        &self,
        Parameters(p): Parameters<SnapshotNameParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        c.snapshot
            .restore(proto::SnapshotRequest {
                session_id,
                name: p.name.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Snapshot '{}' restored.",
            p.name
        ))]))
    }

    #[tool(description = "List all available snapshots on the current emulator.")]
    async fn snapshot_list(&self) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .snapshot
            .list(proto::SessionId { id: session_id })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        let text = if resp.names.is_empty() {
            "No snapshots.".to_string()
        } else {
            resp.names.join("\n")
        };
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Delete a named snapshot from the current emulator to free space.")]
    async fn snapshot_delete(
        &self,
        Parameters(p): Parameters<SnapshotNameParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        c.snapshot
            .delete(proto::SnapshotRequest {
                session_id,
                name: p.name.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Snapshot '{}' deleted.",
            p.name
        ))]))
    }

    #[tool(description = "Replace a failing step in a recording with a corrected action. Call this after run_test fails, then call run_test again to verify.")]
    async fn heal_step(
        &self,
        Parameters(p): Parameters<HealStepParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let resp = c
            .test
            .heal_step(proto::HealRequest {
                file: p.test_file.clone(),
                goal: p.goal.clone(),
                step_index: p.step_index,
                action: Some(p.action.clone()),
                target: p.target.clone(),
                text: p.text.clone(),
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        let msg = if resp.success {
            format!(
                "Step {} of '{}' healed (new target: {}). Run run_test to verify the fix.",
                resp.step_index + 1,
                p.goal,
                resp.new_target
            )
        } else {
            format!(
                "Step {} of '{}' was not healed automatically.",
                resp.step_index + 1,
                p.goal
            )
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    #[tool(description = "Start recording a test. All subsequent action calls are recorded until end_test() is called.")]
    async fn begin_test(
        &self,
        Parameters(p): Parameters<BeginTestParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        c.test
            .begin_test(proto::BeginTestRequest {
                session_id,
                goal: p.goal.clone(),
                test_file: p.test_file.clone(),
                app: p.app.clone().unwrap_or_default(),
            })
            .await
            .map_err(grpc_to_mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Recording started for '{}'. Call action tools normally, then end_test().",
            p.goal
        ))]))
    }

    #[tool(description = "Finish recording and save the recording file to .qaly/.")]
    async fn end_test(
        &self,
        Parameters(p): Parameters<EndTestParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let resp = c
            .test
            .end_test(proto::SessionId { id: session_id })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        let status = match p.passed {
            Some(true) => "✓ passed",
            Some(false) => "✗ failed",
            None => "saved",
        };
        let note = p
            .message
            .as_deref()
            .map(|m| format!(" — {m}"))
            .unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(format!(
            "{status}{note} · {} steps saved for '{}'",
            resp.steps.len(),
            resp.test_name
        ))]))
    }

    #[tool(description = "Replay a recorded test deterministically. Returns pass/fail with details.")]
    async fn run_test(
        &self,
        Parameters(p): Parameters<RunTestParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let session_id = c.session_id.clone();
        let result = c
            .test
            .run_test(proto::RunTestRequest {
                session_id,
                test_file: p.test_file.clone(),
                test_name: p.goal.clone(),
                debug: p.debug,
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();
        Ok(CallToolResult::success(vec![Content::text(
            format_test_result(&result),
        )]))
    }

    #[tool(description = "Run all tests in a .qaly.test file. Replays recorded tests; manages its own device pool. Returns a PASS/FAIL/SKIP report.")]
    async fn run_tests(
        &self,
        Parameters(p): Parameters<RunTestsParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut c = self.client.lock().await;
        let mut stream = c
            .test
            .run_tests(proto::RunTestsRequest {
                test_file: p.test_file.clone(),
                filter: p.filter.clone(),
                workers: p.workers.unwrap_or(1),
                headless: p.headless.unwrap_or(false),
                avd_name: p.avd_name.clone(),
                auto_heal: p.auto_heal,
                debug: p.debug,
            })
            .await
            .map_err(grpc_to_mcp_err)?
            .into_inner();

        let mut results: Vec<proto::TestResult> = Vec::new();
        while let Some(event) = stream.message().await.map_err(grpc_to_mcp_err)? {
            if let Some(proto::test_event::Event::Result(r)) = event.event {
                results.push(r);
            }
        }

        let mut report = String::new();
        let mut pass = 0;
        let mut fail = 0;
        let mut skip = 0;
        for r in &results {
            report.push_str(&format_test_result(r));
            report.push('\n');
            match proto::TestStatus::try_from(r.status) {
                Ok(proto::TestStatus::Pass) => pass += 1,
                Ok(proto::TestStatus::Fail) => fail += 1,
                Ok(proto::TestStatus::Skip) => skip += 1,
                Err(_) => {}
            }
        }
        report.push_str(&format!(
            "\n{pass} passed, {fail} failed, {skip} skipped"
        ));
        Ok(CallToolResult::success(vec![Content::text(report)]))
    }
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let daemon = DaemonClient::connect().await?;
    let server = SimServer::new(daemon);
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
