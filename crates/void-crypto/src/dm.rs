use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey as XPublicKey, StaticSecret};

use crate::{CryptoError, Identity};

pub const DM_INFO: &[u8] = b"void-dm-v1";

pub fn ed25519_public_to_x25519(public: &[u8; 32]) -> Option<[u8; 32]> {
    let compressed = curve25519_dalek::edwards::CompressedEdwardsY::from_slice(public).ok()?;
    let point = compressed.decompress()?;
    Some(point.to_montgomery().to_bytes())
}

pub fn ed25519_seed_to_x25519(seed: &[u8; 32]) -> [u8; 32] {
    let blob = crate::expanded_key_blob_from_seed(seed);
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&blob[..32]);
    secret
}

pub fn conversation_key(
    my_identity: &Identity,
    peer_onion_id: &str,
) -> Result<[u8; 32], CryptoError> {
    let peer_public = crate::onion_id_to_public(peer_onion_id)
        .ok_or_else(|| CryptoError::InvalidOnion(peer_onion_id.to_string()))?;
    let peer_x25519 = ed25519_public_to_x25519(&peer_public)
        .ok_or_else(|| CryptoError::InvalidOnion(peer_onion_id.to_string()))?;

    let my_secret_bytes = ed25519_seed_to_x25519(&my_identity.seed());
    let my_secret = StaticSecret::from(my_secret_bytes);
    let shared = my_secret.diffie_hellman(&XPublicKey::from(peer_x25519));

    let my_id = my_identity.onion_id();
    let (first, second) = if my_id.as_str() <= peer_onion_id {
        (my_id, peer_onion_id.to_string())
    } else {
        (peer_onion_id.to_string(), my_id)
    };
    let salt = format!("{first}{second}");

    let hkdf = Hkdf::<Sha256>::new(Some(salt.as_bytes()), shared.as_bytes());
    let mut okm = [0u8; 32];
    hkdf.expand(DM_INFO, &mut okm)
        .map_err(|e| CryptoError::Kdf(e.to_string()))?;
    Ok(okm)
}

pub fn dm_encrypt(
    key: &[u8; 32],
    aad: &[u8],
    plaintext: &[u8],
) -> Option<([u8; 12], Vec<u8>)> {
    use rand_core::RngCore;
    let mut nonce_bytes = [0u8; 12];
    rand_core::OsRng.fill_bytes(&mut nonce_bytes);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce_bytes),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .ok()?;
    Some((nonce_bytes, ciphertext))
}

pub fn dm_decrypt(
    key: &[u8; 32],
    nonce: &[u8; 12],
    aad: &[u8],
    ciphertext: &[u8],
) -> Option<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peers() -> (Identity, Identity) {
        (Identity::generate(), Identity::generate())
    }

    #[test]
    fn conversation_key_symmetric() {
        let (a, b) = peers();
        let key_a = conversation_key(&a, &b.onion_id()).unwrap();
        let key_b = conversation_key(&b, &a.onion_id()).unwrap();
        assert_eq!(key_a, key_b);
    }

    #[test]
    fn conversation_key_distinct_per_pair() {
        let (a, b) = peers();
        let c = Identity::generate();
        let ab = conversation_key(&a, &b.onion_id()).unwrap();
        let ac = conversation_key(&a, &c.onion_id()).unwrap();
        assert_ne!(ab, ac);
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let (a, b) = peers();
        let key = conversation_key(&a, &b.onion_id()).unwrap();
        let aad = b"void-aad";
        let (nonce, ct) = dm_encrypt(&key, aad, "salut ✨".as_bytes()).unwrap();
        let plain = dm_decrypt(&key, &nonce, aad, &ct).unwrap();
        assert_eq!(plain, "salut ✨".as_bytes());
    }

    #[test]
    fn tamper_fails() {
        let (a, b) = peers();
        let key = conversation_key(&a, &b.onion_id()).unwrap();
        let (nonce, mut ct) = dm_encrypt(&key, b"aad", b"secret").unwrap();
        let len = ct.len();
        ct[len - 1] ^= 1;
        assert!(dm_decrypt(&key, &nonce, b"aad", &ct).is_none());
    }

    #[test]
    fn wrong_aad_fails() {
        let (a, b) = peers();
        let key = conversation_key(&a, &b.onion_id()).unwrap();
        let (nonce, ct) = dm_encrypt(&key, b"aad-1", b"secret").unwrap();
        assert!(dm_decrypt(&key, &nonce, b"aad-2", &ct).is_none());
    }

    #[test]
    fn wrong_key_fails() {
        let (a, b) = peers();
        let key = conversation_key(&a, &b.onion_id()).unwrap();
        let other = conversation_key(&b, &Identity::generate().onion_id()).unwrap();
        let (nonce, ct) = dm_encrypt(&key, b"aad", b"secret").unwrap();
        assert!(dm_decrypt(&other, &nonce, b"aad", &ct).is_none());
    }

    #[test]
    fn invalid_onion_rejected() {
        let (a, _) = peers();
        assert!(conversation_key(&a, "pas-un-onion").is_err());
    }
}
