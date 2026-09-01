use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha3::Digest;

pub const ONION_CHECKSUM_PREFIX: &[u8] = b".onion checksum";
pub const ONION_VERSION_BYTE: u8 = 0x03;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("phrase de récupération invalide: {0}")]
    Mnemonic(String),
    #[error("adresse oignon invalide: {0}")]
    InvalidOnion(String),
    #[error("dérivation de clé impossible: {0}")]
    Kdf(String),
}

pub mod dm;

pub fn recovery_phrase(seed: &[u8; 32]) -> Result<String, CryptoError> {
    bip39::Mnemonic::from_entropy(seed)
        .map(|mnemonic| mnemonic.to_string())
        .map_err(|e| CryptoError::Mnemonic(e.to_string()))
}

pub fn seed_from_recovery_phrase(phrase: &str) -> Result<[u8; 32], CryptoError> {
    let normalized = phrase.split_whitespace().collect::<Vec<_>>().join(" ");
    let normalized = normalized.to_lowercase();
    let mnemonic = bip39::Mnemonic::parse_normalized(&normalized)
        .map_err(|e| CryptoError::Mnemonic(e.to_string()))?;
    let entropy = mnemonic.to_entropy();
    entropy
        .try_into()
        .map_err(|_| CryptoError::Mnemonic("taille d'entropie inattendue".into()))
}

pub fn hex_encode(data: &[u8]) -> String {
    hex::encode(data)
}

pub fn onion_id_is_valid(id: &str) -> bool {
    id.len() == 56 && onion_id_to_public(id).is_some()
}

pub fn onion_id_to_public(id: &str) -> Option<[u8; 32]> {
    if id.len() != 56 {
        return None;
    }
    let bytes = base32_decode_lower(id)?;
    if bytes.len() != 35 {
        return None;
    }
    let mut public = [0u8; 32];
    public.copy_from_slice(&bytes[..32]);
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(ONION_CHECKSUM_PREFIX);
    hasher.update(public);
    hasher.update([ONION_VERSION_BYTE]);
    let digest = hasher.finalize();
    if digest[0] == bytes[32] && digest[1] == bytes[33] && bytes[34] == ONION_VERSION_BYTE {
        Some(public)
    } else {
        None
    }
}

pub fn base32_decode_lower(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 5 / 8);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for c in s.bytes() {
        let value = match c {
            b'a'..=b'z' => c - b'a',
            b'2'..=b'7' => c - b'2' + 26,
            _ => return None,
        };
        buffer = (buffer << 5) | value as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

#[derive(Clone)]
pub struct Identity {
    signing: SigningKey,
}

impl Identity {
    pub fn generate() -> Self {
        let mut csprng = rand_core::OsRng;
        Self {
            signing: SigningKey::generate(&mut csprng),
        }
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            signing: SigningKey::from_bytes(&seed),
        }
    }

    pub fn seed(&self) -> [u8; 32] {
        self.signing.to_bytes()
    }

    pub fn public(&self) -> VerifyingKey {
        self.signing.verifying_key()
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing.sign(message)
    }

    pub fn sign_bytes(&self, message: &[u8]) -> [u8; 64] {
        self.signing.sign(message).to_bytes()
    }

    pub fn verify(public: &VerifyingKey, message: &[u8], signature: &Signature) -> bool {
        ed25519_dalek::Verifier::verify(public, message, signature).is_ok()
    }

    pub fn verify_bytes(public: &VerifyingKey, message: &[u8], signature: &[u8; 64]) -> bool {
        let signature = Signature::from_bytes(signature);
        Self::verify(public, message, &signature)
    }

    pub fn verify_public_bytes(
        public: &[u8; 32],
        message: &[u8],
        signature: &[u8; 64],
    ) -> bool {
        let Ok(public) = VerifyingKey::from_bytes(public) else {
            return false;
        };
        Self::verify_bytes(&public, message, signature)
    }

    pub fn expanded_key_blob(&self) -> [u8; 64] {
        expanded_key_blob_from_seed(&self.seed())
    }

    pub fn onion_service_key_b64(&self) -> String {
        let blob = self.expanded_key_blob();
        base64::engine::general_purpose::STANDARD.encode(blob)
    }

    pub fn onion_id(&self) -> String {
        onion_id_from_public(self.public().as_bytes())
    }

    pub fn onion_address(&self) -> String {
        format!("{}.onion", self.onion_id())
    }

    pub fn fingerprint_short(&self) -> String {
        hex::encode(self.public().as_bytes())[..16].to_string()
    }
}

pub fn expanded_key_blob_from_seed(seed: &[u8; 32]) -> [u8; 64] {
    use sha2::Digest;
    let mut hasher = sha2::Sha512::new();
    hasher.update(seed);
    let digest = hasher.finalize();
    let mut blob = [0u8; 64];
    blob.copy_from_slice(&digest);
    blob[0] &= 248;
    blob[31] &= 127;
    blob[31] |= 64;
    blob
}

pub fn public_from_expanded(blob: &[u8; 64]) -> [u8; 32] {
    let mut scalar_bytes = [0u8; 32];
    scalar_bytes.copy_from_slice(&blob[..32]);
    #[allow(deprecated)]
    let scalar = curve25519_dalek::scalar::Scalar::from_bits(scalar_bytes);
    let point = &scalar * &curve25519_dalek::constants::ED25519_BASEPOINT_POINT;
    let compressed = point.compress();
    *compressed.as_bytes()
}

pub fn onion_id_from_expanded(blob: &[u8; 64]) -> String {
    onion_id_from_public(&public_from_expanded(blob))
}

pub fn onion_id_from_public(public: &[u8; 32]) -> String {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(ONION_CHECKSUM_PREFIX);
    hasher.update(public);
    hasher.update([ONION_VERSION_BYTE]);
    let digest = hasher.finalize();

    let mut data = [0u8; 35];
    data[..32].copy_from_slice(public);
    data[32] = digest[0];
    data[33] = digest[1];
    data[34] = ONION_VERSION_BYTE;
    base32_lower_nopad(&data)
}

pub fn base32_lower_nopad(data: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut out = String::with_capacity(data.len().div_ceil(5) * 8);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in data {
        buffer = (buffer << 8) | byte as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let index = ((buffer >> bits) & 0x1f) as usize;
            out.push(ALPHABET[index] as char);
        }
    }
    if bits > 0 {
        let index = ((buffer << (5 - bits)) & 0x1f) as usize;
        out.push(ALPHABET[index] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b32(bytes: &[u8]) -> String {
        base32_lower_nopad(bytes)
    }

    #[test]
    fn base32_rfc4648_vectors() {
        assert_eq!(b32(b"f"), "my");
        assert_eq!(b32(b"fo"), "mzxq");
        assert_eq!(b32(b"foo"), "mzxw6");
        assert_eq!(b32(b"foob"), "mzxw6yq");
        assert_eq!(b32(b"fooba"), "mzxw6ytb");
        assert_eq!(b32(b"foobar"), "mzxw6ytboi");
    }

    #[test]
    fn onion_id_shape() {
        let id = Identity::generate().onion_id();
        assert_eq!(id.len(), 56);
        assert!(id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        assert!(!id.contains('='))
    }

    #[test]
    fn onion_id_deterministic() {
        let seed = [42u8; 32];
        let a = Identity::from_seed(seed).onion_id();
        let b = Identity::from_seed(seed).onion_id();
        assert_eq!(a, b);
        let c = Identity::from_seed([43u8; 32]).onion_id();
        assert_ne!(a, c);
    }

    #[test]
    fn expanded_blob_shape() {
        let id = Identity::generate();
        let blob = id.expanded_key_blob();
        assert_eq!(blob[0] & 7, 0);
        assert_eq!(blob[31] & 128, 0);
        assert_eq!(blob[31] & 64, 64);
        assert_eq!(public_from_expanded(&blob), *id.public().as_bytes());
    }

    #[test]
    fn onion_service_key_roundtrip() {
        let id = Identity::generate();
        let b64 = id.onion_service_key_b64();
        let raw = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        assert_eq!(raw.len(), 64);
        let blob: [u8; 64] = raw.try_into().unwrap();
        assert_eq!(onion_id_from_expanded(&blob), id.onion_id());
    }

    #[test]
    fn sign_verify() {
        let id = Identity::generate();
        let sig = id.sign(b"void");
        assert!(Identity::verify(&id.public(), b"void", &sig));
        assert!(!Identity::verify(&id.public(), b"voix", &sig));
    }

    #[test]
    fn recovery_phrase_roundtrip() {
        let seed = [7u8; 32];
        let phrase = recovery_phrase(&seed).unwrap();
        let words: Vec<&str> = phrase.split(' ').collect();
        assert_eq!(words.len(), 24);
        let recovered = seed_from_recovery_phrase(&phrase).unwrap();
        assert_eq!(recovered, seed);
    }

    #[test]
    fn recovery_phrase_accepts_dirty_input() {
        let seed = [7u8; 32];
        let phrase = recovery_phrase(&seed).unwrap();
        let dirty = format!("  {}  ", phrase.replace(' ', "   "));
        assert_eq!(seed_from_recovery_phrase(&dirty).unwrap(), seed);
    }

    #[test]
    fn recovery_phrase_rejects_bad_checksum() {
        let seed = [7u8; 32];
        let phrase = recovery_phrase(&seed).unwrap();
        let mut words: Vec<String> = phrase.split(' ').map(String::from).collect();
        words[23] = String::from("zoo");
        let tampered = words.join(" ");
        assert!(seed_from_recovery_phrase(&tampered).is_err());
    }

    #[test]
    fn onion_id_validation() {
        let id = Identity::generate().onion_id();
        assert!(onion_id_is_valid(&id));
        let mut corrupted = id.clone();
        let last = corrupted.pop().unwrap();
        corrupted.push(if last == 'a' { 'b' } else { 'a' });
        assert!(!onion_id_is_valid(&corrupted));
        assert!(!onion_id_is_valid("abc"));
        assert!(!onion_id_is_valid(&format!("{id}0")));
    }

    #[test]
    fn onion_id_to_public_roundtrip() {
        let identity = Identity::generate();
        let id = identity.onion_id();
        let public = onion_id_to_public(&id).unwrap();
        assert_eq!(public, *identity.public().as_bytes());
        assert!(onion_id_to_public(&format!("{id}0")).is_none());
    }

    #[test]
    fn sign_verify_bytes() {
        let identity = Identity::generate();
        let sig = identity.sign_bytes(b"void-handshake");
        assert!(Identity::verify_public_bytes(
            identity.public().as_bytes(),
            b"void-handshake",
            &sig
        ));
        let mut tampered = sig;
        tampered[0] ^= 1;
        assert!(!Identity::verify_public_bytes(
            identity.public().as_bytes(),
            b"void-handshake",
            &tampered
        ));
        assert!(!Identity::verify_public_bytes(
            identity.public().as_bytes(),
            b"void-autre",
            &sig
        ));
    }
}
