use std::time::Duration;

use void_core::{Engine, EngineConfig, MessageKind, TorStatus};

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let tor_dir = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src-tauri/resources/tor"
    ));
    let dir_a = std::env::temp_dir().join("void-voice-a");
    let dir_b = std::env::temp_dir().join("void-voice-b");

    let a = Engine::start(EngineConfig { data_dir: dir_a, tor_dir: tor_dir.clone() }).await?;
    let b = Engine::start(EngineConfig { data_dir: dir_b, tor_dir }).await?;
    wait_online(&a, "A").await?;
    wait_online(&b, "B").await?;

    let id_a = a.identity_info().onion.trim_end_matches(".onion").to_string();
    let id_b = b.identity_info().onion.trim_end_matches(".onion").to_string();

    a.add_peer(&b.invite_link())?;
    b.add_peer(&a.invite_link())?;
    wait_peer_online(&a, &id_b, "A→B").await?;
    wait_peer_online(&b, &id_a, "B→A").await?;
    println!("SESSION_OK");

    let fake_audio: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
    let sent = a.send_voice_dm(&id_b, &fake_audio, 3120)?;
    assert_eq!(sent.kind, MessageKind::Voice);
    assert_eq!(sent.duration_ms, 3120);
    assert!(sent.body.is_empty());
    println!(
        "SENT voice id={} status={:?}",
        sent.message_id, sent.status
    );

    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    loop {
        let received = b
            .dm_history(&id_a, 50, None)
            .unwrap_or_default()
            .into_iter()
            .find(|m| m.message_id == sent.message_id);
        if let Some(received) = received {
            assert_eq!(received.kind, MessageKind::Voice);
            assert_eq!(received.duration_ms, 3120);
            let blob = b
                .voice_blob(&received.message_id)
                .expect("blob vocal absent");
            assert_eq!(blob, fake_audio);
            println!("RECEIVED_OK blob={} octets identiques", blob.len());
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("B n'a pas reçu la note vocale");
        }
        tokio::time::sleep(Duration::from_millis(1500)).await;
    }

    let text_after = a.send_dm(&id_b, "texte apres audio")?;
    assert_eq!(text_after.kind, MessageKind::Text);
    println!("TEXT_STILL_WORKS");

    a.shutdown().await?;
    b.shutdown().await?;
    println!("SMOKE_VOICE_OK");
    Ok(())
}
