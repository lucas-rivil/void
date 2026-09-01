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

async fn wait_for<T>(
    what: &str,
    probe: impl Fn() -> Option<T>,
) -> anyhow::Result<T> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(value) = probe() {
            return Ok(value);
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
    let dir_a = std::env::temp_dir().join("void-groups-a");
    let dir_b = std::env::temp_dir().join("void-groups-b");

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

    let group = a.create_group("la bande à void", &[])?;
    println!("GROUP_CREATED={} members={}", group.group_id, group.members.len());

    let updated = a.add_group_member(&group.group_id, &id_b)?;
    assert_eq!(updated.members.len(), 2);
    wait_for("B reçoit l'invitation", || {
        b.groups().into_iter().find(|g| g.group_id == group.group_id)
    })
    .await?;
    let group_b = b.groups().into_iter().find(|g| g.group_id == group.group_id).unwrap();
    assert_eq!(group_b.members.len(), 2);
    assert_eq!(group_b.name, "la bande à void");
    println!("INVITE_OK");

    a.send_group_message(&group.group_id, "premier message de groupe ✨")?;
    wait_for("B reçoit le message de groupe", || {
        b.group_history(&group.group_id, 50, None)
            .unwrap_or_default()
            .into_iter()
            .find(|m| m.body.contains("premier message"))
    })
    .await?;
    println!("GROUP_MSG_A2B_OK");

    b.send_group_message(&group.group_id, "réponse du groupe")?;
    wait_for("A reçoit la réponse", || {
        a.group_history(&group.group_id, 50, None)
            .unwrap_or_default()
            .into_iter()
            .find(|m| m.body.contains("réponse du groupe"))
    })
    .await?;
    println!("GROUP_MSG_B2A_OK");

    a.remove_group_member(&group.group_id, &id_b)?;
    wait_for("B apprend son exclusion", || {
        (!b.groups().iter().any(|g| g.group_id == group.group_id)).then_some(())
    })
    .await?;
    let after = a.groups().into_iter().find(|g| g.group_id == group.group_id).unwrap();
    assert_eq!(after.members.len(), 1);
    println!("REMOVE_OK");

    assert!(b.send_group_message(&group.group_id, "post-exclusion").is_err());
    let history_b = b.group_history(&group.group_id, 50, None);
    assert!(history_b.is_err() || history_b.unwrap().is_empty());
    println!("HISTORY_PURGED_OK");

    a.shutdown().await?;
    b.shutdown().await?;
    println!("SMOKE_GROUPS_OK");
    Ok(())
}
