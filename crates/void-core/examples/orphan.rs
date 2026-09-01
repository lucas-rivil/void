use std::time::Duration;

use void_core::{Engine, EngineConfig, TorStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let tor_dir = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src-tauri/resources/tor"
    ));
    let data_dir = std::env::temp_dir().join("void-orphan");
    let _ = std::fs::remove_dir_all(&data_dir);
    let engine = Engine::start(EngineConfig { data_dir, tor_dir }).await?;
    let mut rx = engine.subscribe();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    loop {
        if let TorStatus::Online { onion, .. } = engine.status() {
            println!("ORPHAN_ONLINE={onion}");
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("timeout");
        }
        if rx.changed().await.is_err() {
            anyhow::bail!("watch fermé");
        }
    }
    println!("ORPHAN_READY pid={}", std::process::id());
    loop {
        tokio::time::sleep(Duration::from_secs(3600)).await;
    }
}
