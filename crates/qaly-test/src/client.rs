use anyhow::{anyhow, Result};
use std::time::Duration;
use tokio::net::TcpStream;

pub mod proto {
    tonic::include_proto!("qaly.v1");
}

use proto::{
    device_service_client::DeviceServiceClient,
    session_service_client::SessionServiceClient,
    snapshot_service_client::SnapshotServiceClient,
    test_service_client::TestServiceClient, CreateSessionRequest,
};
use tonic::transport::Channel;

const DAEMON_PORT: u16 = 50052;

pub struct DaemonClient {
    pub session_id: String,
    pub device: DeviceServiceClient<Channel>,
    pub test: TestServiceClient<Channel>,
    // The CLI has no snapshot subcommands today, but the daemon exposes the
    // service; retained so the client surface matches qaly-mcp.
    #[allow(dead_code)]
    pub snapshot: SnapshotServiceClient<Channel>,
    // Retained for explicit session teardown via `destroy_session`. The MCP
    // server is long-lived and lets the daemon reap sessions on exit, so this
    // is part of the API surface rather than used on the happy path.
    #[allow(dead_code)]
    session_svc: SessionServiceClient<Channel>,
}

impl DaemonClient {
    pub async fn connect() -> Result<Self> {
        ensure_daemon_running().await?;
        let url = format!("http://127.0.0.1:{DAEMON_PORT}");
        let channel = Channel::from_shared(url)?.connect().await?;

        let mut session_svc = SessionServiceClient::new(channel.clone());
        let info = session_svc
            .create_session(CreateSessionRequest { device_serial: None })
            .await?
            .into_inner();

        Ok(Self {
            session_id: info.id,
            device: DeviceServiceClient::new(channel.clone()),
            test: TestServiceClient::new(channel.clone()),
            snapshot: SnapshotServiceClient::new(channel.clone()),
            session_svc,
        })
    }

    #[allow(dead_code)]
    pub async fn destroy_session(&mut self) -> Result<()> {
        use proto::SessionId;
        self.session_svc
            .destroy_session(SessionId {
                id: self.session_id.clone(),
            })
            .await?;
        Ok(())
    }
}

async fn ensure_daemon_running() -> Result<()> {
    if TcpStream::connect(format!("127.0.0.1:{DAEMON_PORT}"))
        .await
        .is_ok()
    {
        return Ok(());
    }
    let daemon_bin = which_daemon()?;
    tokio::process::Command::new(&daemon_bin)
        .arg("--background")
        .spawn()
        .map_err(|e| anyhow!("failed to start qaly-daemon: {e}"))?;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if TcpStream::connect(format!("127.0.0.1:{DAEMON_PORT}"))
            .await
            .is_ok()
        {
            return Ok(());
        }
    }
    Err(anyhow!(
        "qaly-daemon did not start within 5s — install via: brew install qaly-dev/tap/qaly"
    ))
}

fn which_daemon() -> Result<std::path::PathBuf> {
    if let Ok(path) = which::which("qaly-daemon") {
        return Ok(path);
    }
    if let Ok(mut p) = std::env::current_exe() {
        p.pop();
        p.push("qaly-daemon");
        if p.exists() {
            return Ok(p);
        }
    }
    Err(anyhow!(
        "qaly-daemon not found — install via: brew install qaly-dev/tap/qaly"
    ))
}
