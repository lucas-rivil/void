use std::time::Duration;

use void_core::{DmStatus, Engine, EngineConfig, TorStatus};

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

async fn wait_peer_online(engine: &Engine, peer: &str, what: &str) -> anyhow::Result<()> {
    let mut rx = engine.subscribe_presence();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    loop {
        if engine.presence().iter().any(|p| p.onion_id == peer && p.online) {
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

async fn wait_history_contains(
    engine: &Engine,
    peer: &str,
    needle: &str,
    what: &str,
) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    loop {
        let history = engine.dm_history(peer, 100, None).unwrap_or_default();
        if history.iter().any(|m| m.body.contains(needle)) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("{what} timeout");
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}

async fn wait_status(
    engine: &Engine,
    peer: &str,
    message_id: &str,
    status: DmStatus,
    what: &str,
) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    loop {
        let history = engine.dm_history(peer, 100, None).unwrap_or_default();
        if history
            .iter()
            .any(|m| m.message_id == message_id && m.status == status)
        {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("{what} timeout");
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let tor_dir = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src-tauri/resources/tor"
    ));
    let dir_a = std::env::temp_dir().join("void-dm-a");
    let dir_b = std::env::temp_dir().join("void-dm-b");

    let a = Engine::start(EngineConfig { data_dir: dir_a, tor_dir: tor_dir.clone() }).await?;
    let b = Engine::start(EngineConfig { data_dir: dir_b, tor_dir }).await?;
    wait_online(&a, "A").await?;
    wait_online(&b, "B").await?;

    let id_a = a.identity_info().onion.trim_end_matches(".onion").to_string();
    let id_b = b.identity_info().onion.trim_end_matches(".onion").to_string();

    a.add_peer(&b.invite_link())?;
    b.add_peer(&a.invite_link())?;
    wait_peer_online(&a, &id_b, "session A→B").await?;
    wait_peer_online(&b, &id_a, "session B→A").await?;
    println!("SESSION_OK");

    let sent = a.send_dm(&id_b, "salut void, message chiffré ✨")?;
    println!("SENT={} status={:?}", sent.message_id, sent.status);
    wait_history_contains(&b, &id_a, "salut void", "B reçoit").await?;
    println!("B_RECU_OK");
    wait_status(&a, &id_b, &sent.message_id, DmStatus::Delivered, "accusé A").await?;
    println!("ACK_OK");

    let reply = b.send_dm(&id_a, "bien reçu !")?;
    wait_history_contains(&a, &id_b, "bien reçu", "A reçoit").await?;
    println!("A_RECU_OK");
    wait_status(&b, &id_a, &reply.message_id, DmStatus::Delivered, "accusé B").await?;
    println!("ACK2_OK");

    let history_a = a.dm_history(&id_b, 50, None)?;
    assert_eq!(history_a.len(), 2);
    let history_b = b.dm_history(&id_a, 50, None)?;
    assert_eq!(history_b.len(), 2);
    println!("HISTORY_A={} HISTORY_B={}", history_a.len(), history_b.len());

    a.shutdown().await?;
    b.shutdown().await?;
    println!("SMOKE_DM_OK");
    Ok(())
}
