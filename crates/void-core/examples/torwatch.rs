use std::os::windows::process::CommandExt;
use std::time::Duration;

use void_core::{Engine, EngineConfig, TorStatus};

async fn wait_online(engine: &Engine, what: &str) -> anyhow::Result<()> {
    let mut rx = engine.subscribe();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    loop {
        match engine.status() {
            TorStatus::Online { .. } => return Ok(()),
            TorStatus::Failed { error } => anyhow::bail!("{what} échec: {error}"),
            _ => {}
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("{what} timeout");
        }
        if rx.changed().await.is_err() {
            anyhow::bail!("{what} watch fermé");
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let tor_dir = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src-tauri/resources/tor"
    ));
    let data_dir = std::env::temp_dir().join("void-torwatch");
    let _ = std::fs::remove_dir_all(&data_dir);

    let engine = Engine::start(EngineConfig { data_dir, tor_dir }).await?;
    wait_online(&engine, "premier démarrage").await?;
    println!("ONLINE_1");

    println!("KILL tor.exe");
    let _ = std::process::Command::new("taskkill")
        .args(["/IM", "tor.exe", "/F"])
        .creation_flags(0x0800_0000)
        .status();

    let mut rx = engine.subscribe();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let mut saw_failed = false;
    loop {
        match engine.status() {
            TorStatus::Online { .. } if saw_failed => break,
            TorStatus::Failed { .. } => saw_failed = true,
            _ => {}
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("pas de relance automatique observée");
        }
        if rx.changed().await.is_err() {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    println!("ONLINE_2 (relance automatique OK)");
    println!("SMOKE_TORWATCH_OK");

    engine.shutdown().await?;
    Ok(())
}
