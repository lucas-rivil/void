use base64::Engine;
use void_crypto::{onion_id_from_expanded, Identity};
use void_tor::{launch, TorConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    let tor_dir = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src-tauri/resources/tor"
    ));
    let data_dir = std::env::temp_dir().join("void-keycheck");
    let cfg = TorConfig { tor_dir, data_dir };
    let mut boot = launch(&cfg).await?;

    let reply = boot
        .control
        .send("ADD_ONION NEW:ED25519-V3 Flags=Detach Port=65534")
        .await?;

    let mut new_service_id = String::new();
    let mut new_private_key = String::new();
    for line in &reply {
        if let Some(v) = line.strip_prefix("ServiceID=") {
            new_service_id = v.to_string();
        }
        if let Some(v) = line.strip_prefix("PrivateKey=ED25519-V3:") {
            new_private_key = v.to_string();
        }
    }
    println!("NEW ServiceID      = {new_service_id}");

    let new_blob: [u8; 64] = base64::engine::general_purpose::STANDARD
        .decode(&new_private_key)?
        .try_into()
        .unwrap();
    let derived = onion_id_from_expanded(&new_blob);
    println!("notre id (expanded) = {derived}");
    println!("expanded correspond au ServiceID : {}", derived == new_service_id);

    let identity = Identity::generate();
    let reply2 = boot
        .control
        .send(&format!(
            "ADD_ONION ED25519-V3:{} Flags=Detach Port=65533",
            identity.onion_service_key_b64()
        ))
        .await?;
    let mut own_service_id = String::new();
    for line in &reply2 {
        if let Some(v) = line.strip_prefix("ServiceID=") {
            own_service_id = v.to_string();
        }
    }
    let expected = identity.onion_id();
    println!("ServiceID pour NOTRE clé = {own_service_id}");
    println!("notre onion_id attendu    = {expected}");
    println!("correspondance : {}", own_service_id == expected);

    Ok(())
}
