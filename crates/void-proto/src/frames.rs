use serde::{Deserialize, Serialize};

pub const MAX_FRAME_SIZE: usize = 1024 * 1024;

mod sig64 {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        value: &[u8; 64],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(value)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; 64], D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = [u8; 64];
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("64 octets")
            }
            fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
                value
                    .try_into()
                    .map_err(|_| E::invalid_length(value.len(), &"64 octets"))
            }
        }
        deserializer.deserialize_bytes(Visitor)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Frame {
    Hello {
        onion_id: String,
        display_name: String,
        nonce: [u8; 16],
        #[serde(with = "sig64")]
        signature: [u8; 64],
    },
    Welcome {
        onion_id: String,
        display_name: String,
        nonce: [u8; 16],
        #[serde(with = "sig64")]
        signature: [u8; 64],
    },
    Ping {
        timestamp_ms: u64,
    },
    Pong {
        timestamp_ms: u64,
    },
    Ack {
        message_id: [u8; 16],
    },
    GroupInvite {
        group_id: String,
        name: String,
        members: Vec<crate::groups::MemberEntry>,
        key: crate::groups::KeyBlob,
    },
    GroupMembers {
        group_id: String,
        members: Vec<crate::groups::MemberEntry>,
    },
    GroupMsg {
        envelope: crate::groups::GroupEnvelope,
    },
    GroupRotate {
        group_id: String,
        key: crate::groups::KeyBlob,
    },
    GroupRemove {
        group_id: String,
        member_id: String,
    },
    GroupLeave {
        group_id: String,
    },
    Relay {
        recipient_id: String,
        kind: u8,
        payload: Vec<u8>,
    },
    RelayPush {
        items: Vec<crate::groups::RelayItem>,
    },
    RelayAck {
        ids: Vec<[u8; 16]>,
    },
    SyncDm {
        last_ms: u64,
    },
    ProfileUpdate {
        display_name: String,
        bio: String,
        status: String,
        accent: String,
        avatar_b64: String,
    },
    FriendRequest {
        display_name: String,
    },
    Bye {
        reason: String,
    },
    App {
        payload: Vec<u8>,
    },
}

pub fn encode_frame(frame: &Frame) -> Option<Vec<u8>> {
    let body = postcard::to_allocvec(frame).ok()?;
    if body.len() > MAX_FRAME_SIZE {
        return None;
    }
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    Some(out)
}

pub fn decode_frame(bytes: &[u8]) -> Option<Frame> {
    if bytes.len() > MAX_FRAME_SIZE {
        return None;
    }
    postcard::from_bytes(bytes).ok()
}

pub fn prefix_body_len(prefix: [u8; 4]) -> usize {
    u32::from_be_bytes(prefix) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::groups::{KeyBlob, MemberEntry, RelayItem};

    #[test]
    fn roundtrip_all_variants() {
        let frames = vec![
            Frame::Hello {
                onion_id: "a".repeat(56),
                display_name: "alice".into(),
                nonce: [1; 16],
                signature: [2; 64],
            },
            Frame::Welcome {
                onion_id: "b".repeat(56),
                display_name: "bob ✨".into(),
                nonce: [3; 16],
                signature: [4; 64],
            },
            Frame::Ping { timestamp_ms: 123 },
            Frame::Pong { timestamp_ms: 123 },
            Frame::Ack { message_id: [9; 16] },
            Frame::GroupInvite {
                group_id: "01HZZZZZZZZZZZZZZZZZZZZZZZZ".into(),
                name: "la bande".into(),
                members: vec![MemberEntry {
                    onion_id: "a".repeat(56),
                    display_name: "alice".into(),
                }],
                key: KeyBlob {
                    nonce: [1; 12],
                    ciphertext: vec![4, 5],
                },
            },
            Frame::GroupMembers {
                group_id: "01HZZZZZZZZZZZZZZZZZZZZZZZZ".into(),
                members: vec![],
            },
            Frame::GroupMsg {
                envelope: crate::groups::GroupEnvelope {
                    message_id: [2; 16],
                    group_id: "01HZZZZZZZZZZZZZZZZZZZZZZZZ".into(),
                    author_id: "a".repeat(56),
                    timestamp_ms: 99,
                    kind: 1,
                    duration_ms: 2500,
                    nonce: [3; 12],
                    ciphertext: vec![9, 8, 7],
                },
            },
            Frame::GroupRotate {
                group_id: "01HZZZZZZZZZZZZZZZZZZZZZZZZ".into(),
                key: KeyBlob {
                    nonce: [2; 12],
                    ciphertext: vec![1],
                },
            },
            Frame::GroupRemove {
                group_id: "01HZZZZZZZZZZZZZZZZZZZZZZZZ".into(),
                member_id: "b".repeat(56),
            },
            Frame::GroupLeave {
                group_id: "01HZZZZZZZZZZZZZZZZZZZZZZZZ".into(),
            },
            Frame::Relay {
                recipient_id: "b".repeat(56),
                kind: 1,
                payload: vec![7, 7, 7],
            },
            Frame::RelayPush {
                items: vec![RelayItem {
                    id: [5; 16],
                    kind: 2,
                    payload: vec![1],
                }],
            },
            Frame::RelayAck { ids: vec![[6; 16]] },
            Frame::SyncDm { last_ms: 123456 },
            Frame::ProfileUpdate {
                display_name: "nova".into(),
                bio: "adrift".into(),
                status: "orbiting".into(),
                accent: "#f5f5f5".into(),
                avatar_b64: "aGVsbG8=".into(),
            },
            Frame::FriendRequest {
                display_name: "echo".into(),
            },
            Frame::Bye { reason: "à+".into() },
            Frame::App { payload: vec![0, 1, 2, 255] },
        ];
        for frame in frames {
            let bytes = encode_frame(&frame).unwrap();
            let len = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
            assert_eq!(4 + len, bytes.len());
            let decoded = decode_frame(&bytes[4..]).unwrap();
            assert_eq!(decoded, frame);
        }
    }

    #[test]
    fn prefix_is_big_endian_body_len() {
        let bytes = encode_frame(&Frame::Ping { timestamp_ms: 1 }).unwrap();
        let len = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
        assert_eq!(len, bytes.len() - 4);
        assert_eq!(prefix_body_len(bytes[..4].try_into().unwrap()), len);
    }

    #[test]
    fn decode_garbage_is_none() {
        assert!(decode_frame(&[255, 255, 255]).is_none());
    }
}
