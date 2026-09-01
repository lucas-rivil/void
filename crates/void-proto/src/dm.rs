use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DmEnvelope {
    pub message_id: [u8; 16],
    pub author_id: String,
    pub recipient_id: String,
    pub timestamp_ms: u64,
    pub kind: u8,
    pub duration_ms: u32,
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

pub fn encode_dm(envelope: &DmEnvelope) -> Vec<u8> {
    postcard::to_allocvec(envelope).unwrap_or_default()
}

pub fn decode_dm(bytes: &[u8]) -> Option<DmEnvelope> {
    postcard::from_bytes(bytes).ok()
}

pub fn dm_aad_parts(
    message_id: &[u8; 16],
    author_id: &str,
    recipient_id: &str,
    timestamp_ms: u64,
) -> Vec<u8> {
    let mut aad = Vec::with_capacity(16 + author_id.len() + recipient_id.len() + 8);
    aad.extend_from_slice(message_id);
    aad.extend_from_slice(author_id.as_bytes());
    aad.extend_from_slice(recipient_id.as_bytes());
    aad.extend_from_slice(&timestamp_ms.to_be_bytes());
    aad
}

pub fn dm_aad(envelope: &DmEnvelope) -> Vec<u8> {
    dm_aad_parts(
        &envelope.message_id,
        &envelope.author_id,
        &envelope.recipient_id,
        envelope.timestamp_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DmEnvelope {
        DmEnvelope {
            message_id: [7; 16],
            author_id: "a".repeat(56),
            recipient_id: "b".repeat(56),
            timestamp_ms: 1_700_000_000_000,
            kind: 1,
            duration_ms: 4200,
            nonce: [3; 12],
            ciphertext: vec![1, 2, 3, 4, 5, 6, 7],
        }
    }

    #[test]
    fn roundtrip() {
        let envelope = sample();
        let bytes = encode_dm(&envelope);
        assert_eq!(decode_dm(&bytes).unwrap(), envelope);
    }

    #[test]
    fn decode_garbage_none() {
        assert!(decode_dm(&[255, 255, 255]).is_none());
    }

    #[test]
    fn aad_is_deterministic_and_bound() {
        let envelope = sample();
        let a = dm_aad(&envelope);
        assert_eq!(a, dm_aad(&envelope));
        let mut other = envelope.clone();
        other.timestamp_ms += 1;
        assert_ne!(a, dm_aad(&other));
    }
}
