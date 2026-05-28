use anyhow::{Context, Result};
use clap::Parser;
use qaly_core::{AdbDevice, Session};
use qaly_core::testing::{parse_test_file, recording_path, run_entries_sequential, Recording, TestResult};

#[derive(Parser)]
#[command(name = "qaly-test", about = "Run Qaly test suites from .qaly.test files")]
struct Cli {
    /// Path to the .qaly.test file
    test_file: String,
    /// Only run tests whose goal contains this substring
    #[arg(long)]
    filter: Option<String>,
    /// Fail tests on duplicate actionable labels (overrides test file setting).
    #[arg(long)]
    strict_labels: bool,
    /// Fuzzy-match failing selectors and patch recordings automatically.
    /// Use during development to handle minor UI drift — never in CI.
    #[arg(long)]
    auto_heal: bool,
    /// Capture debug screenshots on failure; write to .qaly/<stem>/last-failure/.
    #[arg(long)]
    debug: bool,
    /// Same as --debug but write artifacts to a custom path.
    #[arg(long)]
    debug_dir: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let test_path = std::path::Path::new(&cli.test_file);
    let debug_output_dir: Option<std::path::PathBuf> = cli.debug_dir
        .as_deref()
        .map(std::path::PathBuf::from)
        .or_else(|| if cli.debug { Some(qaly_core::testing::default_debug_dir(test_path)) } else { None });

    let replay_opts = qaly_core::testing::ReplayOpts {
        auto_heal: cli.auto_heal,
        heal_test_file: if cli.auto_heal { Some(test_path.to_path_buf()) } else { None },
        heal_goal: None,
        debug_artifacts: debug_output_dir.clone(),
    };
    let mut tf = parse_test_file(test_path)
        .with_context(|| format!("failed to parse '{}'", cli.test_file))?;
    if cli.strict_labels {
        tf.duplicate_policy = qaly_core::actions::DuplicatePolicy::Error;
    }
    qaly_core::testing::cleanup_orphaned_recordings(test_path, &tf.goals())
        .with_context(|| "failed to clean up orphaned recordings")?;

    let goals: Vec<String> = tf.goals().into_iter()
        .filter(|g| cli.filter.as_deref().is_none_or(|f| g.to_lowercase().contains(&f.to_lowercase())))
        .collect();

    if goals.is_empty() {
        eprintln!("No tests matched.");
        return Ok(());
    }

    let mut session = Session::new(Box::new(AdbDevice::new()));

    // Build items for run_entries_sequential
    let mut items: Vec<(usize, Option<Recording>, &qaly_core::testing::TestEntry)> = Vec::new();
    let mut error_map: std::collections::HashMap<usize, TestResult> = std::collections::HashMap::new();

    for (i, entry) in tf.entries.iter().enumerate() {
        let goal = &entry.goal;
        if !goals.contains(goal) { continue; }
        let rec_path = recording_path(test_path, i, goal);
        if rec_path.exists() {
            match Recording::load(&rec_path) {
                Ok(rec) => items.push((i, Some(rec), entry)),
                Err(e) => { error_map.insert(i, TestResult::load_error(goal, &e)); }
            }
        } else {
            items.push((i, None, entry));  // unrecorded
        }
    }

    let pairs = run_entries_sequential(&items, &tf, &mut session, replay_opts);
    let mut all: std::collections::HashMap<usize, TestResult> = error_map;
    for (idx, r) in pairs {
        all.insert(idx, r);
    }
    let mut sorted: Vec<(usize, TestResult)> = all.into_iter().collect();
    sorted.sort_by_key(|(i, _)| *i);
    let results: Vec<TestResult> = sorted.into_iter().map(|(_, r)| r).collect();

    println!("{}", qaly_core::testing::format_report(&results));
    let has_failures = results.iter().any(|r| r.status == qaly_core::testing::TestStatus::Failed);
    std::process::exit(if has_failures { 1 } else { 0 });
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_debug_flag() {
        let cli = Cli::try_parse_from(["qaly-test", "tests/foo.qaly.test", "--debug"]).unwrap();
        assert!(cli.debug);
        assert!(cli.debug_dir.is_none());
    }

    #[test]
    fn parses_debug_dir_flag() {
        let cli = Cli::try_parse_from(["qaly-test", "tests/foo.qaly.test", "--debug-dir", "/tmp/out"]).unwrap();
        assert_eq!(cli.debug_dir.as_deref(), Some("/tmp/out"));
    }
}
