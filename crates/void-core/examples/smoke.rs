use std::time::Duration;

use void_core::{Engine, EngineConfig, TorStatus};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let args: Vec<String> = std::env::args().collect();
    let tor_dir = std::path::PathBuf::from(
        args.get(1).cloned().unwrap_or_else(|| {
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../apps/desktop/src-tauri/resources/tor"
            )
            .to_string()
        }),
    );
    let data_dir = std::env::temp_dir().join("void-smoke");

    let engine = Engine::start(EngineConfig { data_dir, tor_dir }).await?;
    println!("identité: {}", engine.identity_info().onion);

    let mut rx = engine.subscribe();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    loop {
        let status = engine.status();
        println!("status = {status:?}");
        match status {
            TorStatus::Online { .. } => break,
            TorStatus::Failed { error } => anyhow::bail!("échec: {error}"),
            _ => {}
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("timeout global du smoke test");
        }
        if rx.changed().await.is_err() {
            break;
        }
    }

    if let TorStatus::Online { onion, socks } = engine.status() {
        println!("ONION_OK={onion} SOCKS={socks}");
    }

    tokio::time::sleep(Duration::from_secs(5)).await;
    engine.shutdown().await?;
    println!("SMOKE_OK");
    Ok(())
}
