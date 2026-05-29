//! Setup wizard logic for `qaly init` / `qaly doctor`.
//!
//! Inlined from the former core `setup` + `Config` modules. These are pure
//! filesystem/process operations and do not require the daemon, so they live in
//! the CLI directly rather than over gRPC.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

// ── Config ─────────────────────────────────────────────────────────────────

/// Minimal config writer used by the setup wizard. The daemon owns runtime
/// config; the CLI only needs to persist the detected adb binary and AVD name.
#[derive(Debug, Default, serde::Serialize)]
pub struct Config {
    pub emulator: EmulatorCfg,
    pub sdk: SdkCfg,
}

#[derive(Debug, serde::Serialize)]
pub struct EmulatorCfg {
    pub auto_start: bool,
    pub headless: bool,
    pub avd: String,
}

impl Default for EmulatorCfg {
    fn default() -> Self {
        EmulatorCfg {
            auto_start: true,
            headless: true,
            avd: "sim-default".to_string(),
        }
    }
}

#[derive(Debug, Default, serde::Serialize)]
pub struct SdkCfg {
    pub adb_binary: Option<PathBuf>,
    pub android_sdk_root: Option<PathBuf>,
}

impl Config {
    /// Platform config dir: ~/.config/qaly/config.toml (Linux/macOS).
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("qaly")
            .join("config.toml")
    }

    /// Persist to ~/.config/qaly/config.toml, creating parent dirs as needed.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = toml::to_string_pretty(self).expect("config serialize");
        std::fs::write(path, s)
    }
}

// ── Detection ────────────────────────────────────────────────────────────────

/// Results of the environment detection phase.
#[derive(Debug, Default)]
pub struct DetectResult {
    /// Path to a working `adb` binary, if found.
    pub adb_path: Option<PathBuf>,
    /// Path to the `emulator` binary, if found. Retained for completeness; the
    /// wizard keys off `available_avds` rather than the binary path directly.
    #[allow(dead_code)]
    pub emulator_path: Option<PathBuf>,
    /// AVD names available on this machine (requires emulator binary).
    pub available_avds: Vec<String>,
    /// Emulator serial numbers currently connected (e.g. "emulator-5554").
    pub running_emulator_serials: Vec<String>,
    /// Path to the `qaly-mcp` binary, if found.
    pub mcp_binary: Option<PathBuf>,
    /// Path to `~/.claude.json` if Claude Code is configured here.
    pub claude_code_config: Option<PathBuf>,
    /// Path to `~/.cursor/mcp.json` if Cursor is configured here.
    pub cursor_config: Option<PathBuf>,
}

/// Run all detection steps against the given home directory.
pub fn detect(home: &Path) -> DetectResult {
    let adb_path = find_adb(home);
    let emulator_path = find_emulator(home);
    let running_emulator_serials = adb_path
        .as_deref()
        .map(list_running_emulators)
        .unwrap_or_default();
    let available_avds = emulator_path
        .as_deref()
        .map(list_avds)
        .unwrap_or_default();
    let mcp_binary = find_mcp_binary();
    let claude_code_config = find_agent_config(home, ".claude.json");
    let cursor_config = find_agent_config(home, ".cursor/mcp.json");
    DetectResult {
        adb_path,
        emulator_path,
        available_avds,
        running_emulator_serials,
        mcp_binary,
        claude_code_config,
        cursor_config,
    }
}

fn find_adb(home: &Path) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ADB_BINARY") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(sdk) = std::env::var("ANDROID_SDK_ROOT") {
        let p = PathBuf::from(sdk).join("platform-tools/adb");
        if p.exists() {
            return Some(p);
        }
    }
    let macos = home.join("Library/Android/sdk/platform-tools/adb");
    if macos.exists() {
        return Some(macos);
    }
    let linux = home.join("android-sdk/platform-tools/adb");
    if linux.exists() {
        return Some(linux);
    }
    which_bin("adb")
}

fn find_emulator(home: &Path) -> Option<PathBuf> {
    if let Ok(sdk) = std::env::var("ANDROID_SDK_ROOT") {
        let p = PathBuf::from(sdk).join("emulator/emulator");
        if p.exists() {
            return Some(p);
        }
    }
    let macos = home.join("Library/Android/sdk/emulator/emulator");
    if macos.exists() {
        return Some(macos);
    }
    let linux = home.join("android-sdk/emulator/emulator");
    if linux.exists() {
        return Some(linux);
    }
    which_bin("emulator")
}

fn list_running_emulators(adb: &Path) -> Vec<String> {
    let out = std::process::Command::new(adb).args(["devices"]).output().ok();
    let Some(out) = out else {
        return vec![];
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.contains("emulator-") && l.contains("device"))
        .filter_map(|l| l.split_whitespace().next().map(String::from))
        .collect()
}

fn list_avds(emulator: &Path) -> Vec<String> {
    let out = std::process::Command::new(emulator)
        .arg("-list-avds")
        .output()
        .ok();
    let Some(out) = out else {
        return vec![];
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

fn find_mcp_binary() -> Option<PathBuf> {
    for p in ["/usr/local/bin/qaly-mcp", "/opt/homebrew/bin/qaly-mcp"] {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    which_bin("qaly-mcp")
}

fn find_agent_config(home: &Path, rel: &str) -> Option<PathBuf> {
    let p = home.join(rel);
    if p.exists() {
        Some(p)
    } else {
        None
    }
}

fn which_bin(name: &str) -> Option<PathBuf> {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))
        .filter(|p| !p.as_os_str().is_empty())
}

// ── Agent registration ───────────────────────────────────────────────────────

pub struct AgentConfig {
    /// Path to the agent's MCP JSON config file (e.g. `~/.claude.json`).
    pub config_path: PathBuf,
    pub mcp_binary: PathBuf,
    pub adb_binary: PathBuf,
}

/// Patch the agent's MCP JSON config to include `qaly-mcp`. Creates the file if
/// it doesn't exist. Leaves all other `mcpServers` entries untouched.
pub fn register_mcp_agent(cfg: &AgentConfig) -> Result<()> {
    let existing = std::fs::read_to_string(&cfg.config_path).unwrap_or_else(|_| "{}".into());
    let mut v: serde_json::Value =
        serde_json::from_str(&existing).context("parse json")?;
    let entry = qaly_entry(&cfg.mcp_binary, &cfg.adb_binary);
    v["mcpServers"]["qaly"] = entry;
    if let Some(parent) = cfg.config_path.parent() {
        std::fs::create_dir_all(parent).context("mkdir")?;
    }
    let json_str = serde_json::to_string_pretty(&v).context("serialize json")?;
    std::fs::write(&cfg.config_path, json_str).context("write config")?;
    Ok(())
}

/// Write a standalone `qaly-mcp-config.json` snippet to `dir`.
pub fn write_generic_snippet(
    dir: &Path,
    mcp_binary: &Path,
    adb_binary: &Path,
) -> Result<PathBuf> {
    let snippet = serde_json::json!({
        "mcpServers": {
            "qaly": qaly_entry(mcp_binary, adb_binary)
        }
    });
    let path = dir.join("qaly-mcp-config.json");
    let json_str = serde_json::to_string_pretty(&snippet).context("serialize json")?;
    std::fs::write(&path, json_str).context("write snippet")?;
    Ok(path)
}

fn qaly_entry(mcp_binary: &Path, adb_binary: &Path) -> serde_json::Value {
    serde_json::json!({
        "command": mcp_binary.display().to_string(),
        "env": {
            "ADB_BINARY": adb_binary.display().to_string()
        }
    })
}

// ── Sample test ──────────────────────────────────────────────────────────────

const SAMPLE_CONTENT: &str = r#"app: com.android.settings

tests:
  - Verify that the Settings title is visible
"#;

/// Write `qaly-smoke.qaly.test` to `dir`. Returns the path to the created file.
pub fn write_sample_test(dir: &Path) -> Result<PathBuf> {
    let path = dir.join("qaly-smoke.qaly.test");
    std::fs::write(&path, SAMPLE_CONTENT).context("write sample test")?;
    Ok(path)
}
