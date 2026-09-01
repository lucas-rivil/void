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

async fn wait_peer_offline(engine: &Engine, peer: &str, what: &str) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    loop {
        if engine.presence().iter().any(|p| p.onion_id == peer && p.online) {
            if tokio::time::Instant::now() >= deadline {
                anyhow::bail!("{what} timeout");
            }
            tokio::time::sleep(Duration::from_millis(1000)).await;
        } else {
            return Ok(());
        }
    }
}

async fn wait_history(
    engine: &Engine,
    peer: &str,
    needle: &str,
    what: &str,
) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    loop {
        let found = engine
            .dm_history(peer, 200, None)
            .unwrap_or_default()
            .iter()
            .any(|m| m.body.contains(needle));
        if found {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("{what} timeout");
        }
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
}

async fn wait_for<F: Fn() -> bool>(what: &str, probe: F) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    loop {
        if probe() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("{what} timeout");
        }
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let tor_dir = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src-tauri/resources/tor"
    ));
    let base = std::env::temp_dir();
    let dir_a = base.join("void-relay-a");
    let dir_b = base.join("void-relay-b");
    let dir_c = base.join("void-relay-c");

    let a = Engine::start(EngineConfig { data_dir: dir_a, tor_dir: tor_dir.clone() }).await?;
    let b = Engine::start(EngineConfig { data_dir: dir_b, tor_dir: tor_dir.clone() }).await?;
    let c = Engine::start(EngineConfig { data_dir: dir_c, tor_dir: tor_dir.clone() }).await?;
    wait_online(&a, "A").await?;
    wait_online(&b, "B").await?;
    wait_online(&c, "C").await?;

    let id_a = a.identity_info().onion.trim_end_matches(".onion").to_string();
    let id_b = b.identity_info().onion.trim_end_matches(".onion").to_string();
    let id_c = c.identity_info().onion.trim_end_matches(".onion").to_string();

    a.add_peer(&b.invite_link())?;
    a.add_peer(&c.invite_link())?;
    b.add_peer(&a.invite_link())?;
    b.add_peer(&c.invite_link())?;
    c.add_peer(&a.invite_link())?;
    c.add_peer(&b.invite_link())?;

    wait_peer_online(&a, &id_b, "A→B").await?;
    wait_peer_online(&a, &id_c, "A→C").await?;
    wait_peer_online(&b, &id_c, "B→C").await?;
    println!("MESH_OK");

    b.shutdown().await?;
    wait_peer_offline(&a, &id_b, "B offline vu de A").await?;
    wait_peer_offline(&c, &id_b, "B offline vu de C").await?;

    let relayed = a.send_dm(&id_b, "message relayé pendant ton absence")?;
    println!("RELAY_SEND status={:?}", relayed.status);
    assert_eq!(relayed.status, DmStatus::Sent);
    wait_for("C retient l'enveloppe", || c.relay_queue_len() >= 1).await?;
    println!("RELAY_HELD_OK");

    let b = Engine::start(EngineConfig { data_dir: base.join("void-relay-b"), tor_dir: tor_dir.clone() }).await?;
    wait_online(&b, "B2").await?;
    wait_peer_online(&b, &id_c, "B2→C").await?;
    wait_history(&b, &id_a, "message relayé pendant ton absence", "B reçoit le relayé").await?;
    println!("RELAY_DELIVERED_OK");
    wait_for("C purge l'enveloppe", || c.relay_queue_len() == 0).await?;

    a.shutdown().await?;
    c.shutdown().await?;
    b.shutdown().await?;
    wait_peer_offline(&a, &id_b, "aucun pair").await.ok();

    let a = Engine::start(EngineConfig { data_dir: base.join("void-relay-a"), tor_dir: tor_dir.clone() }).await?;
    wait_online(&a, "A2").await?;
    wait_peer_offline(&a, &id_b, "B offline pour A2").await.ok();
    let queued = a.send_dm(&id_b, "message en file locale")?;
    println!("QUEUE_SEND status={:?}", queued.status);
    assert_eq!(queued.status, DmStatus::Queued);

    let c = Engine::start(EngineConfig { data_dir: base.join("void-relay-c"), tor_dir: tor_dir.clone() }).await?;
    wait_online(&c, "C2").await?;
    wait_peer_online(&a, &id_c, "A2→C2").await?;
    wait_for("C2 retient le message en file", || c.relay_queue_len() >= 1).await?;
    let status_now = a
        .dm_history(&id_b, 200, None)?
        .iter()
        .find(|m| m.body.contains("message en file locale"))
        .map(|m| m.status);
    println!("QUEUE_FLUSHED status={status_now:?}");
    assert_eq!(status_now, Some(DmStatus::Sent));

    let b = Engine::start(EngineConfig { data_dir: base.join("void-relay-b"), tor_dir: tor_dir.clone() }).await?;
    wait_online(&b, "B3").await?;
    wait_peer_online(&b, &id_c, "B3→C2").await?;
    wait_history(&b, &id_a, "message en file locale", "B reçoit le message en file").await?;
    println!("QUEUE_DELIVERED_OK");

    a.shutdown().await?;
    b.shutdown().await?;
    c.shutdown().await?;
    println!("SMOKE_RELAY_OK");
    Ok(())
}
