use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use void_proto::{Frame, GroupEnvelope, KeyBlob, MemberEntry};

use crate::engine::{CoreEvent, DmMessage, DmStatus, EngineInner};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct GroupState {
    pub group_id: String,
    pub name: String,
    pub key_b64: String,
    pub owner_id: String,
    pub members: Vec<MemberEntry>,
    pub created_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMemberInfo {
    pub onion_id: String,
    pub display_name: String,
    pub online: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupInfo {
    pub group_id: String,
    pub name: String,
    pub owner_id: String,
    pub members: Vec<GroupMemberInfo>,
    #[serde(rename = "createdAt")]
    pub created_ms: u64,
}

pub(crate) fn load_groups(path: &std::path::Path) -> Vec<GroupState> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub(crate) fn save_groups(path: &std::path::Path, groups: &[GroupState]) -> Result<()> {
    let json = serde_json::to_string_pretty(groups)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub(crate) fn to_info(inner: &Arc<EngineInner>, state: &GroupState) -> GroupInfo {
    let sessions = inner.sessions.lock().unwrap();
    GroupInfo {
        group_id: state.group_id.clone(),
        name: state.name.clone(),
        owner_id: state.owner_id.clone(),
        members: state
            .members
            .iter()
            .map(|member| GroupMemberInfo {
                onion_id: member.onion_id.clone(),
                display_name: member.display_name.clone(),
                online: sessions.contains_key(&member.onion_id),
            })
            .collect(),
        created_ms: state.created_ms,
    }
}

fn emit(inner: &Arc<EngineInner>, event: CoreEvent) {
    let _ = inner.events.send(event);
}

pub(crate) fn fanout(
    inner: &Arc<EngineInner>,
    members: &[MemberEntry],
    frame: &Frame,
) -> usize {
    let my_id = inner.identity.lock().unwrap().onion_id();
    let sessions = inner.sessions.lock().unwrap();
    let mut count = 0;
    for member in members {
        if member.onion_id == my_id {
            continue;
        }
        if let Some(session) = sessions.get(&member.onion_id) {
            if session.tx.try_send(frame.clone()).is_ok() {
                count += 1;
            }
        }
    }
    count
}

pub(crate) fn generate_key() -> [u8; 32] {
    use rand_core::RngCore;
    let mut key = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut key);
    key
}

pub(crate) fn encrypt_key_for(
    inner: &Arc<EngineInner>,
    recipient_id: &str,
    group_id: &str,
    key: &[u8; 32],
) -> Result<KeyBlob> {
    let identity = inner.identity.lock().unwrap().clone();
    let conversation =
        void_crypto::dm::conversation_key(&identity, recipient_id)?;
    let aad = void_proto::group_key_aad(group_id, recipient_id);
    let (nonce, ciphertext) =
        void_crypto::dm::dm_encrypt(&conversation, &aad, key)
            .ok_or_else(|| anyhow!("chiffrement de clé impossible"))?;
    Ok(KeyBlob {
        nonce,
        ciphertext,
    })
}

pub(crate) fn decrypt_key_from(
    inner: &Arc<EngineInner>,
    sender_id: &str,
    group_id: &str,
    blob: &KeyBlob,
) -> Result<[u8; 32]> {
    let identity = inner.identity.lock().unwrap().clone();
    let conversation =
        void_crypto::dm::conversation_key(&identity, sender_id)?;
    let aad = void_proto::group_key_aad(group_id, &identity.onion_id());
    let key = void_crypto::dm::dm_decrypt(
        &conversation,
        &blob.nonce,
        &aad,
        &blob.ciphertext,
    )
    .ok_or_else(|| anyhow!("déchiffrement de clé de groupe impossible"))?;
    key.try_into()
        .map_err(|_| anyhow!("taille de clé de groupe inattendue"))
}

pub(crate) fn upsert_group(
    inner: &Arc<EngineInner>,
    state: GroupState,
    is_new: bool,
) {
    let group_id = state.group_id.clone();
    let mut groups = inner.groups.lock().unwrap();
    match groups.iter().position(|g| g.group_id == group_id) {
        Some(index) => groups[index] = state,
        None => groups.push(state),
    }
    let _ = save_groups(&inner.data_dir.join("groups.json"), &groups);
    let info = to_info(
        inner,
        groups.iter().find(|g| g.group_id == group_id).unwrap(),
    );
    drop(groups);
    emit(
        inner,
        if is_new {
            CoreEvent::GroupNew { group: info }
        } else {
            CoreEvent::GroupUpdated { group: info }
        },
    );
}

pub(crate) fn handle_invite(
    inner: &Arc<EngineInner>,
    sender_id: &str,
    group_id: String,
    name: String,
    members: Vec<MemberEntry>,
    key: KeyBlob,
) {
    let my_id = inner.identity.lock().unwrap().onion_id();
    if !members.iter().any(|m| m.onion_id == my_id) {
        warn!("invitation au groupe {group_id} sans nous en tant que membre");
        return;
    }
    let group_key = match decrypt_key_from(inner, sender_id, &group_id, &key) {
        Ok(group_key) => group_key,
        Err(e) => {
            warn!("invitation au groupe {group_id} illisible: {e}");
            return;
        }
    };
    let existed = inner
        .groups
        .lock()
        .unwrap()
        .iter()
        .any(|g| g.group_id == group_id);
    let state = GroupState {
        group_id,
        name,
        key_b64: {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(group_key)
        },
        owner_id: sender_id.to_string(),
        members,
        created_ms: now_ms(),
    };
    info!("groupe rejoint: {}", state.name);
    upsert_group(inner, state, !existed);
}

pub(crate) fn handle_members(
    inner: &Arc<EngineInner>,
    sender_id: &str,
    group_id: String,
    members: Vec<MemberEntry>,
) {
    let mut groups = inner.groups.lock().unwrap();
    let Some(index) = groups.iter().position(|g| g.group_id == group_id) else {
        return;
    };
    let my_id = inner.identity.lock().unwrap().onion_id();
    if !groups[index].members.iter().any(|m| m.onion_id == sender_id) {
        warn!("membres du groupe {group_id} mis à jour par un non-membre");
        return;
    }
    if !members.iter().any(|m| m.onion_id == my_id) {
        warn!("mise à jour du groupe {group_id} nous excluant, ignorée");
        return;
    }
    groups[index].members = members;
    let _ = save_groups(&inner.data_dir.join("groups.json"), &groups);
    let info = to_info(inner, &groups[index]);
    drop(groups);
    emit(inner, CoreEvent::GroupUpdated { group: info });
}

pub(crate) fn handle_message(inner: &Arc<EngineInner>, envelope: GroupEnvelope) {
    let (state, index) = {
        let groups = inner.groups.lock().unwrap();
        let Some(index) = groups
            .iter()
            .position(|g| g.group_id == envelope.group_id)
        else {
            warn!("message pour un groupe inconnu: {}", envelope.group_id);
            return;
        };
        let state = groups[index].clone();
        (state, index)
    };
    if !state
        .members
        .iter()
        .any(|m| m.onion_id == envelope.author_id)
    {
        warn!(
            "message de groupe {} d'un non-membre",
            state.group_id
        );
        return;
    }
    use base64::Engine;
    let Ok(key) = base64::engine::general_purpose::STANDARD.decode(&state.key_b64)
    else {
        warn!("clé de groupe {} corrompue", state.group_id);
        return;
    };
    let Ok(key) = <[u8; 32]>::try_from(key.as_slice()) else {
        return;
    };
    let aad = void_proto::group_aad(&envelope);
    let Some(plaintext) =
        void_crypto::dm::dm_decrypt(&key, &envelope.nonce, &aad, &envelope.ciphertext)
    else {
        warn!("déchiffrement impossible dans le groupe {}", state.group_id);
        return;
    };
    if plaintext.len() > crate::engine::VOICE_MAX_BYTES.max(8000) {
        warn!("message anormalement volumineux dans {}", state.group_id);
        return;
    }

    let message = match envelope.kind {
        void_proto::KIND_VOICE => {
            if envelope.duration_ms == 0
                || envelope.duration_ms > crate::engine::VOICE_MAX_DURATION_MS
                || plaintext.is_empty()
            {
                warn!("note vocale invalide dans {}", state.group_id);
                return;
            }
            let message_id =
                ulid::Ulid::from_bytes(envelope.message_id).to_string();
            if crate::engine::write_blob(&inner.data_dir, &message_id, &plaintext).is_err() {
                warn!("écriture de la note vocale impossible");
                return;
            }
            DmMessage {
                message_id,
                peer_id: envelope.group_id.clone(),
                author_id: envelope.author_id.clone(),
                body: String::new(),
                created_ms: envelope.timestamp_ms,
                status: DmStatus::Delivered,
                kind: crate::engine::MessageKind::Voice,
                duration_ms: envelope.duration_ms,
            }
        }
        _ => {
            let Ok(body) = String::from_utf8(plaintext) else {
                return;
            };
            if body.is_empty() || body.chars().count() > 4000 {
                return;
            }
            DmMessage {
                message_id: ulid::Ulid::from_bytes(envelope.message_id).to_string(),
                peer_id: envelope.group_id.clone(),
                author_id: envelope.author_id.clone(),
                body,
                created_ms: envelope.timestamp_ms,
                status: DmStatus::Delivered,
                kind: crate::engine::MessageKind::Text,
                duration_ms: 0,
            }
        }
    };
    let inserted = inner
        .store
        .lock()
        .unwrap()
        .insert_message(&void_store::DmRecord {
            id: message.message_id.clone(),
            peer_id: message.peer_id.clone(),
            author_id: message.author_id.clone(),
            body: message.body.clone(),
            created_ms: message.created_ms,
            status: message.status.as_u8(),
            kind: message.kind.as_u8(),
            duration_ms: message.duration_ms,
        })
        .unwrap_or(false);
    if !inserted {
        debug!("message de groupe en double ignoré");
        return;
    }
    info!("message de groupe reçu dans {}", state.name);
    let _ = index;
    emit(inner, CoreEvent::GroupMessage { message });
}

pub(crate) fn handle_rotate(
    inner: &Arc<EngineInner>,
    sender_id: &str,
    group_id: String,
    key: KeyBlob,
) {
    let mut groups = inner.groups.lock().unwrap();
    let Some(index) = groups.iter().position(|g| g.group_id == group_id) else {
        return;
    };
    if groups[index].owner_id != sender_id {
        warn!("rotation de clé du groupe {group_id} par un non-propriétaire");
        return;
    }
    let Ok(group_key) = decrypt_key_from(inner, sender_id, &group_id, &key) else {
        warn!("rotation de clé du groupe {group_id} illisible");
        return;
    };
    use base64::Engine;
    groups[index].key_b64 =
        base64::engine::general_purpose::STANDARD.encode(group_key);
    let _ = save_groups(&inner.data_dir.join("groups.json"), &groups);
    let info = to_info(inner, &groups[index]);
    drop(groups);
    info!("clé du groupe {group_id} renouvelée");
    emit(inner, CoreEvent::GroupUpdated { group: info });
}

pub(crate) fn handle_remove(
    inner: &Arc<EngineInner>,
    sender_id: &str,
    group_id: String,
    member_id: String,
) {
    let mut groups = inner.groups.lock().unwrap();
    let Some(index) = groups.iter().position(|g| g.group_id == group_id) else {
        return;
    };
    if groups[index].owner_id != sender_id {
        warn!("exclusion dans le groupe {group_id} par un non-propriétaire");
        return;
    }
    let my_id = inner.identity.lock().unwrap().onion_id();
    if member_id == my_id {
        groups.remove(index);
        let _ = save_groups(&inner.data_dir.join("groups.json"), &groups);
        drop(groups);
        let _ = inner
            .store
            .lock()
            .unwrap()
            .delete_conversation(&group_id);
        warn!("exclu du groupe {group_id}");
        emit(inner, CoreEvent::GroupRemoved { group_id });
        return;
    }
    groups[index].members.retain(|m| m.onion_id != member_id);
    let _ = save_groups(&inner.data_dir.join("groups.json"), &groups);
    let info = to_info(inner, &groups[index]);
    drop(groups);
    emit(inner, CoreEvent::GroupUpdated { group: info });
}

pub(crate) fn handle_leave(inner: &Arc<EngineInner>, sender_id: &str, group_id: String) {
    let mut groups = inner.groups.lock().unwrap();
    let Some(index) = groups.iter().position(|g| g.group_id == group_id) else {
        return;
    };
    groups[index].members.retain(|m| m.onion_id != sender_id);
    let _ = save_groups(&inner.data_dir.join("groups.json"), &groups);
    let info = to_info(inner, &groups[index]);
    drop(groups);
    emit(inner, CoreEvent::GroupUpdated { group: info });
}

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(crate) fn validate_member_online(
    inner: &Arc<EngineInner>,
    onion_id: &str,
) -> Result<()> {
    let sessions = inner.sessions.lock().unwrap();
    if sessions.contains_key(onion_id) {
        Ok(())
    } else {
        bail!("ce pair est hors ligne")
    }
}
