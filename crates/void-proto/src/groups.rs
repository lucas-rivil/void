use serde::{Deserialize, Serialize};

pub const GROUP_AAD_PREFIX: &[u8] = b"void-group-msg-v1";
pub const GROUP_KEY_AAD_PREFIX: &[u8] = b"void-group-key-v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemberEntry {
    pub onion_id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyBlob {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupEnvelope {
    pub message_id: [u8; 16],
    pub group_id: String,
    pub author_id: String,
    pub timestamp_ms: u64,
    pub kind: u8,
    pub duration_ms: u32,
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

pub const KIND_TEXT: u8 = 0;
pub const KIND_VOICE: u8 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelayItem {
    pub id: [u8; 16],
    pub kind: u8,
    pub payload: Vec<u8>,
}

pub const RELAY_KIND_DM: u8 = 1;
pub const RELAY_KIND_GROUP: u8 = 2;

pub fn encode_group_envelope(envelope: &GroupEnvelope) -> Vec<u8> {
    postcard::to_allocvec(envelope).unwrap_or_default()
}

pub fn decode_group_envelope(bytes: &[u8]) -> Option<GroupEnvelope> {
    postcard::from_bytes(bytes).ok()
}

pub fn group_aad_parts(
    message_id: &[u8; 16],
    group_id: &str,
    author_id: &str,
    timestamp_ms: u64,
) -> Vec<u8> {
    let mut aad = Vec::with_capacity(
        GROUP_AAD_PREFIX.len() + 16 + group_id.len() + author_id.len() + 8,
    );
    aad.extend_from_slice(GROUP_AAD_PREFIX);
    aad.extend_from_slice(message_id);
    aad.extend_from_slice(group_id.as_bytes());
    aad.extend_from_slice(author_id.as_bytes());
    aad.extend_from_slice(&timestamp_ms.to_be_bytes());
    aad
}

pub fn group_aad(envelope: &GroupEnvelope) -> Vec<u8> {
    group_aad_parts(
        &envelope.message_id,
        &envelope.group_id,
        &envelope.author_id,
        envelope.timestamp_ms,
    )
}

pub fn group_key_aad(group_id: &str, recipient_id: &str) -> Vec<u8> {
    let mut aad = Vec::with_capacity(GROUP_KEY_AAD_PREFIX.len() + group_id.len() + recipient_id.len());
    aad.extend_from_slice(GROUP_KEY_AAD_PREFIX);
    aad.extend_from_slice(group_id.as_bytes());
    aad.extend_from_slice(recipient_id.as_bytes());
    aad
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aad_is_deterministic_and_bound() {
        let envelope = GroupEnvelope {
            message_id: [1; 16],
            group_id: "01HZZZZZZZZZZZZZZZZZZZZZZZZ".into(),
            author_id: "a".repeat(56),
            timestamp_ms: 42,
            kind: 0,
            duration_ms: 0,
            nonce: [0; 12],
            ciphertext: vec![1, 2, 3],
        };
        assert_eq!(group_aad(&envelope), group_aad(&envelope));
        let mut other = envelope.clone();
        other.group_id = "01HZZZZZZZZZZZZZZZZZZZZZZZY".into();
        assert_ne!(group_aad(&envelope), group_aad(&other));
    }

    #[test]
    fn key_aad_recipient_bound() {
        let a = group_key_aad("g", "alice");
        let b = group_key_aad("g", "bob");
        assert_ne!(a, b);
        assert!(a.starts_with(GROUP_KEY_AAD_PREFIX));
    }
}
