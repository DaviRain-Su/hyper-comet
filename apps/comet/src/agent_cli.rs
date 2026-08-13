//! `comet agent` — ProofShip UserExecutor surface (the Raft-style daemon).
//!
//! The web app never runs a cloud agent. This CLI reports the local executor
//! that stays attached to the Cloudflare relay (`desktop-{deviceId}`).

use comet_engine::{EngineConfig, InstanceLock, resolve_relay_base, resolve_relay_identity};
use comet_proto::StudioRelayStatus;
use comet_rpc::methods;

/// Print whether the engine is up and which web session this machine owns.
pub async fn status(config: EngineConfig) -> anyhow::Result<()> {
    let report = collect(&config).await;
    println!("Engine:   {}", report.engine);
    println!(
        "Relay:    {}",
        if report.enabled {
            report.base.as_deref().unwrap_or("on")
        } else {
            "off (PROOFSHIP_RELAY=off)"
        }
    );
    if report.enabled {
        println!(
            "Online:   {}",
            if report.connected {
                "yes — web can send prompts"
            } else if report.engine_running {
                "connecting…"
            } else {
                "no — start `comet daemon` or `comet agent start`"
            }
        );
        println!("Device:   {}", report.device_id);
        println!("Session:  {}", report.session_id);
        if let Some(url) = &report.web_url {
            println!("Web:      {url}");
        }
    }
    Ok(())
}

/// Print only the web Sessions URL (for scripts / QR).
pub async fn url(config: EngineConfig) -> anyhow::Result<()> {
    let report = collect(&config).await;
    match report.web_url {
        Some(url) => {
            println!("{url}");
            Ok(())
        }
        None => anyhow::bail!("web relay is off (PROOFSHIP_RELAY=off)"),
    }
}

struct AgentReport {
    engine: String,
    engine_running: bool,
    enabled: bool,
    connected: bool,
    base: Option<String>,
    device_id: String,
    session_id: String,
    web_url: Option<String>,
}

async fn collect(config: &EngineConfig) -> AgentReport {
    let engine_running = InstanceLock::holder(&config.data_dir).is_some();
    let engine = match InstanceLock::holder(&config.data_dir) {
        Some(pid) => format!("running (pid {pid})"),
        None => "not running".into(),
    };

    if let Some(live) = query_live(config.ipc_port).await {
        return AgentReport {
            engine,
            engine_running,
            enabled: live.enabled,
            connected: live.connected,
            base: live.base,
            device_id: live.device_id,
            session_id: live.session_id,
            web_url: live.web_url,
        };
    }

    let device_id = read_device_id(&config.data_dir).unwrap_or_else(|| "desktop".into());
    match resolve_relay_base(std::env::var("PROOFSHIP_RELAY").ok().as_deref()) {
        Some(base) => {
            let id = resolve_relay_identity(
                &base,
                &device_id,
                std::env::var("PROOFSHIP_DEVICE_ID").ok().as_deref(),
                std::env::var("PROOFSHIP_SESSION_ID")
                    .ok()
                    .or_else(|| std::env::var("PROOFSHIP_LAUNCH_ID").ok())
                    .as_deref(),
            );
            let web_url = id.web_url();
            AgentReport {
                engine,
                engine_running,
                enabled: true,
                connected: false,
                base: Some(id.base),
                device_id: id.device_id,
                session_id: id.session_id,
                web_url: Some(web_url),
            }
        }
        None => AgentReport {
            engine,
            engine_running,
            enabled: false,
            connected: false,
            base: None,
            device_id,
            session_id: String::new(),
            web_url: None,
        },
    }
}

async fn query_live(ipc_port: u16) -> Option<StudioRelayStatus> {
    let client = comet_rpc::connect_ws(&format!("ws://127.0.0.1:{ipc_port}"))
        .await
        .ok()?;
    let value = client
        .call(methods::STUDIO_RELAY_STATUS, serde_json::json!({}))
        .await
        .ok()?;
    serde_json::from_value(value).ok()
}

fn read_device_id(data_dir: &std::path::Path) -> Option<String> {
    let raw = std::fs::read_to_string(data_dir.join("device-id")).ok()?;
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
