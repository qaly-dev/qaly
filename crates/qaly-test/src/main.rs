use anyhow::Result;
use clap::Parser;
mod client;
use client::{DaemonClient, proto};

#[derive(Parser)]
#[command(name = "qaly-test", about = "Run Qaly test suites from .qaly.test files")]
struct Cli {
    test_file:       String,
    #[arg(long)] filter:       Option<String>,
    #[arg(long)] strict_labels: bool,   // parsed but sent as-is (daemon handles test file settings)
    #[arg(long)] auto_heal:    bool,
    #[arg(long)] debug:        bool,
    #[arg(long)] debug_dir:    Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut client = DaemonClient::connect().await?;

    let mut stream = client.test.run_tests(proto::RunTestsRequest {
        test_file:  cli.test_file.clone(),
        filter:     cli.filter.clone(),
        workers:    1,
        headless:   false,
        avd_name:   None,
        auto_heal:  cli.auto_heal,
        debug:      cli.debug,
    }).await?.into_inner();

    let mut has_failures = false;
    while let Some(event) = stream.message().await? {
        use proto::test_event::Event;
        match event.event {
            Some(Event::Result(r)) => {
                let status = if r.status == proto::TestStatus::Pass as i32 { "PASS" }
                             else if r.status == proto::TestStatus::Skip as i32 { "SKIP" }
                             else { has_failures = true; "FAIL" };
                println!("{status}  {}", r.test_name);
            }
            Some(Event::Started(s)) => println!("running  {}", s.test_name),
            _ => {}
        }
    }

    client.destroy_session().await.ok();
    if has_failures { std::process::exit(1); }
    Ok(())
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
