use std::fs;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use qaly_core::device::keycode;
use qaly_core::{AdbDevice, Session};

mod setup_helpers {
    use std::path::{Path, PathBuf};

    /// Architecture string for the system image based on the current host.
    pub fn host_arch() -> &'static str {
        if cfg!(target_arch = "aarch64") { "arm64-v8a" } else { "x86_64" }
    }

    pub fn sdk_manager_path(sdk_root: &Path) -> PathBuf {
        sdk_root.join("cmdline-tools/latest/bin/sdkmanager")
    }

    /// Extract AVD names from `avdmanager list avd` output.
    /// Only used in unit tests.
    #[cfg(test)]
    pub fn parse_avd_names(output: &str) -> Vec<&str> {
        output
            .lines()
            .filter_map(|l| {
                let t = l.trim();
                t.strip_prefix("Name: ")
            })
            .collect()
    }
}

#[derive(Parser)]
#[command(name = "qaly", about = "Agent-friendly Android control (twin of qaly-mcp)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List attached devices.
    Devices,
    /// Launch an app by package id.
    Launch { package: String },
    /// Capture screen; prints JSON, optionally writes annotated PNG.
    Perceive {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        out: Option<String>,
    },
    /// Tap an element by id (e3) or label.
    Tap { target: String },
    /// Type text into the focused field.
    Type { text: String },
    /// Tap a target then type text.
    Fill { target: String, text: String },
    /// Swipe in a direction: up|down|left|right.
    Swipe { direction: String },
    /// Press a key: back|home|enter|recent.
    Key { name: String },
    /// Block until a label appears.
    WaitFor {
        label: String,
        #[arg(long, default_value_t = 5000)]
        timeout_ms: u64,
    },
    /// Assert a label is visible (non-zero exit if not).
    AssertVisible { label: String },
    /// Run a raw adb shell command and print its stdout.
    Shell { command: String },
    /// Run a .qaly.test file: replay recorded tests and print a report.
    Test {
        /// Path to the .qaly.test file
        file: String,
        /// Only run tests whose goal contains this substring
        #[arg(long)]
        filter: Option<String>,
        /// Number of parallel devices (default 1 = sequential)
        #[arg(long)]
        workers: Option<usize>,
        /// Run new emulators without a window
        #[arg(long)]
        headless: bool,
        /// AVD name to use when starting new emulators
        #[arg(long)]
        avd: Option<String>,
        /// Fail tests on duplicate actionable labels (overrides test file setting).
        #[arg(long)]
        strict_labels: bool,
        /// Fuzzy-match failing selectors and patch the recording automatically.
        /// Use during development only — never in CI.
        #[arg(long)]
        auto_heal: bool,
        /// Capture a rolling screenshot buffer during replay; write PNG artifacts to
        /// .qaly/<stem>/last-failure/ when a test fails. Use during development — never in CI.
        #[arg(long)]
        debug: bool,
        /// Same as --debug but write artifacts to a custom path.
        /// If both --debug and --debug-dir are given, --debug-dir wins.
        #[arg(long)]
        debug_dir: Option<String>,
    },
    /// Interactive setup wizard: detect prerequisites, install what's missing,
    /// register qaly-mcp with your AI agent, and create a sample test.
    Init,
    /// Report environment status without modifying anything.
    Doctor,
    /// (deprecated) Alias for `init`.
    #[command(hide = true)]
    Setup,
    /// (deprecated) Alias for `doctor`.
    #[command(hide = true)]
    SetupCheck,
    /// Rename .sim/ recording directories to .qaly/ in the current directory tree.
    Migrate,
}

fn swipe_vector(direction: &str) -> Result<(i32, i32, i32, i32)> {
    qaly_core::testing::swipe_coords(direction)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

fn run_init(check_only: bool) -> Result<()> {
    use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    use qaly_core::setup::{
        detect::detect,
        register::{register_mcp_agent, write_generic_snippet, AgentConfig},
        sample::write_sample_test,
    };
    use std::path::PathBuf;

    let mp = MultiProgress::new();
    let spinner_style = ProgressStyle::with_template("{spinner:.green} {wide_msg}")
        .unwrap()
        .tick_strings(&["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]);
    let done_style = ProgressStyle::with_template("{prefix:.green} {wide_msg}").unwrap();
    let warn_style = ProgressStyle::with_template("{prefix:.yellow} {wide_msg}").unwrap();
    let fail_style = ProgressStyle::with_template("{prefix:.red} {wide_msg}").unwrap();

    let mk = |msg: &str| -> ProgressBar {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(spinner_style.clone());
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb
    };
    let finish_ok = |pb: &ProgressBar, msg: String| {
        pb.set_style(done_style.clone());
        pb.set_prefix("✓");
        pb.finish_with_message(msg);
    };
    let finish_warn = |pb: &ProgressBar, msg: String| {
        pb.set_style(warn_style.clone());
        pb.set_prefix("◦");
        pb.finish_with_message(msg);
    };
    let finish_fail = |pb: &ProgressBar, msg: String| {
        pb.set_style(fail_style.clone());
        pb.set_prefix("✗");
        pb.finish_with_message(msg);
    };

    println!();
    println!("  \x1b[34;1m⬥ qaly init\x1b[0m");
    println!("  Setting up qaly for Android testing...");
    println!();

    let home = dirs::home_dir().context("cannot determine home directory")?;

    // ── 1. Detect everything ─────────────────────────────────────────────────
    let pb = mk("Detecting environment...");
    let mut detected = detect(&home);
    pb.finish_and_clear();

    // ── 2. ADB (required) ────────────────────────────────────────────────────
    let adb_path: PathBuf = if let Some(ref p) = detected.adb_path {
        let pb = mp.add(ProgressBar::new(0));
        finish_ok(&pb, format!("ADB detected   {}", p.display()));
        p.clone()
    } else if check_only {
        let pb = mp.add(ProgressBar::new(0));
        finish_fail(&pb, "ADB not found".into());
        eprintln!("\n  qaly requires adb. Install Android platform-tools:\n    brew install android-platform-tools\n  Then run qaly init again.\n");
        std::process::exit(1);
    } else {
        let pb = mp.add(ProgressBar::new(0));
        finish_fail(&pb, "ADB not found".into());
        let install = dialoguer::Confirm::new()
            .with_prompt("  Install Android platform-tools automatically?")
            .default(true)
            .interact()
            .unwrap_or(false);
        if install {
            let sdk_target = home.join(".qaly/sdk");
            let pb2 = mk("Downloading platform-tools...");
            install_cmdline_tools(&sdk_target)?;
            let new_adb = sdk_target.join("platform-tools/adb");
            finish_ok(&pb2, format!("ADB installed  {}", new_adb.display()));
            detected = detect(&home);
            detected.adb_path.clone().unwrap_or(new_adb)
        } else {
            eprintln!("\n  ADB is required to use qaly.\n  Install it and run qaly init again.\n");
            std::process::exit(1);
        }
    };

    // ── 3. Emulator (required) ───────────────────────────────────────────────
    let has_running = !detected.running_emulator_serials.is_empty();
    let has_avds = !detected.available_avds.is_empty();
    if has_running {
        let pb = mp.add(ProgressBar::new(0));
        let serial = &detected.running_emulator_serials[0];
        finish_ok(&pb, format!("Emulator running  {serial}"));
    } else if has_avds {
        let avd = &detected.available_avds[0];
        let pb = mp.add(ProgressBar::new(0));
        finish_fail(&pb, "No emulator running".into());
        eprintln!("\n  Start your emulator:\n    emulator -avd {avd} -no-window -no-audio &\n  Then run qaly init again.\n");
        std::process::exit(1);
    } else {
        let pb = mp.add(ProgressBar::new(0));
        finish_fail(&pb, "No emulator or AVD found".into());
        eprintln!("\n  No Android emulator found. Create one in Android Studio or via:\n    avdmanager create avd -n qaly-default -k \"system-images;android-36;google_apis;x86_64\"\n  Then run qaly init again.\n");
        std::process::exit(1);
    }

    if check_only { return Ok(()); }

    // ── 4. Write config ───────────────────────────────────────────────────────
    {
        let pb = mk("Writing config...");
        let mut cfg = qaly_core::Config::default();
        cfg.sdk.adb_binary = Some(adb_path.clone());
        if let Some(avd) = detected.available_avds.first() {
            cfg.emulator.avd = avd.clone();
        }
        cfg.save().context("failed to write config")?;
        finish_ok(&pb, format!("Config written   {}", qaly_core::Config::path().display()));
    }

    // ── 5. MCP registration (optional) ───────────────────────────────────────
    let mcp_binary = detected.mcp_binary.clone().unwrap_or_else(|| PathBuf::from("qaly-mcp"));
    if let Some(ref config_path) = detected.claude_code_config {
        let pb = mk("Registering with Claude Code...");
        match register_mcp_agent(&AgentConfig {
            config_path: config_path.clone(),
            mcp_binary: mcp_binary.clone(),
            adb_binary: adb_path.clone(),
        }) {
            Ok(_) => finish_ok(&pb, "Claude Code  → registered qaly-mcp".into()),
            Err(e) => finish_warn(&pb, format!("Claude Code  → skipped ({e})")),
        }
    }
    if let Some(ref config_path) = detected.cursor_config {
        let pb = mk("Registering with Cursor...");
        match register_mcp_agent(&AgentConfig {
            config_path: config_path.clone(),
            mcp_binary: mcp_binary.clone(),
            adb_binary: adb_path.clone(),
        }) {
            Ok(_) => finish_ok(&pb, "Cursor  → registered qaly-mcp".into()),
            Err(e) => finish_warn(&pb, format!("Cursor  → skipped ({e})")),
        }
    }
    {
        let pb = mk("Writing MCP config snippet...");
        let cwd = std::env::current_dir().unwrap_or_else(|_| home.clone());
        match write_generic_snippet(&cwd, &mcp_binary, &adb_path) {
            Ok(p) => finish_ok(&pb, format!("MCP snippet  → {}", p.display())),
            Err(e) => finish_warn(&pb, format!("MCP snippet  → skipped ({e})")),
        }
    }

    // ── 6. Sample test (optional) ─────────────────────────────────────────────
    {
        let pb = mk("Creating sample test...");
        let cwd = std::env::current_dir().unwrap_or_else(|_| home.clone());
        match write_sample_test(&cwd) {
            Ok(p) => finish_ok(&pb, format!("Sample test  → {}", p.file_name().unwrap().to_string_lossy())),
            Err(e) => finish_warn(&pb, format!("Sample test  → skipped ({e})")),
        }
    }

    // ── 7. Done ───────────────────────────────────────────────────────────────
    println!();
    println!("  \x1b[32;1m✓ All done!\x1b[0m  Next steps:");
    println!("    Run your sample test:  \x1b[34mqaly test qaly-smoke.qaly.test\x1b[0m");
    println!("    Open docs:             \x1b[34mhttps://qaly.dev/docs\x1b[0m");
    println!();

    Ok(())
}

fn install_cmdline_tools(sdk_root: &std::path::Path) -> Result<()> {
    use std::process::Command;

    let arch = setup_helpers::host_arch();
    let os = if cfg!(target_os = "macos") { "mac" } else { "linux" };
    // cmdline-tools version — pinned for reproducibility.
    let url = format!(
        "https://dl.google.com/android/repository/commandlinetools-{os}-11076708_latest.zip"
    );
    let _ = arch; // arch is used for system-image install below

    let tmp = tempfile::tempdir().context("tmpdir")?;
    let zip_path = tmp.path().join("cmdline-tools.zip");

    // Download
    let status = Command::new("curl")
        .args(["-fsSL", "-o", zip_path.to_str().unwrap(), &url])
        .status()
        .context("curl failed")?;
    anyhow::ensure!(status.success(), "curl download failed");

    // Unzip into a temp dir
    let unzip_dir = tmp.path().join("unzipped");
    std::fs::create_dir_all(&unzip_dir)?;
    let status = Command::new("unzip")
        .args(["-q", zip_path.to_str().unwrap(), "-d", unzip_dir.to_str().unwrap()])
        .status()
        .context("unzip failed")?;
    anyhow::ensure!(status.success(), "unzip failed");

    // Move cmdline-tools → sdk_root/cmdline-tools/latest
    let dest = sdk_root.join("cmdline-tools/latest");
    std::fs::create_dir_all(&dest)?;
    // The zip contains a `cmdline-tools/` folder at root.
    let src = unzip_dir.join("cmdline-tools");
    for entry in std::fs::read_dir(&src).context("read unzipped dir")? {
        let entry = entry?;
        let to = dest.join(entry.file_name());
        if to.exists() { std::fs::remove_dir_all(&to).ok(); }
        std::fs::rename(entry.path(), to)?;
    }

    // Install required SDK packages
    let sdkmanager = setup_helpers::sdk_manager_path(sdk_root);
    let arch_image = format!("system-images;android-36;google_apis;{arch}");
    let status = Command::new(&sdkmanager)
        .args([
            "--install",
            "platform-tools",
            "emulator",
            &arch_image,
        ])
        .env("JAVA_OPTS", "-Dfile.encoding=UTF-8")
        .stdin(std::process::Stdio::null()) // auto-accept licenses piped separately
        .status()
        .context("sdkmanager install failed")?;
    // sdkmanager may exit non-zero if license not accepted; accept and retry.
    if !status.success() {
        // Accept licenses
        let mut child = Command::new(&sdkmanager)
            .arg("--licenses")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("sdkmanager --licenses failed")?;
        if let Some(stdin) = child.stdin.take() {
            use std::io::Write;
            let mut stdin = stdin;
            // Send 'y' for each license prompt (typically ≤10).
            for _ in 0..10 { let _ = stdin.write_all(b"y\n"); }
        }
        child.wait().context("wait sdkmanager --licenses")?;

        // Retry install
        let retry_status = Command::new(&sdkmanager)
            .args(["--install", "platform-tools", "emulator", &arch_image])
            .status()
            .context("sdkmanager retry failed")?;
        anyhow::ensure!(retry_status.success(), "sdkmanager install failed after license acceptance");
    }

    Ok(())
}


fn main() -> Result<()> {
    qaly_core::Config::load().apply_env();
    let cli = Cli::parse();
    let mut session = Session::new(Box::new(AdbDevice::new()));
    if let Ok(dir) = std::env::var("QALY_RUN_DIR") {
        if let Err(e) = session.enable_logging(std::path::Path::new(&dir)) {
            eprintln!("qaly: run logging disabled: {e}");
        }
    }

    match cli.command {
        Command::Devices => {
            let dev = AdbDevice::new();
            for s in qaly_core::DeviceController::list_devices(&dev)? {
                println!("{s}");
            }
        }
        Command::Launch { package } => session.launch(&package)?,
        Command::Perceive { json, out } => {
            let (screen, annotated) = session.perceive()?;
            if json || out.is_none() {
                println!("{}", serde_json::to_string_pretty(&screen)?);
            }
            if let Some(path) = out {
                fs::write(&path, &annotated).with_context(|| format!("write {path}"))?;
                eprintln!("annotated screenshot -> {path}");
            }
        }
        // The CLI is stateless per-invocation, so perceive() first to populate
        // the screen that tap/fill resolve their target against.
        Command::Tap { target } => {
            session.perceive()?;
            session.tap(&target)?;
        }
        Command::Type { text } => session.type_text(&text)?,
        Command::Fill { target, text } => {
            session.perceive()?;
            session.fill(&target, &text)?;
        }
        Command::Swipe { direction } => {
            let (x1, y1, x2, y2) = swipe_vector(&direction)?;
            session.swipe(x1, y1, x2, y2, 300)?;
        }
        Command::Key { name } => {
            let code = keycode(&name).ok_or_else(|| anyhow::anyhow!("unknown key '{name}'"))?;
            session.key(code)?;
        }
        Command::WaitFor { label, timeout_ms } => session.wait_for(&label, timeout_ms)?,
        Command::AssertVisible { label } => session.assert_visible(&label)?,
        Command::Shell { command } => {
            let out = session.shell(&command)?;
            print!("{out}");
        }
        Command::Test { file, filter, workers, headless, avd, strict_labels, auto_heal, debug, debug_dir } => {
            use qaly_core::testing::{parse_test_file, recording_path, Recording, TestResult};
            let test_path = std::path::Path::new(&file);
            let debug_output_dir: Option<std::path::PathBuf> = debug_dir
                .as_deref()
                .map(std::path::PathBuf::from)
                .or_else(|| if debug { Some(qaly_core::testing::default_debug_dir(test_path)) } else { None });

            let replay_opts = qaly_core::testing::ReplayOpts {
                auto_heal,
                heal_test_file: if auto_heal { Some(test_path.to_path_buf()) } else { None },
                heal_goal: None,  // overridden per-test inside the runner
                debug_artifacts: debug_output_dir.clone(),
            };
            let mut tf = parse_test_file(test_path)
                .with_context(|| format!("failed to parse '{file}'"))?;
            if strict_labels {
                tf.duplicate_policy = qaly_core::actions::DuplicatePolicy::Error;
            }
            qaly_core::testing::cleanup_orphaned_recordings(test_path, &tf.goals())
                .with_context(|| "failed to clean up orphaned recordings")?;

            let n_workers = workers.unwrap_or(1);
            let results: Vec<TestResult> = if n_workers > 1 {
                use qaly_core::testing::{run_tests_parallel, run_preloaded_tests_parallel};
                use qaly_core::emulator::{EmulatorConfig, start_pool, stop_pool};

                let avd_name = avd.unwrap_or_else(|| {
                    std::env::var("AVD_NAME").unwrap_or_else(|_| "Medium_Phone_API_36.1".into())
                });

                let par_results = if tf.fixtures.is_empty() {
                    // ── No fixtures: classic parallel with base snapshot ──────────────
                    if matches!(tf.clean_state, qaly_core::testing::CleanStateConfig::AppData) {
                        eprintln!("qaly: warning: clean_state 'app_data' is not supported with \
                                   --workers > 1 in no-fixture parallel mode (base snapshot will be \
                                   restored instead). Remove --workers or use a fixture-based layout.");
                    }
                    let config = EmulatorConfig {
                        avd_name: avd_name.clone(),
                        pool_size: n_workers,
                        headless,
                        ..EmulatorConfig::default()
                    };
                    let (pool, managed) = start_pool(&config)
                        .with_context(|| "failed to start emulator pool")?;

                    let base_snapshot = format!("qaly_base_{}", Recording::now_unix());
                    let pool_size = pool.size();
                    let mut snap_devices = Vec::new();
                    for _ in 0..pool_size {
                        let device = pool.acquire();
                        device.snapshot_save(&base_snapshot)
                            .with_context(|| "failed to save base snapshot")?;
                        snap_devices.push(device);
                    }
                    for d in snap_devices { pool.release(d); }

                    let mut recorded: Vec<(usize, Recording)> = Vec::new();
                    let mut unrecorded_r: Vec<(usize, TestResult)> = Vec::new();
                    for (i, entry) in tf.entries.iter().enumerate() {
                        let goal = &entry.goal;
                        if let Some(f) = &filter {
                            if !goal.to_lowercase().contains(&f.to_lowercase()) { continue; }
                        }
                        let rec_path = recording_path(test_path, i, goal);
                        if rec_path.exists() {
                            match Recording::load(&rec_path) {
                                Ok(rec) => recorded.push((i, rec)),
                                Err(e) => unrecorded_r.push((i, TestResult::load_error(goal, &e))),
                            }
                        } else {
                            unrecorded_r.push((i, TestResult::unrecorded(goal)));
                        }
                    }

                    let refs: Vec<(usize, &Recording)> = recorded.iter().map(|(i, r)| (*i, r)).collect();
                    let mut par = run_tests_parallel(&refs, &pool, &base_snapshot, replay_opts.clone());
                    par.extend(unrecorded_r);
                    stop_pool(managed).with_context(|| "failed to stop emulators")?;
                    par
                } else {
                    // ── Fixtures present: Phase 1 sequential + Phase 2 parallel ─────
                    //
                    // ALL emulators run as -read-only so that multiple instances of the
                    // same AVD can coexist (Android emulator requirement).
                    //
                    // Phase 1 boots ONE managed device from qaly_clean_state (if the
                    // snapshot exists) so tests start from a known state without needing
                    // a runtime snapshot_restore (which -read-only blocks).
                    //
                    // Phase 2 boots N devices directly FROM each fixture snapshot.
                    // Workers are already in the right state the moment they finish
                    // booting — no runtime snapshot_restore required.
                    //
                    // Before starting anything we kill any non-read-only emulator that
                    // is already running; if left alive it prevents -read-only instances
                    // from starting ("Another emulator instance is running" error).

                    // ── Prepare: detect clean_state, kill conflicting emulators ───────
                    let adb_bin = std::env::var("ADB_BINARY")
                        .unwrap_or_else(|_| "adb".into());

                    // Check on disk — no running emulator required.
                    let clean_snap_exists = qaly_core::snapshot_exists_on_disk(
                        &avd_name, qaly_core::CLEAN_STATE_SNAPSHOT,
                    );

                    if !clean_snap_exists {
                        eprintln!(
                            "qaly: warning — '{}' snapshot not found; Phase 1 \
                             will cold-boot (tests may start from wrong state). \
                             Run tests once sequentially to create it.",
                            qaly_core::CLEAN_STATE_SNAPSHOT
                        );
                    }

                    // Kill any running emulator — it may be non-read-only, which blocks
                    // the -read-only managed instances we are about to start.
                    qaly_core::kill_all_running_emulators(&adb_bin);

                    // ── Phase 1: one device, sequential ──────────────────────────────
                    let p1_config = EmulatorConfig {
                        avd_name: avd_name.clone(),
                        pool_size: 1,
                        headless,
                        boot_snapshot: if clean_snap_exists {
                            Some(qaly_core::CLEAN_STATE_SNAPSHOT.into())
                        } else {
                            None
                        },
                        ..EmulatorConfig::default()
                    };
                    let (p1_pool, p1_managed) = start_pool(&p1_config)
                        .with_context(|| "failed to start Phase 1 emulator")?;

                    let p1_device = p1_pool.acquire();
                    let mut p1_session = qaly_core::Session::new(p1_device);
                    let mut p1_items: Vec<(usize, Option<Recording>, &qaly_core::testing::TestEntry)> = Vec::new();
                    let mut p1_errors: std::collections::HashMap<usize, TestResult> = std::collections::HashMap::new();

                    for (i, entry) in tf.entries.iter().enumerate() {
                        if entry.fixture.is_some() { continue; }
                        let goal = &entry.goal;
                        if let Some(f) = &filter {
                            if !goal.to_lowercase().contains(&f.to_lowercase()) { continue; }
                        }
                        let rec_path = recording_path(test_path, i, goal);
                        if rec_path.exists() {
                            match Recording::load(&rec_path) {
                                Ok(rec) => p1_items.push((i, Some(rec), entry)),
                                Err(e) => { p1_errors.insert(i, TestResult::load_error(goal, &e)); }
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
                    stop_pool(p1_managed).with_context(|| "failed to stop Phase 1 emulator")?;

                    // ── Phase 2: N workers, one per fixture group ─────────────────────
                    // Group fixture tests by fixture name so each group boots from the
                    // correct snapshot. Within a group, all tests run in parallel.

                    // Collect fixture groups: fixture_name → [(idx, Recording)]
                    // BTreeMap gives deterministic group-processing order across runs.
                    let mut fixture_groups: std::collections::BTreeMap<
                        String, Vec<(usize, Recording)>
                    > = std::collections::BTreeMap::new();
                    let mut p2_errors: Vec<(usize, TestResult)> = Vec::new();

                    for (i, entry) in tf.entries.iter().enumerate() {
                        let Some(fixture_name) = &entry.fixture else { continue };
                        let goal = &entry.goal;
                        if let Some(f) = &filter {
                            if !goal.to_lowercase().contains(&f.to_lowercase()) { continue; }
                        }
                        let rec_path = recording_path(test_path, i, goal);
                        if rec_path.exists() {
                            match Recording::load(&rec_path) {
                                Ok(rec) => {
                                    fixture_groups
                                        .entry(fixture_name.clone())
                                        .or_default()
                                        .push((i, rec));
                                }
                                Err(e) => p2_errors.push((i, TestResult::load_error(goal, &e))),
                            }
                        } else {
                            p2_errors.push((i, TestResult::unrecorded(goal)));
                        }
                    }

                    let mut p2_pairs: Vec<(usize, TestResult)> = p2_errors;

                    for (fixture_name, group) in &fixture_groups {
                        let snap_name = format!("qaly_fixture_{fixture_name}");
                        let workers_for_group = group.len().min(n_workers);

                        // Boot N workers directly from the fixture snapshot.
                        let p2_config = EmulatorConfig {
                            avd_name: avd_name.clone(),
                            pool_size: workers_for_group,
                            headless,
                            boot_snapshot: Some(snap_name.clone()),
                            ..EmulatorConfig::default()
                        };
                        let (p2_pool, p2_managed) = start_pool(&p2_config)
                            .with_context(|| format!("failed to start Phase 2 workers for fixture '{fixture_name}'"))?;

                        let refs: Vec<(usize, &Recording)> =
                            group.iter().map(|(i, r)| (*i, r)).collect();
                        let mut group_results = run_preloaded_tests_parallel(&refs, &p2_pool, replay_opts.clone());
                        p2_pairs.append(&mut group_results);

                        stop_pool(p2_managed)
                            .with_context(|| format!("failed to stop Phase 2 workers for '{fixture_name}'"))?;
                    }

                    let mut all: std::collections::HashMap<usize, TestResult> = p1_errors;
                    for (idx, r) in p1_pairs { all.insert(idx, r); }
                    for (idx, r) in p2_pairs { all.insert(idx, r); }
                    let mut sorted: Vec<(usize, TestResult)> = all.into_iter().collect();
                    sorted.sort_by_key(|(i, _)| *i);
                    sorted
                };

                par_results.into_iter().map(|(_, r)| r).collect()
            } else {
                // Sequential: use run_entries_sequential for fixture-aware replay
                let mut items: Vec<(usize, Option<Recording>, &qaly_core::testing::TestEntry)> = Vec::new();
                let mut error_map: std::collections::HashMap<usize, TestResult> = std::collections::HashMap::new();

                for (i, entry) in tf.entries.iter().enumerate() {
                    let goal = &entry.goal;
                    if let Some(f) = &filter {
                        if !goal.to_lowercase().contains(&f.to_lowercase()) { continue; }
                    }
                    let rec_path = recording_path(test_path, i, goal);
                    if rec_path.exists() {
                        match Recording::load(&rec_path) {
                            Ok(rec) => items.push((i, Some(rec), entry)),
                            Err(e) => { error_map.insert(i, TestResult::load_error(goal, &e)); }
                        }
                    } else {
                        items.push((i, None, entry));  // will be marked Unrecorded
                    }
                }

                let pairs = qaly_core::testing::run_entries_sequential(&items, &tf, &mut session, replay_opts.clone());
                let mut all: std::collections::HashMap<usize, TestResult> = error_map;
                for (idx, r) in pairs { all.insert(idx, r); }
                let mut sorted: Vec<(usize, TestResult)> = all.into_iter().collect();
                sorted.sort_by_key(|(i, _)| *i);
                sorted.into_iter().map(|(_, r)| r).collect()
            };

            println!("{}", qaly_core::testing::format_report(&results));
            if results.iter().any(|r| r.status == qaly_core::testing::TestStatus::Failed) {
                std::process::exit(1);
            }
        }
        Command::Init => run_init(false)?,
        Command::Doctor => run_init(true)?,
        Command::Setup => run_init(false)?,
        Command::SetupCheck => run_init(true)?,
        Command::Migrate => {
            let cwd = std::env::current_dir()?;
            let mut renamed = 0usize;
            for entry in walkdir::WalkDir::new(&cwd)
                .into_iter()
                .filter_entry(|e| {
                    // Don't recurse inside a .sim we're about to rename
                    e.depth() == 0 || e.file_name() != ".sim" || !e.file_type().is_dir()
                })
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir() && e.file_name() == ".sim")
            {
                let old = entry.path();
                let new = old.parent().unwrap().join(".qaly");
                if new.exists() {
                    eprintln!("skip: {} (target already exists)", old.display());
                    continue;
                }
                std::fs::rename(old, &new)
                    .with_context(|| format!("failed to rename {} to {}", old.display(), new.display()))?;
                println!("renamed: {} → {}", old.display(), new.display());
                renamed += 1;
            }
            println!("Done. {} director{} renamed.", renamed, if renamed == 1 { "y" } else { "ies" });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_tap_subcommand() {
        let cli = Cli::try_parse_from(["qaly", "tap", "e3"]).unwrap();
        assert!(matches!(cli.command, Command::Tap { target } if target == "e3"));
    }

    #[test]
    fn parses_fill_subcommand() {
        let cli = Cli::try_parse_from(["qaly", "fill", "e2", "07"]).unwrap();
        assert!(matches!(cli.command, Command::Fill { target, text } if target == "e2" && text == "07"));
    }

    #[test]
    fn parses_shell_subcommand() {
        let cli = Cli::try_parse_from(["qaly", "shell", "dumpsys battery"]).unwrap();
        assert!(matches!(cli.command, Command::Shell { command } if command == "dumpsys battery"));
    }

    #[test]
    fn parses_test_subcommand() {
        let cli = Cli::try_parse_from(["qaly", "test", "tests/foo.qaly.test"]).unwrap();
        assert!(matches!(cli.command, Command::Test { file, .. } if file == "tests/foo.qaly.test"));
    }

    #[test]
    fn parses_test_with_workers_flag() {
        let cli = Cli::try_parse_from(["qaly", "test", "tests/foo.qaly.test", "--workers", "3"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Test { workers: Some(3), .. }
        ));
    }

    #[test]
    fn parses_test_with_headless_flag() {
        let cli = Cli::try_parse_from(["qaly", "test", "tests/foo.qaly.test", "--headless"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Test { headless: true, .. }
        ));
    }

    #[test]
    fn parses_test_with_strict_labels_flag() {
        let cli = Cli::try_parse_from(["qaly", "test", "tests/foo.qaly.test", "--strict-labels"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Test { strict_labels: true, .. }
        ));
    }

    #[test]
    fn parses_test_with_auto_heal_flag() {
        let cli = Cli::try_parse_from(["qaly", "test", "tests/foo.qaly.test", "--auto-heal"]).unwrap();
        assert!(matches!(cli.command, Command::Test { auto_heal: true, .. }));
    }

    #[test]
    fn parses_test_with_debug_flag() {
        let cli = Cli::try_parse_from(["qaly", "test", "tests/foo.qaly.test", "--debug"]).unwrap();
        assert!(matches!(cli.command, Command::Test { debug: true, .. }));
    }

    #[test]
    fn parses_test_with_debug_dir_flag() {
        let cli = Cli::try_parse_from([
            "qaly", "test", "tests/foo.qaly.test", "--debug-dir", "/tmp/out",
        ]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Test { debug_dir: Some(ref d), .. } if d == "/tmp/out"
        ));
    }

    mod setup_tests {
        use super::setup_helpers::*;

        #[test]
        fn parse_avd_list_extracts_names() {
            let output = "Available Android Virtual Devices:\n    Name: sim-default\n    Name: Pixel_7\n";
            let names = parse_avd_names(output);
            assert_eq!(names, vec!["sim-default", "Pixel_7"]);
        }

        #[test]
        fn parse_avd_list_empty() {
            let names = parse_avd_names("There are no Android Virtual Devices installed.\n");
            assert!(names.is_empty());
        }

        #[test]
        fn detect_arch_returns_known_value() {
            let arch = host_arch();
            assert!(arch == "arm64-v8a" || arch == "x86_64", "unexpected arch: {arch}");
        }
    }
}
