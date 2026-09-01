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
    let data_dir = std::env::temp_dir().join("void-smoke-restore");

    let engine = Engine::start(EngineConfig { data_dir, tor_dir }).await?;
    wait_online(&engine, "premier bootstrap").await?;
    let first_onion = engine.identity_info().onion;
    println!("ONLINE1={first_onion}");

    let phrase = engine.recovery_phrase()?;
    let words = phrase.split(' ').count();
    assert_eq!(words, 24, "la phrase doit faire 24 mots");

    let future_identity = void_crypto::Identity::generate();
    let new_seed = future_identity.seed();
    let new_phrase = void_crypto::recovery_phrase(&new_seed)?;
    let expected_onion = future_identity.onion_address();

    let info = engine.restore_from_phrase(&new_phrase).await?;
    assert_eq!(info.onion, expected_onion);
    assert_ne!(info.onion, first_onion);
    assert!(engine.recovery_confirmed());

    wait_online(&engine, "bootstrap après restauration").await?;
    match engine.status() {
        TorStatus::Online { onion, .. } => {
            assert_eq!(onion, expected_onion);
            println!("ONLINE2={onion}");
        }
        _ => anyhow::bail!("statut inattendu"),
    }

    let relink = engine.restore_from_phrase(&phrase).await?;
    assert_eq!(relink.onion, first_onion);
    wait_online(&engine, "retour identité initiale").await?;
    println!("ONLINE3={}", engine.identity_info().onion);

    engine.shutdown().await?;
    println!("SMOKE_RESTORE_OK");
    Ok(())
}
