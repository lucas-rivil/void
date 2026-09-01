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

async fn wait_peer_online(
    engine: &Engine,
    peer_onion_id: &str,
    what: &str,
) -> anyhow::Result<()> {
    let mut rx = engine.subscribe_presence();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    loop {
        let found = engine
            .presence()
            .iter()
            .any(|p| p.onion_id == peer_onion_id && p.online);
        if found {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("{what} timeout");
        }
        if rx.changed().await.is_err() {
            tokio::time::sleep(Duration::from_millis(500)).await;
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
    let dir_a = std::env::temp_dir().join("void-p2p-a");
    let dir_b = std::env::temp_dir().join("void-p2p-b");

    let a = Engine::start(EngineConfig { data_dir: dir_a, tor_dir: tor_dir.clone() }).await?;
    let b = Engine::start(EngineConfig { data_dir: dir_b, tor_dir }).await?;

    wait_online(&a, "A").await?;
    wait_online(&b, "B").await?;
    let onion_a = a.identity_info().onion;
    let onion_b = b.identity_info().onion;
    println!("ONLINE_A={onion_a}");
    println!("ONLINE_B={onion_b}");

    a.add_peer(&b.invite_link())?;
    b.add_peer(&a.invite_link())?;

    let id_a = onion_a.trim_end_matches(".onion").to_string();
    let id_b = onion_b.trim_end_matches(".onion").to_string();

    wait_peer_online(&a, &id_b, "A→B").await?;
    println!("A voit B en ligne");
    wait_peer_online(&b, &id_a, "B→A").await?;
    println!("B voit A en ligne");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let rtt_a = a
            .presence()
            .iter()
            .find(|p| p.onion_id == id_b)
            .and_then(|p| p.rtt_ms);
        let rtt_b = b
            .presence()
            .iter()
            .find(|p| p.onion_id == id_a)
            .and_then(|p| p.rtt_ms);
        if let (Some(ra), Some(rb)) = (rtt_a, rtt_b) {
            println!("RTT_A={ra}ms RTT_B={rb}ms");
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            println!("RTT indisponible (ping loop 20s)");
            break;
        }
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }

    a.shutdown().await?;
    b.shutdown().await?;
    println!("SMOKE_P2P_OK");
    Ok(())
}
