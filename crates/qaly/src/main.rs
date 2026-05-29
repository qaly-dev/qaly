mod client;
mod setup;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use client::proto;
use client::DaemonClient;

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
    /// Launch an app by package id.
    Launch { package: String },
    /// Capture screen; prints JSON, optionally writes a PNG.
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
        workers: Option<u32>,
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

/// Map a direction name to the proto SwipeDirection enum value.
fn swipe_direction_value(direction: &str) -> Result<i32> {
    match direction {
        "up" => Ok(proto::SwipeDirection::Up as i32),
        "down" => Ok(proto::SwipeDirection::Down as i32),
        "left" => Ok(proto::SwipeDirection::Left as i32),
        "right" => Ok(proto::SwipeDirection::Right as i32),
        other => Err(anyhow::anyhow!(
            "unknown swipe direction '{other}'. Valid: up|down|left|right"
        )),
    }
}

/// Render a proto TestResult into human-readable text.
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

fn run_init(check_only: bool) -> Result<()> {
    use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    use setup::{register_mcp_agent, write_generic_snippet, write_sample_test, AgentConfig};
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
    let mut detected = setup::detect(&home);
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
            detected = setup::detect(&home);
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
        let mut cfg = setup::Config::default();
        cfg.sdk.adb_binary = Some(adb_path.clone());
        if let Some(avd) = detected.available_avds.first() {
            cfg.emulator.avd = avd.clone();
        }
        cfg.save().context("failed to write config")?;
        finish_ok(&pb, format!("Config written   {}", setup::Config::path().display()));
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

fn run_migrate() -> Result<()> {
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
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // ── Setup / filesystem commands (no daemon) ──────────────────────────
        Command::Init => return run_init(false),
        Command::Doctor => return run_init(true),
        Command::Setup => return run_init(false),
        Command::SetupCheck => return run_init(true),
        Command::Migrate => return run_migrate(),
        _ => {}
    }

    // ── Device / test commands (gRPC to qaly-daemon) ─────────────────────────
    match cli.command {
        Command::Launch { package } => {
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .launch_app(proto::LaunchRequest { session_id, package })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            if !resp.message.is_empty() {
                println!("{}", resp.message);
            }
        }
        Command::Perceive { json, out } => {
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .perceive(proto::PerceiveRequest {
                    session_id,
                    // text_only when we only need the JSON tree; request a
                    // screenshot only when --out is given.
                    text_only: out.is_none(),
                })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            if json || out.is_none() {
                println!("{}", serde_json::to_string_pretty(&resp.elements)?);
            }
            if let Some(path) = out {
                let png = resp.screenshot_png.unwrap_or_default();
                std::fs::write(&path, &png).with_context(|| format!("write {path}"))?;
                eprintln!("screenshot -> {path}");
            }
        }
        Command::Tap { target } => {
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .tap(proto::TapRequest { session_id, target })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            if !resp.message.is_empty() {
                println!("{}", resp.message);
            }
        }
        Command::Type { text } => {
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .type_text(proto::TypeTextRequest { session_id, text })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            if !resp.message.is_empty() {
                println!("{}", resp.message);
            }
        }
        Command::Fill { target, text } => {
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .fill(proto::FillRequest { session_id, target, text })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            if !resp.message.is_empty() {
                println!("{}", resp.message);
            }
        }
        Command::Swipe { direction } => {
            let dir = swipe_direction_value(&direction)?;
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .swipe(proto::SwipeRequest { session_id, direction: dir })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            if !resp.message.is_empty() {
                println!("{}", resp.message);
            }
        }
        Command::Key { name } => {
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .press_key(proto::PressKeyRequest { session_id, keycode: name })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            if !resp.message.is_empty() {
                println!("{}", resp.message);
            }
        }
        Command::WaitFor { label, timeout_ms } => {
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .wait_for(proto::WaitForRequest {
                    session_id,
                    target: label,
                    timeout_ms: timeout_ms as u32,
                })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            if !resp.message.is_empty() {
                println!("{}", resp.message);
            }
        }
        Command::AssertVisible { label } => {
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .assert_visible(proto::AssertRequest { session_id, target: label })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            if !resp.success {
                if !resp.message.is_empty() {
                    eprintln!("{}", resp.message);
                }
                std::process::exit(1);
            }
            if !resp.message.is_empty() {
                println!("{}", resp.message);
            }
        }
        Command::Shell { command } => {
            let mut client = DaemonClient::connect().await?;
            let session_id = client.session_id.clone();
            let resp = client
                .device
                .shell(proto::ShellRequest { session_id, command })
                .await?
                .into_inner();
            client.destroy_session().await.ok();
            print!("{}", resp.stdout);
        }
        Command::Test {
            file,
            filter,
            workers,
            headless,
            avd,
            strict_labels,
            auto_heal,
            debug,
            debug_dir,
        } => {
            // --strict-labels and --debug-dir are not yet plumbed through the
            // gRPC RunTests API; the daemon honors the test-file setting and a
            // default debug dir. Warn rather than silently drop the flag.
            if strict_labels {
                eprintln!("qaly: note: --strict-labels is controlled by the test file under the daemon; flag ignored.");
            }
            if debug_dir.is_some() {
                eprintln!("qaly: note: --debug-dir is not supported under the daemon; using the default debug directory.");
            }

            let mut client = DaemonClient::connect().await?;
            let mut stream = client
                .test
                .run_tests(proto::RunTestsRequest {
                    test_file: file,
                    filter,
                    workers: workers.unwrap_or(1),
                    headless,
                    avd_name: avd,
                    auto_heal,
                    debug: debug || debug_dir.is_some(),
                })
                .await?
                .into_inner();

            let mut results: Vec<proto::TestResult> = Vec::new();
            while let Some(event) = stream.message().await? {
                use proto::test_event::Event;
                match event.event {
                    Some(Event::Started(s)) => println!("running  {}", s.test_name),
                    Some(Event::Result(r)) => {
                        println!("{}", format_test_result(&r));
                        results.push(r);
                    }
                    _ => {}
                }
            }
            client.destroy_session().await.ok();

            let mut pass = 0;
            let mut fail = 0;
            let mut skip = 0;
            for r in &results {
                match proto::TestStatus::try_from(r.status) {
                    Ok(proto::TestStatus::Pass) => pass += 1,
                    Ok(proto::TestStatus::Fail) => fail += 1,
                    Ok(proto::TestStatus::Skip) => skip += 1,
                    Err(_) => {}
                }
            }
            println!("\n{pass} passed, {fail} failed, {skip} skipped");
            if fail > 0 {
                std::process::exit(1);
            }
        }
        // Setup commands handled above.
        Command::Init
        | Command::Doctor
        | Command::Setup
        | Command::SetupCheck
        | Command::Migrate => unreachable!(),
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

    #[test]
    fn swipe_direction_maps_known_values() {
        assert_eq!(swipe_direction_value("up").unwrap(), proto::SwipeDirection::Up as i32);
        assert_eq!(swipe_direction_value("down").unwrap(), proto::SwipeDirection::Down as i32);
        assert!(swipe_direction_value("sideways").is_err());
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
