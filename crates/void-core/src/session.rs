use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use void_proto::{
    dm_aad, dm_aad_parts, decode_dm, decode_frame, decode_group_envelope, encode_dm,
    encode_frame, encode_group_envelope, new_nonce, sign_handshake, verify_handshake,
    DmEnvelope, Frame, GroupEnvelope, RelayItem, MAX_FRAME_SIZE, HELLO_DOMAIN,
    WELCOME_DOMAIN, RELAY_KIND_DM, RELAY_KIND_GROUP,
};

use crate::engine::{CoreEvent, DmMessage, DmStatus, EngineInner, PeerInfo};

pub const P2P_VIRTUAL_PORT: u16 = 8477;

const DIAL_TIMEOUT: Duration = Duration::from_secs(120);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
const DIAL_INTERVAL: Duration = Duration::from_secs(45);
const PING_INTERVAL: Duration = Duration::from_secs(20);

pub const RELAY_MAX_ITEMS: u64 = 1000;
pub const RELAY_MAX_PER_SENDER: u64 = 200;
pub const RELAY_MAX_PAYLOAD: usize = 262_144;
pub const RELAY_TTL_MS: u64 = 7 * 24 * 3600 * 1000;
pub const SYNC_RESEND_LIMIT: u64 = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Outgoing,
    Incoming,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceInfo {
    pub onion_id: String,
    pub display_name: String,
    pub online: bool,
    pub direction: Option<Direction>,
    pub connected_since: Option<u64>,
    pub rtt_ms: Option<u64>,
}

pub(crate) struct PeerSession {
    pub(crate) conn_id: u64,
    pub(crate) initiator_id: String,
    pub(crate) direction: Direction,
    pub(crate) display_name: String,
    pub(crate) connected_since: u64,
    pub(crate) rtt_ms: Option<u64>,
    pub(crate) tx: mpsc::Sender<Frame>,
}

pub(crate) async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    frame: &Frame,
) -> bool {
    match encode_frame(frame) {
        Some(bytes) => writer.write_all(&bytes).await.is_ok(),
        None => false,
    }
}

pub(crate) async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Option<Frame> {
    let mut prefix = [0u8; 4];
    reader.read_exact(&mut prefix).await.ok()?;
    let len = u32::from_be_bytes(prefix) as usize;
    if len > MAX_FRAME_SIZE {
        return None;
    }
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await.ok()?;
    decode_frame(&body)
}

pub(crate) async fn read_frame_lenient<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Option<Frame> {
    loop {
        let mut prefix = [0u8; 4];
        reader.read_exact(&mut prefix).await.ok()?;
        let len = u32::from_be_bytes(prefix) as usize;
        if len > MAX_FRAME_SIZE {
            return None;
        }
        let mut body = vec![0u8; len];
        reader.read_exact(&mut body).await.ok()?;
        match decode_frame(&body) {
            Some(frame) => return Some(frame),
            None => debug!("frame inconnue ignorée ({len} octets)"),
        }
    }
}

pub(crate) async fn accept_loop(inner: Arc<EngineInner>, listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((socket, _peer)) => {
                let inner = Arc::clone(&inner);
                tokio::spawn(async move {
                    handle_incoming(inner, socket).await;
                });
            }
            Err(e) => warn!("accept: {e}"),
        }
    }
}

async fn handle_incoming(inner: Arc<EngineInner>, socket: TcpStream) {
    let mut socket = socket;
    let outcome = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        let Some(Frame::Hello {
            onion_id,
            display_name,
            nonce,
            signature,
        }) = read_frame(&mut socket).await
        else {
            return;
        };
        let own_id = inner.identity.lock().unwrap().onion_id();
        if onion_id == own_id {
            warn!("connexion entrante: boucle sur soi-même ignorée");
            return;
        }
        if !verify_handshake(HELLO_DOMAIN, &onion_id, &nonce, &signature) {
            warn!("handshake invalide de {onion_id}");
            return;
        }
        let display_name = sanitize_name(&display_name);
        let (welcome, initiator) = {
            let identity = inner.identity.lock().unwrap().clone();
            let nonce = new_nonce();
            let signature = sign_handshake(&identity, WELCOME_DOMAIN, &nonce);
            let frame = Frame::Welcome {
                onion_id: identity.onion_id(),
                display_name: inner.profile.lock().unwrap().display_name.clone(),
                nonce,
                signature,
            };
            (frame, onion_id.clone())
        };
        if !write_frame(&mut socket, &welcome).await {
            return;
        }
        register_session(
            inner,
            socket,
            onion_id,
            display_name,
            Direction::Incoming,
            initiator,
        )
        .await;
    })
    .await;
    if outcome.is_err() {
        debug!("handshake entrant expiré");
    }
}

pub(crate) async fn dial_peer(inner: Arc<EngineInner>, socks: SocketAddr, contact: PeerInfo) {
    let own_id = inner.identity.lock().unwrap().onion_id();
    if contact.onion_id == own_id {
        return;
    }
    let target = format!("{}.onion:{P2P_VIRTUAL_PORT}", contact.onion_id);
    let connected = tokio::time::timeout(DIAL_TIMEOUT, async {
        tokio_socks::tcp::Socks5Stream::connect(socks, target.as_str()).await
    })
    .await;
    let Ok(Ok(stream)) = connected else {
        debug!("dial échoué vers {}", contact.onion_id);
        return;
    };
    let mut socket = stream.into_inner();
    socket.set_nodelay(true).ok();

    let hello = {
        let identity = inner.identity.lock().unwrap().clone();
        let nonce = new_nonce();
        let signature = sign_handshake(&identity, HELLO_DOMAIN, &nonce);
        Frame::Hello {
            onion_id: identity.onion_id(),
            display_name: inner.profile.lock().unwrap().display_name.clone(),
            nonce,
            signature,
        }
    };

    let outcome = tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        if !write_frame(&mut socket, &hello).await {
            return;
        }
        let Some(Frame::Welcome {
            onion_id,
            display_name,
            nonce,
            signature,
        }) = read_frame(&mut socket).await
        else {
            return;
        };
        if onion_id != contact.onion_id {
            warn!("welcome mismatch: attendu {} reçu {onion_id}", contact.onion_id);
            return;
        }
        if !verify_handshake(WELCOME_DOMAIN, &onion_id, &nonce, &signature) {
            warn!("welcome invalide de {onion_id}");
            return;
        }
        let own_initiator = inner.identity.lock().unwrap().onion_id();
        register_session(
            inner,
            socket,
            onion_id,
            sanitize_name(&display_name),
            Direction::Outgoing,
            own_initiator,
        )
        .await;
    })
    .await;
    if outcome.is_err() {
        debug!("handshake sortant expiré vers {}", contact.onion_id);
    }
}

async fn register_session(
    inner: Arc<EngineInner>,
    socket: TcpStream,
    peer_onion_id: String,
    display_name: String,
    direction: Direction,
    initiator_id: String,
) {
    let conn_id = inner.conn_counter.fetch_add(1, Ordering::SeqCst) + 1;
    let (tx, rx) = mpsc::channel::<Frame>(64);

    {
        let mut sessions = inner.sessions.lock().unwrap();
        if let Some(existing) = sessions.get(&peer_onion_id) {
            if existing.initiator_id <= initiator_id {
                debug!("connexion dupliquée ignorée pour {peer_onion_id}");
                return;
            }
            let old = sessions.remove(&peer_onion_id).unwrap();
            let _ = old.tx.try_send(Frame::Bye {
                reason: "superseded".into(),
            });
        }
        sessions.insert(
            peer_onion_id.clone(),
            PeerSession {
                conn_id,
                initiator_id,
                direction,
                display_name,
                connected_since: unix_now(),
                rtt_ms: None,
                tx: tx.clone(),
            },
        );
    }
    info!("session établie avec {peer_onion_id} ({direction:?}, conn {conn_id})");
    refresh_presence(&inner);

    let (read_half, write_half) = socket.into_split();
    let writer_peer = peer_onion_id.clone();
    tokio::spawn(async move {
        writer_task(write_half, rx).await;
        debug!("writer fermé pour {writer_peer}");
    });
    let established_inner = Arc::clone(&inner);
    let established_peer = peer_onion_id.clone();
    tokio::spawn(async move {
        session_established(established_inner, established_peer).await;
    });
    tokio::spawn(async move {
        reader_task(inner, conn_id, peer_onion_id, read_half).await;
    });
}

async fn writer_task<W: AsyncWrite + Unpin>(mut writer: W, mut rx: mpsc::Receiver<Frame>) {
    while let Some(frame) = rx.recv().await {
        let bytes = match encode_frame(&frame) {
            Some(bytes) => bytes,
            None => break,
        };
        if writer.write_all(&bytes).await.is_err() {
            break;
        }
    }
}

async fn reader_task<R: AsyncRead + Unpin>(
    inner: Arc<EngineInner>,
    conn_id: u64,
    peer_onion_id: String,
    mut reader: R,
) {
    loop {
        let Some(frame) = read_frame_lenient(&mut reader).await else {
            break;
        };
        let session_alive = {
            let sessions = inner.sessions.lock().unwrap();
            sessions
                .get(&peer_onion_id)
                .map(|s| s.conn_id == conn_id)
                .unwrap_or(false)
        };
        if !session_alive {
            break;
        }
        match frame {
            Frame::Ping { timestamp_ms } => {
                let tx = {
                    let sessions = inner.sessions.lock().unwrap();
                    sessions.get(&peer_onion_id).map(|s| s.tx.clone())
                };
                if let Some(tx) = tx {
                    let _ = tx.try_send(Frame::Pong { timestamp_ms });
                }
            }
            Frame::Pong { timestamp_ms } => {
                let now = now_ms();
                let mut sessions = inner.sessions.lock().unwrap();
                if let Some(session) = sessions.get_mut(&peer_onion_id) {
                    if session.conn_id == conn_id {
                        session.rtt_ms = Some(now.saturating_sub(timestamp_ms));
                    }
                }
                refresh_presence_locked(&inner, &mut sessions);
            }
            Frame::Ack { message_id } => {
                let message_id = ulid::Ulid::from_bytes(message_id).to_string();
                let updated = inner
                    .store
                    .lock()
                    .unwrap()
                    .set_status(&message_id, DmStatus::Delivered.as_u8())
                    .unwrap_or(false);
                if updated {
                    let _ = inner.events.send(CoreEvent::DmStatus {
                        peer_id: peer_onion_id.clone(),
                        message_id,
                        status: DmStatus::Delivered,
                    });
                }
            }
            Frame::Bye { reason } => {
                debug!("bye reçu de {peer_onion_id}: {reason}");
                break;
            }
            Frame::GroupInvite { group_id, name, members, key } => {
                crate::groups::handle_invite(&inner, &peer_onion_id, group_id, name, members, key);
            }
            Frame::GroupMembers { group_id, members } => {
                crate::groups::handle_members(&inner, &peer_onion_id, group_id, members);
            }
            Frame::GroupMsg { envelope } => {
                crate::groups::handle_message(&inner, envelope);
            }
            Frame::GroupRotate { group_id, key } => {
                crate::groups::handle_rotate(&inner, &peer_onion_id, group_id, key);
            }
            Frame::GroupRemove { group_id, member_id } => {
                crate::groups::handle_remove(&inner, &peer_onion_id, group_id, member_id);
            }
            Frame::GroupLeave { group_id } => {
                crate::groups::handle_leave(&inner, &peer_onion_id, group_id);
            }
            Frame::App { payload } => {
                if let Some(envelope) = decode_dm(&payload) {
                    process_dm(&inner, Some(&peer_onion_id), envelope);
                } else {
                    warn!("envelope DM illisible de {peer_onion_id}");
                }
            }
            Frame::Relay { recipient_id, kind, payload } => {
                handle_relay_offer(&inner, &peer_onion_id, recipient_id, kind, payload);
            }
            Frame::RelayPush { items } => {
                let mut ack_ids = Vec::new();
                for item in items {
                    process_relay_item(&inner, &item);
                    ack_ids.push(item.id);
                }
                if !ack_ids.is_empty() {
                    let tx = {
                        let sessions = inner.sessions.lock().unwrap();
                        sessions.get(&peer_onion_id).map(|s| s.tx.clone())
                    };
                    if let Some(tx) = tx {
                        let _ = tx.try_send(Frame::RelayAck { ids: ack_ids });
                    }
                }
            }
            Frame::RelayAck { ids } => {
                let purged = inner.store.lock().unwrap().relay_delete_ids(&ids);
                if let Ok(purged) = purged {
                    debug!("{purged} enveloppes relais purgées (ack de {peer_onion_id})");
                }
            }
            Frame::SyncDm { last_ms } => {
                handle_sync_dm(&inner, &peer_onion_id, last_ms);
            }
            Frame::ProfileUpdate { display_name, bio, status, accent, avatar_b64 } => {
                let display_name = sanitize_name(&display_name);
                let my_id = inner.identity.lock().unwrap().onion_id();
                if peer_onion_id == my_id || display_name.is_empty() {
                    debug!("profile update invalide de {peer_onion_id}");
                } else {
                    let bio = sanitize_name(&bio);
                    let status = sanitize_name(&status);
                    let accent = sanitize_name(&accent);
                    {
                        let mut sessions = inner.sessions.lock().unwrap();
                        if let Some(session) = sessions.get_mut(&peer_onion_id) {
                            if session.conn_id == conn_id {
                                session.display_name = display_name.clone();
                            }
                        }
                        refresh_presence_locked(&inner, &mut sessions);
                    }
                    {
                        let mut contacts = inner.contacts.lock().unwrap();
                        let mut changed = false;
                        if let Some(contact) = contacts
                            .iter_mut()
                            .find(|c| c.onion_id == peer_onion_id)
                        {
                            contact.display_name = display_name.clone();
                            changed = true;
                        }
                        if changed {
                            let _ = crate::engine::persist_json(
                                &inner.data_dir.join("contacts.json"),
                                &*contacts,
                            );
                        }
                    }
                    {
                        let mut groups = inner.groups.lock().unwrap();
                        let mut changed = false;
                        for group in groups.iter_mut() {
                            if let Some(member) = group
                                .members
                                .iter_mut()
                                .find(|m| m.onion_id == peer_onion_id)
                            {
                                member.display_name = display_name.clone();
                                changed = true;
                            }
                        }
                        if changed {
                            let _ = crate::groups::save_groups(
                                &inner.data_dir.join("groups.json"),
                                &groups,
                            );
                        }
                    }
                    {
                        use base64::Engine;
                        let mut has_avatar = false;
                        if !avatar_b64.is_empty() && avatar_b64.len() < 120_000 {
                            if let Ok(bytes) =
                                base64::engine::general_purpose::STANDARD.decode(&avatar_b64)
                            {
                                if !bytes.is_empty()
                                    && bytes.len() <= crate::engine::Engine::AVATAR_MAX_BYTES
                                {
                                    let dir = inner.data_dir.join("avatars");
                                    let _ = std::fs::create_dir_all(&dir);
                                    let path = dir.join(format!("{peer_onion_id}.png"));
                                    if std::fs::write(&path, &bytes).is_ok() {
                                        has_avatar = true;
                                    }
                                }
                            }
                        } else if avatar_b64.is_empty() {
                            let path = inner
                                .data_dir
                                .join("avatars")
                                .join(format!("{peer_onion_id}.png"));
                            let _ = std::fs::remove_file(path);
                            has_avatar = false;
                        }
                        let mut profiles = inner.peer_profiles.lock().unwrap();
                        let entry = profiles.entry(peer_onion_id.clone()).or_default();
                        entry.bio = bio;
                        entry.status = status;
                        entry.accent = accent;
                        if has_avatar || avatar_b64.is_empty() {
                            entry.has_avatar = has_avatar;
                        }
                        let _ = crate::engine::persist_json(
                            &inner.data_dir.join("peer_profiles.json"),
                            &*profiles,
                        );
                    }
                    refresh_presence(&inner);
                    let _ = inner.events.send(CoreEvent::ProfileUpdated {
                        peer_id: peer_onion_id.clone(),
                    });
                    info!("{peer_onion_id} s'appelle maintenant « {display_name} »");
                }
            }
            Frame::FriendRequest { display_name } => {
                let display_name = sanitize_name(&display_name);
                let my_id = inner.identity.lock().unwrap().onion_id();
                if peer_onion_id == my_id || display_name.is_empty() {
                    debug!("friend request invalide de {peer_onion_id}");
                } else {
                    let already_contact = inner
                        .contacts
                        .lock()
                        .unwrap()
                        .iter()
                        .any(|c| c.onion_id == peer_onion_id);
                    if already_contact {
                        debug!("friend request de {peer_onion_id} déjà contact, ignorée");
                    } else {
                        let mut requests = inner.requests.lock().unwrap();
                        match requests.iter_mut().find(|r| r.onion_id == peer_onion_id) {
                            Some(existing) => existing.display_name = display_name.clone(),
                            None => {
                                requests.push(crate::engine::PendingRequest {
                                    onion_id: peer_onion_id.clone(),
                                    display_name: display_name.clone(),
                                    received_at: now_ms(),
                                });
                                let _ = crate::engine::persist_json(
                                    &inner.data_dir.join("requests.json"),
                                    &*requests,
                                );
                            }
                        }
                        let _ = inner.events.send(CoreEvent::FriendRequest {
                            peer_id: peer_onion_id.clone(),
                            display_name: display_name.clone(),
                        });
                        info!("demande de pair reçue de « {display_name} »");
                    }
                }
            }
            Frame::Hello { .. } | Frame::Welcome { .. } => {
                debug!("frame de handshake inattendue de {peer_onion_id}");
                break;
            }
        }
    }
    let mut sessions = inner.sessions.lock().unwrap();
    let removed = sessions
        .get(&peer_onion_id)
        .map(|s| s.conn_id == conn_id)
        .unwrap_or(false);
    if removed {
        sessions.remove(&peer_onion_id);
        info!("session fermée avec {peer_onion_id}");
        refresh_presence_locked(&inner, &mut sessions);
    }
}

pub(crate) async fn dial_loop(inner: Arc<EngineInner>) {
    loop {
        let socks = inner.socks.lock().unwrap().clone();
        if let Some(socks) = socks {
            let own_id = inner.identity.lock().unwrap().onion_id();
            let contacts = inner.contacts.lock().unwrap().clone();
            let online: Vec<String> = {
                let sessions = inner.sessions.lock().unwrap();
                sessions.keys().cloned().collect()
            };
            for contact in contacts {
                if contact.onion_id == own_id || online.contains(&contact.onion_id) {
                    continue;
                }
                let inner = Arc::clone(&inner);
                tokio::spawn(async move {
                    dial_peer(inner, socks, contact).await;
                });
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(DIAL_INTERVAL) => {}
            _ = inner.dial_notify.notified() => {}
        }
    }
}

pub(crate) async fn ping_loop(inner: Arc<EngineInner>) {
    loop {
        tokio::time::sleep(PING_INTERVAL).await;
        let sessions = inner.sessions.lock().unwrap();
        for (_, session) in sessions.iter() {
            let _ = session.tx.try_send(Frame::Ping {
                timestamp_ms: now_ms(),
            });
        }
    }
}

pub(crate) fn close_all_sessions(inner: &Arc<EngineInner>) {
    let mut sessions = inner.sessions.lock().unwrap();
    for (_, session) in sessions.iter() {
        let _ = session.tx.try_send(Frame::Bye {
            reason: "shutdown".into(),
        });
    }
    sessions.clear();
    refresh_presence_locked(inner, &mut sessions);
}

pub(crate) fn close_session(inner: &Arc<EngineInner>, peer_onion_id: &str) {
    let mut sessions = inner.sessions.lock().unwrap();
    if let Some(session) = sessions.remove(peer_onion_id) {
        let _ = session.tx.try_send(Frame::Bye {
            reason: "removed".into(),
        });
    }
    refresh_presence_locked(inner, &mut sessions);
}

pub(crate) fn compute_presence(inner: &Arc<EngineInner>) -> Vec<PresenceInfo> {
    let sessions = inner.sessions.lock().unwrap();
    let contacts = inner.contacts.lock().unwrap();
    presence_from(&sessions, &contacts)
}

fn presence_from(
    sessions: &HashMap<String, PeerSession>,
    contacts: &[PeerInfo],
) -> Vec<PresenceInfo> {
    let mut list: Vec<PresenceInfo> = Vec::new();

    for contact in contacts.iter() {
        let session = sessions.get(&contact.onion_id);
        let display_name = session
            .map(|s| s.display_name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| fallback_name(&contact.display_name, &contact.onion_id));
        list.push(PresenceInfo {
            onion_id: contact.onion_id.clone(),
            display_name,
            online: session.is_some(),
            direction: session.map(|s| s.direction),
            connected_since: session.map(|s| s.connected_since),
            rtt_ms: session.and_then(|s| s.rtt_ms),
        });
    }

    for (onion_id, session) in sessions.iter() {
        if contacts.iter().any(|c| c.onion_id == *onion_id) {
            continue;
        }
        list.push(PresenceInfo {
            onion_id: onion_id.clone(),
            display_name: fallback_name(&session.display_name, onion_id),
            online: true,
            direction: Some(session.direction),
            connected_since: Some(session.connected_since),
            rtt_ms: session.rtt_ms,
        });
    }

    list.sort_by(|a, b| b.online.cmp(&a.online).then(a.display_name.cmp(&b.display_name)));
    list
}

pub(crate) fn refresh_presence(inner: &Arc<EngineInner>) {
    let mut sessions = inner.sessions.lock().unwrap();
    refresh_presence_locked(inner, &mut sessions);
}

fn refresh_presence_locked(
    inner: &Arc<EngineInner>,
    sessions: &mut HashMap<String, PeerSession>,
) {
    let contacts = inner.contacts.lock().unwrap();
    let presence = presence_from(sessions, &contacts);
    let _ = inner.presence_tx.send_replace(presence);
}

fn process_dm(inner: &Arc<EngineInner>, via_peer: Option<&str>, envelope: DmEnvelope) {
    let own_id = inner.identity.lock().unwrap().onion_id();
    if envelope.recipient_id != own_id {
        warn!(
            "envelope DM destiné à {} alors que nous sommes {own_id}",
            envelope.recipient_id
        );
        return;
    }
    let author_id = envelope.author_id.clone();
    let identity = inner.identity.lock().unwrap().clone();
    let key = match void_crypto::dm::conversation_key(&identity, &author_id) {
        Ok(key) => key,
        Err(e) => {
            warn!("dérivation de clé impossible avec {author_id}: {e}");
            return;
        }
    };
    let aad = dm_aad(&envelope);
    let Some(plaintext) = void_crypto::dm::dm_decrypt(&key, &envelope.nonce, &aad, &envelope.ciphertext)
    else {
        warn!("déchiffrement impossible d'un message de {author_id}");
        return;
    };
    if plaintext.len() > crate::engine::VOICE_MAX_BYTES.max(8000) {
        warn!("message anormalement volumineux de {author_id}");
        return;
    }

    let message_id = ulid::Ulid::from_bytes(envelope.message_id).to_string();
    let message = match envelope.kind {
        void_proto::KIND_VOICE => {
            if envelope.duration_ms == 0
                || envelope.duration_ms > crate::engine::VOICE_MAX_DURATION_MS
                || plaintext.is_empty()
            {
                warn!("note vocale invalide de {author_id}");
                return;
            }
            if crate::engine::write_blob(&inner.data_dir, &message_id, &plaintext).is_err() {
                warn!("écriture de la note vocale impossible");
                return;
            }
            DmMessage {
                message_id: message_id.clone(),
                peer_id: author_id.clone(),
                author_id: author_id.clone(),
                body: String::new(),
                created_ms: envelope.timestamp_ms,
                status: DmStatus::Delivered,
                kind: crate::engine::MessageKind::Voice,
                duration_ms: envelope.duration_ms,
            }
        }
        _ => {
            let Ok(body) = String::from_utf8(plaintext) else {
                warn!("message non-utf8 de {author_id}");
                return;
            };
            if body.is_empty() || body.chars().count() > 4000 {
                warn!("message invalide de {author_id}");
                return;
            }
            DmMessage {
                message_id: message_id.clone(),
                peer_id: author_id.clone(),
                author_id: author_id.clone(),
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

    if via_peer == Some(author_id.as_str()) {
        let ack_tx = {
            let sessions = inner.sessions.lock().unwrap();
            sessions.get(&author_id).map(|s| s.tx.clone())
        };
        if let Some(tx) = ack_tx {
            let _ = tx.try_send(Frame::Ack {
                message_id: envelope.message_id,
            });
        }
    }
    if inserted {
        let _ = inner.events.send(CoreEvent::DmNew { message });
        info!("message reçu de {author_id}{}", if via_peer.is_some() { "" } else { " (via relais)" });
    } else {
        debug!("message en double de {author_id} ignoré");
    }
}

fn handle_relay_offer(
    inner: &Arc<EngineInner>,
    via_peer: &str,
    recipient_id: String,
    kind: u8,
    payload: Vec<u8>,
) {
    let my_id = inner.identity.lock().unwrap().onion_id();
    if recipient_id == my_id {
        process_relay_item(
            inner,
            &RelayItem {
                id: [0; 16],
                kind,
                payload,
            },
        );
        return;
    }
    if payload.len() > RELAY_MAX_PAYLOAD {
        warn!("offre relais trop volumineuse ({})", payload.len());
        return;
    }
    let mut store = inner.store.lock().unwrap();
    let total = store.relay_count().unwrap_or(0);
    if total >= RELAY_MAX_ITEMS {
        warn!("file relais pleine, offre de {via_peer} refusée");
        return;
    }
    let from_sender = store.relay_count_from(via_peer).unwrap_or(0);
    if from_sender >= RELAY_MAX_PER_SENDER {
        warn!("quota relais atteint pour {via_peer}");
        return;
    }
    let row = void_store::RelayRow {
        id: ulid::Ulid::new().to_bytes(),
        sender_id: via_peer.to_string(),
        recipient_id: recipient_id.clone(),
        kind,
        payload,
        stored_ms: now_ms(),
    };
    if store.relay_insert(&row).is_ok() {
        info!("enveloppe retenue pour {recipient_id} via {via_peer} (file: {})", total + 1);
    }
}

fn process_relay_item(inner: &Arc<EngineInner>, item: &RelayItem) {
    match item.kind {
        RELAY_KIND_DM => {
            if let Some(envelope) = decode_dm(&item.payload) {
                process_dm(inner, None, envelope);
            } else {
                warn!("enveloppe DM relais illisible");
            }
        }
        RELAY_KIND_GROUP => {
            if let Some(envelope) = decode_group_envelope(&item.payload) {
                crate::groups::handle_message(inner, envelope);
            } else {
                warn!("enveloppe groupe relais illisible");
            }
        }
        other => warn!("type relais inconnu: {other}"),
    }
}

fn handle_sync_dm(inner: &Arc<EngineInner>, peer_onion_id: &str, last_ms: u64) {
    let my_id = inner.identity.lock().unwrap().onion_id();
    let records = {
        let mut store = inner.store.lock().unwrap();
        store
            .messages_from_since(peer_onion_id, &my_id, last_ms, SYNC_RESEND_LIMIT)
            .unwrap_or_default()
    };
    if records.is_empty() {
        return;
    }
    let tx = {
        let sessions = inner.sessions.lock().unwrap();
        sessions.get(peer_onion_id).map(|s| s.tx.clone())
    };
    let Some(tx) = tx else { return };
    let mut resent = 0;
    for record in records {
        if let Some(envelope) = build_dm_envelope(inner, peer_onion_id, &record) {
            if tx
                .try_send(Frame::App {
                    payload: encode_dm(&envelope),
                })
                .is_ok()
            {
                resent += 1;
            }
        }
    }
    if resent > 0 {
        info!("{resent} message(s) resynchronisé(s) vers {peer_onion_id}");
    }
}

async fn session_established(inner: Arc<EngineInner>, peer_onion_id: String) {
    tokio::time::sleep(Duration::from_millis(400)).await;
    let push_items: Vec<RelayItem> = {
        let mut store = inner.store.lock().unwrap();
        let _ = store.relay_purge_expired(now_ms().saturating_sub(RELAY_TTL_MS));
        store
            .relay_for_recipient(&peer_onion_id)
            .unwrap_or_default()
            .into_iter()
            .map(|row| RelayItem {
                id: row.id,
                kind: row.kind,
                payload: row.payload,
            })
            .collect()
    };
    let last_ms = {
        let mut store = inner.store.lock().unwrap();
        store
            .max_created_ms(&peer_onion_id, &peer_onion_id)
            .unwrap_or(None)
            .unwrap_or(0)
    };
    {
        let sessions = inner.sessions.lock().unwrap();
        if let Some(session) = sessions.get(&peer_onion_id) {
            if !push_items.is_empty() {
                let _ = session.tx.try_send(Frame::RelayPush { items: push_items });
            }
            let _ = session.tx.try_send(Frame::SyncDm { last_ms });
            let in_contacts = inner
                .contacts
                .lock()
                .unwrap()
                .iter()
                .any(|c| c.onion_id == peer_onion_id);
            if in_contacts {
                let display_name = inner.profile.lock().unwrap().display_name.clone();
                let _ = session.tx.try_send(Frame::FriendRequest { display_name });
                let profile_frame = {
                    let profile = inner.profile.lock().unwrap();
                    Frame::ProfileUpdate {
                        display_name: profile.display_name.clone(),
                        bio: profile.bio.clone(),
                        status: profile.status.clone(),
                        accent: profile.accent.clone(),
                        avatar_b64: profile.avatar_b64.clone(),
                    }
                };
                let _ = session.tx.try_send(profile_frame);
            }
        }
    }
    flush_pending(&inner);
}

pub(crate) fn flush_pending(inner: &Arc<EngineInner>) {
    let my_id = inner.identity.lock().unwrap().onion_id();
    let queued = {
        let mut store = inner.store.lock().unwrap();
        store.queued_messages(&my_id).unwrap_or_default()
    };
    if queued.is_empty() {
        return;
    }
    let group_ids: Vec<String> = {
        let groups = inner.groups.lock().unwrap();
        groups.iter().map(|g| g.group_id.clone()).collect()
    };
    for record in queued {
        if group_ids.contains(&record.peer_id) {
            flush_group_record(inner, &record);
        } else {
            flush_dm_record(inner, &record);
        }
    }
}

fn flush_dm_record(inner: &Arc<EngineInner>, record: &void_store::DmRecord) {
    let Some(envelope) = build_dm_envelope(inner, &record.peer_id, record) else {
        return;
    };
    let payload = encode_dm(&envelope);
    let sessions = snapshot_sessions(inner);
    let direct = sessions
        .iter()
        .find(|(onion, _)| *onion == record.peer_id)
        .map(|(_, tx)| {
            tx.try_send(Frame::App {
                payload: payload.clone(),
            })
            .is_ok()
        })
        .unwrap_or(false);
    if direct {
        let _ = inner.store.lock().unwrap().set_status(&record.id, 1);
        return;
    }
    let mut relayed = 0;
    for (onion, tx) in sessions {
        if onion == record.peer_id {
            continue;
        }
        if tx.try_send(Frame::Relay {
            recipient_id: record.peer_id.clone(),
            kind: RELAY_KIND_DM,
            payload: payload.clone(),
        })
        .is_ok()
        {
            relayed += 1;
        }
    }
    if relayed > 0 {
        let _ = inner.store.lock().unwrap().set_status(&record.id, 1);
        info!("message en attente relayé vers {} via {relayed} pair(s)", record.peer_id);
    }
}

fn flush_group_record(inner: &Arc<EngineInner>, record: &void_store::DmRecord) {
    let Some(envelope) = build_group_envelope(inner, record) else {
        return;
    };
    let payload = encode_group_envelope(&envelope);
    let my_id = inner.identity.lock().unwrap().onion_id();
    let members: Vec<String> = {
        let groups = inner.groups.lock().unwrap();
        groups
            .iter()
            .find(|g| g.group_id == record.peer_id)
            .map(|g| g.members.iter().map(|m| m.onion_id.clone()).collect())
            .unwrap_or_default()
    };
    if members.is_empty() {
        let _ = inner.store.lock().unwrap().set_status(&record.id, 1);
        return;
    }
    let sessions = snapshot_sessions(inner);
    let mut any_direct = false;
    for onion in members.iter() {
        if onion == &my_id {
            continue;
        }
        if let Some((_, tx)) = sessions.iter().find(|(s, _)| s == onion) {
            if tx.try_send(Frame::GroupMsg {
                envelope: envelope.clone(),
            })
            .is_ok()
            {
                any_direct = true;
            }
        }
    }
    let mut any_relay = false;
    for onion in members.iter() {
        if onion == &my_id {
            continue;
        }
        if sessions.iter().any(|(s, _)| s == onion) {
            continue;
        }
        for (relay_onion, tx) in sessions.iter() {
            if relay_onion == onion {
                continue;
            }
            if tx.try_send(Frame::Relay {
                recipient_id: onion.clone(),
                kind: RELAY_KIND_GROUP,
                payload: payload.clone(),
            })
            .is_ok()
            {
                any_relay = true;
            }
        }
    }
    if any_direct || any_relay {
        let _ = inner.store.lock().unwrap().set_status(&record.id, 1);
        info!("message de groupe en attente renvoyé ({})", record.peer_id);
    }
}

pub(crate) fn build_dm_envelope(
    inner: &Arc<EngineInner>,
    peer_onion_id: &str,
    record: &void_store::DmRecord,
) -> Option<DmEnvelope> {
    let identity = inner.identity.lock().unwrap().clone();
    let my_id = identity.onion_id();
    let key = void_crypto::dm::conversation_key(&identity, peer_onion_id).ok()?;
    let message_id = ulid::Ulid::from_string(&record.id).ok()?;
    let plaintext: Vec<u8> = if record.kind == void_proto::KIND_VOICE {
        std::fs::read(crate::engine::blob_path(&inner.data_dir, &record.id)).ok()?
    } else {
        record.body.clone().into_bytes()
    };
    let aad = dm_aad_parts(&message_id.to_bytes(), &my_id, peer_onion_id, record.created_ms);
    let (nonce, ciphertext) =
        void_crypto::dm::dm_encrypt(&key, &aad, &plaintext)?;
    Some(DmEnvelope {
        message_id: message_id.to_bytes(),
        author_id: my_id,
        recipient_id: peer_onion_id.to_string(),
        timestamp_ms: record.created_ms,
        kind: record.kind,
        duration_ms: record.duration_ms,
        nonce,
        ciphertext,
    })
}

pub(crate) fn build_group_envelope(
    inner: &Arc<EngineInner>,
    record: &void_store::DmRecord,
) -> Option<GroupEnvelope> {
    let identity = inner.identity.lock().unwrap().clone();
    let my_id = identity.onion_id();
    let state = {
        let groups = inner.groups.lock().unwrap();
        groups
            .iter()
            .find(|g| g.group_id == record.peer_id)
            .cloned()?
    };
    use base64::Engine;
    let key = base64::engine::general_purpose::STANDARD
        .decode(&state.key_b64)
        .ok()?;
    let key: [u8; 32] = key.try_into().ok()?;
    let message_id = ulid::Ulid::from_string(&record.id).ok()?;
    let plaintext: Vec<u8> = if record.kind == void_proto::KIND_VOICE {
        std::fs::read(crate::engine::blob_path(&inner.data_dir, &record.id)).ok()?
    } else {
        record.body.clone().into_bytes()
    };
    let aad = void_proto::group_aad_parts(
        &message_id.to_bytes(),
        &record.peer_id,
        &my_id,
        record.created_ms,
    );
    let (nonce, ciphertext) =
        void_crypto::dm::dm_encrypt(&key, &aad, &plaintext)?;
    Some(GroupEnvelope {
        message_id: message_id.to_bytes(),
        group_id: record.peer_id.clone(),
        author_id: my_id,
        timestamp_ms: record.created_ms,
        kind: record.kind,
        duration_ms: record.duration_ms,
        nonce,
        ciphertext,
    })
}

pub(crate) fn snapshot_sessions(
    inner: &Arc<EngineInner>,
) -> Vec<(String, mpsc::Sender<Frame>)> {
    let sessions = inner.sessions.lock().unwrap();
    sessions
        .iter()
        .map(|(onion, session)| (onion.clone(), session.tx.clone()))
        .collect()
}

fn sanitize_name(name: &str) -> String {
    name.trim().chars().take(64).collect()
}

fn fallback_name(name: &str, onion_id: &str) -> String {
    if name.is_empty() {
        onion_id.chars().take(10).collect()
    } else {
        name.to_string()
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
