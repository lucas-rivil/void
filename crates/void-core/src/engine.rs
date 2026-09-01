use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch, Mutex as AsyncMutex, Notify};
use tracing::{error, info, warn};
use void_crypto::Identity;
use void_proto::Invite;
use void_tor::{launch, TorConfig, TorHandle};
use void_store::Store;

use crate::groups::{self, GroupInfo, GroupState};
use crate::session::{self, PresenceInfo};

#[derive(Clone, Debug, Default, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TorStatus {
    #[default]
    Starting,
    Bootstrapping { progress: u8 },
    Online { onion: String, socks: String },
    Failed { error: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityInfo {
    pub display_name: String,
    pub onion: String,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerInfo {
    #[serde(alias = "onion_id")]
    pub onion_id: String,
    #[serde(alias = "fingerprint")]
    pub fingerprint: String,
    #[serde(alias = "display_name")]
    pub display_name: String,
    #[serde(alias = "added_at")]
    pub added_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DmStatus {
    Queued,
    Sent,
    Delivered,
}

impl DmStatus {
    pub fn as_u8(self) -> u8 {
        match self {
            DmStatus::Queued => 0,
            DmStatus::Sent => 1,
            DmStatus::Delivered => 2,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            2 => DmStatus::Delivered,
            1 => DmStatus::Sent,
            _ => DmStatus::Queued,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    Text,
    Voice,
}

impl MessageKind {
    pub fn as_u8(self) -> u8 {
        match self {
            MessageKind::Text => 0,
            MessageKind::Voice => 1,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => MessageKind::Voice,
            _ => MessageKind::Text,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DmMessage {
    pub message_id: String,
    pub peer_id: String,
    pub author_id: String,
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_ms: u64,
    pub status: DmStatus,
    pub kind: MessageKind,
    pub duration_ms: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "type", rename_all_fields = "camelCase")]
pub enum CoreEvent {
    DmNew { message: DmMessage },
    DmStatus { peer_id: String, message_id: String, status: DmStatus },
    GroupNew { group: GroupInfo },
    GroupUpdated { group: GroupInfo },
    GroupRemoved { group_id: String },
    GroupMessage { message: DmMessage },
    FriendRequest { peer_id: String, display_name: String },
    FriendRequestHandled { peer_id: String },
    ProfileUpdated { peer_id: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Profile {
    pub(crate) display_name: String,
    pub(crate) bio: String,
    pub(crate) status: String,
    pub(crate) accent: String,
    pub(crate) avatar_b64: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            bio: String::new(),
            status: String::new(),
            accent: String::new(),
            avatar_b64: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub(crate) struct PeerProfile {
    pub bio: String,
    pub status: String,
    pub accent: String,
    pub has_avatar: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnProfileInfo {
    pub display_name: String,
    pub bio: String,
    pub status: String,
    pub accent: String,
    pub has_avatar: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerProfileInfo {
    pub onion_id: String,
    pub display_name: String,
    pub bio: String,
    pub status: String,
    pub accent: String,
    pub has_avatar: bool,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub notifications_enabled: bool,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            language: "en".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub relay_queue: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingRequest {
    pub onion_id: String,
    pub display_name: String,
    pub received_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct RecoveryState {
    confirmed: bool,
}

pub struct EngineConfig {
    pub data_dir: PathBuf,
    pub tor_dir: PathBuf,
}

#[derive(Clone)]
pub struct Engine {
    inner: Arc<EngineInner>,
}

pub(crate) struct EngineInner {
    pub(crate) identity: Mutex<Identity>,
    pub(crate) profile: Mutex<Profile>,
    pub(crate) recovery: Mutex<RecoveryState>,
    pub(crate) contacts: Mutex<Vec<PeerInfo>>,
    pub(crate) requests: Mutex<Vec<PendingRequest>>,
    pub(crate) peer_profiles: Mutex<std::collections::HashMap<String, PeerProfile>>,
    pub(crate) groups: Mutex<Vec<GroupState>>,
    pub(crate) settings: Mutex<Settings>,
    pub(crate) store: Mutex<Store>,
    pub(crate) events: broadcast::Sender<CoreEvent>,
    pub(crate) data_dir: PathBuf,
    pub(crate) tor_cfg: TorConfig,
    pub(crate) app_port: u16,
    pub(crate) socks: Mutex<Option<SocketAddr>>,
    pub(crate) tor: AsyncMutex<Option<TorHandle>>,
    pub(crate) sessions: Mutex<std::collections::HashMap<String, session::PeerSession>>,
    pub(crate) presence_tx: watch::Sender<Vec<PresenceInfo>>,
    pub(crate) dial_notify: Notify,
    pub(crate) conn_counter: AtomicU64,
    pub(crate) bootstrapping: AtomicBool,
    pub(crate) status_tx: watch::Sender<TorStatus>,
}

impl Engine {
    pub async fn start(cfg: EngineConfig) -> Result<Arc<Engine>> {
        std::fs::create_dir_all(&cfg.data_dir)
            .with_context(|| format!("création de {}", cfg.data_dir.display()))?;
        let tor_data_dir = cfg.data_dir.join("tor");
        std::fs::create_dir_all(&tor_data_dir).ok();

        let identity = load_or_create_identity(&cfg.data_dir)?;
        let profile = load_profile(&cfg.data_dir, &identity)?;
        let recovery = load_json_or_default(&cfg.data_dir.join("recovery.json"));
        let contacts: Vec<PeerInfo> = load_json_or_default(&cfg.data_dir.join("contacts.json"));
        let requests: Vec<PendingRequest> =
            load_json_or_default(&cfg.data_dir.join("requests.json"));
        let peer_profiles: std::collections::HashMap<String, PeerProfile> =
            load_json_or_default(&cfg.data_dir.join("peer_profiles.json"));
        let group_states = groups::load_groups(&cfg.data_dir.join("groups.json"));
        let settings: Settings = load_json_or_default(&cfg.data_dir.join("settings.json"));
        let store = Store::open(&cfg.data_dir.join("messages.db"))?;
        let (events, _events_rx) = broadcast::channel(64);
        info!("identité: {}", identity.onion_address());

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let app_port = listener.local_addr()?.port();
        info!("listener local 127.0.0.1:{app_port} prêt pour le trafic oignon");

        let (status_tx, _status_rx) = watch::channel(TorStatus::Starting);
        let (presence_tx, _presence_rx) = watch::channel(Vec::<PresenceInfo>::new());

        let inner = Arc::new(EngineInner {
            identity: Mutex::new(identity),
            profile: Mutex::new(profile),
            recovery: Mutex::new(recovery),
            contacts: Mutex::new(contacts),
            requests: Mutex::new(requests),
            peer_profiles: Mutex::new(peer_profiles),
            groups: Mutex::new(group_states),
            settings: Mutex::new(settings),
            store: Mutex::new(store),
            events,
            data_dir: cfg.data_dir,
            tor_cfg: TorConfig {
                tor_dir: cfg.tor_dir,
                data_dir: tor_data_dir,
            },
            app_port,
            socks: Mutex::new(None),
            tor: AsyncMutex::new(None),
            sessions: Mutex::new(std::collections::HashMap::new()),
            presence_tx,
            dial_notify: Notify::new(),
            conn_counter: AtomicU64::new(0),
            bootstrapping: AtomicBool::new(false),
            status_tx,
        });

        let bootstrap = Arc::clone(&inner);
        tokio::spawn(async move {
            run_bootstrap(bootstrap).await;
        });

        tokio::spawn(tor_supervisor(Arc::clone(&inner)));
        tokio::spawn(session::accept_loop(Arc::clone(&inner), listener));
        tokio::spawn(session::dial_loop(Arc::clone(&inner)));
        tokio::spawn(session::ping_loop(Arc::clone(&inner)));

        Ok(Arc::new(Engine { inner }))
    }

    pub fn subscribe(&self) -> watch::Receiver<TorStatus> {
        self.inner.status_tx.subscribe()
    }

    pub fn status(&self) -> TorStatus {
        self.inner.status_tx.borrow().clone()
    }

    pub fn subscribe_presence(&self) -> watch::Receiver<Vec<PresenceInfo>> {
        self.inner.presence_tx.subscribe()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<CoreEvent> {
        self.inner.events.subscribe()
    }

    pub fn send_dm(&self, peer_onion_id: &str, text: &str) -> Result<DmMessage> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > 4000 {
            bail!("message invalide (1 à 4000 caractères)");
        }
        let message = self.send_dm_payload(
            peer_onion_id,
            MessageKind::Text,
            0,
            text.as_bytes(),
        )?;
        Ok(message)
    }

    pub fn send_voice_dm(
        &self,
        peer_onion_id: &str,
        data: &[u8],
        duration_ms: u32,
    ) -> Result<DmMessage> {
        if data.is_empty() || data.len() > VOICE_MAX_BYTES {
            bail!("note vocale invalide (max 60 secondes)");
        }
        if duration_ms == 0 || duration_ms > VOICE_MAX_DURATION_MS {
            bail!("durée invalide");
        }
        let message = self.send_dm_payload(
            peer_onion_id,
            MessageKind::Voice,
            duration_ms,
            data,
        )?;
        write_blob(&self.inner.data_dir, &message.message_id, data)
            .context("écriture de la note vocale")?;
        Ok(message)
    }

    fn send_dm_payload(
        &self,
        peer_onion_id: &str,
        kind: MessageKind,
        duration_ms: u32,
        plaintext: &[u8],
    ) -> Result<DmMessage> {
        let identity = self.inner.identity.lock().unwrap().clone();
        let my_id = identity.onion_id();
        if peer_onion_id == my_id {
            bail!("impossible de s'écrire à soi-même");
        }
        if void_crypto::onion_id_to_public(peer_onion_id).is_none() {
            bail!("adresse oignon invalide");
        }
        let direct_tx = {
            let sessions = self.inner.sessions.lock().unwrap();
            sessions.get(peer_onion_id).map(|s| s.tx.clone())
        };

        let message_id = ulid::Ulid::new();
        let timestamp_ms = now_ms();
        let aad = void_proto::dm_aad_parts(
            &message_id.to_bytes(),
            &my_id,
            peer_onion_id,
            timestamp_ms,
        );
        let key = void_crypto::dm::conversation_key(&identity, peer_onion_id)?;
        let (nonce, ciphertext) =
            void_crypto::dm::dm_encrypt(&key, &aad, plaintext)
                .ok_or_else(|| anyhow!("chiffrement impossible"))?;

        let envelope = void_proto::DmEnvelope {
            message_id: message_id.to_bytes(),
            author_id: my_id.clone(),
            recipient_id: peer_onion_id.to_string(),
            timestamp_ms,
            kind: kind.as_u8(),
            duration_ms,
            nonce,
            ciphertext,
        };
        let payload = void_proto::encode_dm(&envelope);

        let status = if let Some(tx) = direct_tx {
            if tx
                .try_send(void_proto::Frame::App { payload })
                .is_ok()
            {
                DmStatus::Sent
            } else {
                DmStatus::Queued
            }
        } else {
            let sessions = session::snapshot_sessions(&self.inner);
            let mut relayed = 0;
            for (onion, tx) in sessions {
                if onion == peer_onion_id {
                    continue;
                }
                if tx.try_send(void_proto::Frame::Relay {
                    recipient_id: peer_onion_id.to_string(),
                    kind: void_proto::RELAY_KIND_DM,
                    payload: payload.clone(),
                })
                .is_ok()
                {
                    relayed += 1;
                }
            }
            if relayed > 0 {
                DmStatus::Sent
            } else {
                DmStatus::Queued
            }
        };

        let message = DmMessage {
            message_id: message_id.to_string(),
            peer_id: peer_onion_id.to_string(),
            author_id: my_id,
            body: if kind == MessageKind::Text {
                String::from_utf8_lossy(plaintext).into_owned()
            } else {
                String::new()
            },
            created_ms: timestamp_ms,
            status,
            kind,
            duration_ms,
        };
        self.inner.store.lock().unwrap().insert_message(&record_of(&message))?;
        Ok(message)
    }

    pub fn relay_queue_len(&self) -> u64 {
        self.inner.store.lock().unwrap().relay_count().unwrap_or(0)
    }

    pub fn settings(&self) -> Settings {
        self.inner.settings.lock().unwrap().clone()
    }

    pub fn set_settings(&self, settings: Settings) -> Result<()> {
        let json = serde_json::to_string_pretty(&settings)?;
        std::fs::write(self.inner.data_dir.join("settings.json"), json)?;
        *self.inner.settings.lock().unwrap() = settings;
        Ok(())
    }

    pub fn app_info(&self) -> AppInfo {
        AppInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            data_dir: self.inner.data_dir.display().to_string(),
            relay_queue: self.relay_queue_len(),
        }
    }

    pub fn dm_history(
        &self,
        peer_onion_id: &str,
        limit: u64,
        before_id: Option<&str>,
    ) -> Result<Vec<DmMessage>> {
        let records = self
            .inner
            .store
            .lock()
            .unwrap()
            .list_messages(peer_onion_id, limit, before_id)?;
        Ok(records.into_iter().map(message_of).collect())
    }

    pub fn groups(&self) -> Vec<GroupInfo> {
        let states = self.inner.groups.lock().unwrap().clone();
        states
            .iter()
            .map(|state| groups::to_info(&self.inner, state))
            .collect()
    }

    pub fn create_group(&self, name: &str, member_ids: &[String]) -> Result<GroupInfo> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 32 {
            bail!("nom de groupe invalide (1 à 32 caractères)");
        }
        let my_id = self.inner.identity.lock().unwrap().onion_id();
        let my_display = self.inner.profile.lock().unwrap().display_name.clone();
        let contacts = self.inner.contacts.lock().unwrap().clone();

        let mut members = vec![void_proto::MemberEntry {
            onion_id: my_id.clone(),
            display_name: my_display,
        }];
        for id in member_ids {
            if id == &my_id || members.iter().any(|m| m.onion_id == *id) {
                continue;
            }
            let contact = contacts
                .iter()
                .find(|c| c.onion_id == *id)
                .ok_or_else(|| anyhow!("pair inconnu: {id}"))?;
            groups::validate_member_online(&self.inner, id)?;
            members.push(void_proto::MemberEntry {
                onion_id: id.clone(),
                display_name: if contact.display_name.is_empty() {
                    id[..10].to_string()
                } else {
                    contact.display_name.clone()
                },
            });
        }

        let key = groups::generate_key();
        let state = GroupState {
            group_id: ulid::Ulid::new().to_string(),
            name: name.to_string(),
            key_b64: {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(key)
            },
            owner_id: my_id,
            members: members.clone(),
            created_ms: groups::now_ms(),
        };

        for member in members.iter().skip(1) {
            let blob = groups::encrypt_key_for(
                &self.inner,
                &member.onion_id,
                &state.group_id,
                &key,
            )?;
            let frame = void_proto::Frame::GroupInvite {
                group_id: state.group_id.clone(),
                name: state.name.clone(),
                members: state.members.clone(),
                key: blob,
            };
            let sent = {
                let sessions = self.inner.sessions.lock().unwrap();
                sessions
                    .get(&member.onion_id)
                    .map(|s| s.tx.try_send(frame).is_ok())
                    .unwrap_or(false)
            };
            if !sent {
                bail!("envoi de l'invitation impossible vers {}", member.display_name);
            }
        }

        let mut group_states = self.inner.groups.lock().unwrap();
        group_states.push(state);
        let _ = groups::save_groups(&self.inner.data_dir.join("groups.json"), &group_states);
        let info = groups::to_info(
            &self.inner,
            group_states.last().unwrap(),
        );
        drop(group_states);
        let _ = self.inner.events.send(CoreEvent::GroupNew { group: info.clone() });
        info!("groupe créé: {}", info.name);
        Ok(info)
    }

    pub fn add_group_member(&self, group_id: &str, member_id: &str) -> Result<GroupInfo> {
        let my_id = self.inner.identity.lock().unwrap().onion_id();
        {
            let group_states = self.inner.groups.lock().unwrap();
            let state = group_states
                .iter()
                .find(|g| g.group_id == group_id)
                .ok_or_else(|| anyhow!("groupe introuvable"))?;
            if state.members.iter().any(|m| m.onion_id == member_id) {
                bail!("déjà membre du groupe");
            }
        }
        let contact_display = {
            let contacts = self.inner.contacts.lock().unwrap();
            contacts
                .iter()
                .find(|c| c.onion_id == member_id)
                .map(|c| {
                    if c.display_name.is_empty() {
                        member_id[..10].to_string()
                    } else {
                        c.display_name.clone()
                    }
                })
                .ok_or_else(|| anyhow!("pair inconnu"))?
        };
        groups::validate_member_online(&self.inner, member_id)?;

        let (state, group_key) = {
            let group_states = self.inner.groups.lock().unwrap();
            let state = group_states
                .iter()
                .find(|g| g.group_id == group_id)
                .ok_or_else(|| anyhow!("groupe introuvable"))?;
            let state = state.clone();
            use base64::Engine;
            let key = base64::engine::general_purpose::STANDARD
                .decode(&state.key_b64)
                .map_err(|_| anyhow!("clé de groupe corrompue"))?;
            let key: [u8; 32] =
                key.try_into().map_err(|_| anyhow!("clé de groupe corrompue"))?;
            (state, key)
        };

        let mut new_members = state.members.clone();
        new_members.push(void_proto::MemberEntry {
            onion_id: member_id.to_string(),
            display_name: contact_display,
        });

        let blob =
            groups::encrypt_key_for(&self.inner, member_id, group_id, &group_key)?;
        let invite = void_proto::Frame::GroupInvite {
            group_id: group_id.to_string(),
            name: state.name.clone(),
            members: new_members.clone(),
            key: blob,
        };
        let invited = {
            let sessions = self.inner.sessions.lock().unwrap();
            sessions
                .get(member_id)
                .map(|s| s.tx.try_send(invite).is_ok())
                .unwrap_or(false)
        };
        if !invited {
            bail!("envoi de l'invitation impossible");
        }

        let others: Vec<void_proto::MemberEntry> = state
            .members
            .iter()
            .filter(|m| m.onion_id != my_id)
            .cloned()
            .collect();
        groups::fanout(
            &self.inner,
            &others,
            &void_proto::Frame::GroupMembers {
                group_id: group_id.to_string(),
                members: new_members.clone(),
            },
        );

        let mut group_states = self.inner.groups.lock().unwrap();
        let index = group_states
            .iter()
            .position(|g| g.group_id == group_id)
            .ok_or_else(|| anyhow!("groupe introuvable"))?;
        group_states[index].members = new_members;
        let _ = groups::save_groups(&self.inner.data_dir.join("groups.json"), &group_states);
        let info = groups::to_info(&self.inner, &group_states[index]);
        drop(group_states);
        let _ = self.inner.events.send(CoreEvent::GroupUpdated { group: info.clone() });
        Ok(info)
    }

    pub fn remove_group_member(&self, group_id: &str, member_id: &str) -> Result<GroupInfo> {
        let my_id = self.inner.identity.lock().unwrap().onion_id();
        if member_id == my_id {
            bail!("utilisez quitter le groupe");
        }
        let (state, group_key) = {
            let group_states = self.inner.groups.lock().unwrap();
            let state = group_states
                .iter()
                .find(|g| g.group_id == group_id)
                .ok_or_else(|| anyhow!("groupe introuvable"))?;
            if state.owner_id != my_id {
                bail!("seul le propriétaire peut exclure un membre");
            }
            if !state.members.iter().any(|m| m.onion_id == member_id) {
                bail!("ce pair n'est pas membre du groupe");
            }
            let state = state.clone();
            use base64::Engine;
            let key = base64::engine::general_purpose::STANDARD
                .decode(&state.key_b64)
                .map_err(|_| anyhow!("clé de groupe corrompue"))?;
            let key: [u8; 32] =
                key.try_into().map_err(|_| anyhow!("clé de groupe corrompue"))?;
            (state, key)
        };

        let remaining: Vec<void_proto::MemberEntry> = state
            .members
            .iter()
            .filter(|m| m.onion_id != member_id)
            .cloned()
            .collect();

        let new_key = groups::generate_key();
        for member in remaining.iter() {
            if member.onion_id == my_id {
                continue;
            }
            let blob =
                groups::encrypt_key_for(&self.inner, &member.onion_id, group_id, &new_key)?;
            groups::fanout(
                &self.inner,
                std::slice::from_ref(member),
                &void_proto::Frame::GroupRotate {
                    group_id: group_id.to_string(),
                    key: blob,
                },
            );
        }

        groups::fanout(
            &self.inner,
            &state.members,
            &void_proto::Frame::GroupRemove {
                group_id: group_id.to_string(),
                member_id: member_id.to_string(),
            },
        );
        let _ = group_key;

        let mut group_states = self.inner.groups.lock().unwrap();
        let index = group_states
            .iter()
            .position(|g| g.group_id == group_id)
            .ok_or_else(|| anyhow!("groupe introuvable"))?;
        group_states[index].members = remaining;
        group_states[index].key_b64 = {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(new_key)
        };
        let _ = groups::save_groups(&self.inner.data_dir.join("groups.json"), &group_states);
        let info = groups::to_info(&self.inner, &group_states[index]);
        drop(group_states);
        let _ = self.inner.events.send(CoreEvent::GroupUpdated { group: info.clone() });
        Ok(info)
    }

    pub fn leave_group(&self, group_id: &str) -> Result<()> {
        let state = {
            let group_states = self.inner.groups.lock().unwrap();
            let state = group_states
                .iter()
                .find(|g| g.group_id == group_id)
                .ok_or_else(|| anyhow!("groupe introuvable"))?;
            state.clone()
        };
        groups::fanout(
            &self.inner,
            &state.members,
            &void_proto::Frame::GroupLeave {
                group_id: group_id.to_string(),
            },
        );
        {
            let mut group_states = self.inner.groups.lock().unwrap();
            group_states.retain(|g| g.group_id != group_id);
            let _ = groups::save_groups(&self.inner.data_dir.join("groups.json"), &group_states);
        }
        self.inner
            .store
            .lock()
            .unwrap()
            .delete_conversation(group_id)?;
        let _ = self
            .inner
            .events
            .send(CoreEvent::GroupRemoved { group_id: group_id.to_string() });
        info!("groupe quitté: {}", state.name);
        Ok(())
    }

    pub fn send_group_message(&self, group_id: &str, text: &str) -> Result<DmMessage> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > 4000 {
            bail!("message invalide (1 à 4000 caractères)");
        }
        self.send_group_payload(group_id, MessageKind::Text, 0, text.as_bytes())
    }

    pub fn send_voice_group(
        &self,
        group_id: &str,
        data: &[u8],
        duration_ms: u32,
    ) -> Result<DmMessage> {
        if data.is_empty() || data.len() > VOICE_MAX_BYTES {
            bail!("note vocale invalide (max 60 secondes)");
        }
        if duration_ms == 0 || duration_ms > VOICE_MAX_DURATION_MS {
            bail!("durée invalide");
        }
        let message =
            self.send_group_payload(group_id, MessageKind::Voice, duration_ms, data)?;
        write_blob(&self.inner.data_dir, &message.message_id, data)
            .context("écriture de la note vocale")?;
        Ok(message)
    }

    fn send_group_payload(
        &self,
        group_id: &str,
        kind: MessageKind,
        duration_ms: u32,
        plaintext: &[u8],
    ) -> Result<DmMessage> {
        let (state, group_key) = {
            let group_states = self.inner.groups.lock().unwrap();
            let state = group_states
                .iter()
                .find(|g| g.group_id == group_id)
                .ok_or_else(|| anyhow!("groupe introuvable"))?;
            let state = state.clone();
            use base64::Engine;
            let key = base64::engine::general_purpose::STANDARD
                .decode(&state.key_b64)
                .map_err(|_| anyhow!("clé de groupe corrompue"))?;
            let key: [u8; 32] =
                key.try_into().map_err(|_| anyhow!("clé de groupe corrompue"))?;
            (state, key)
        };
        let my_id = self.inner.identity.lock().unwrap().onion_id();
        let message_id = ulid::Ulid::new();
        let timestamp_ms = groups::now_ms();
        let aad = void_proto::group_aad_parts(
            &message_id.to_bytes(),
            group_id,
            &my_id,
            timestamp_ms,
        );
        let (nonce, ciphertext) =
            void_crypto::dm::dm_encrypt(&group_key, &aad, plaintext)
                .ok_or_else(|| anyhow!("chiffrement impossible"))?;
        let envelope = void_proto::GroupEnvelope {
            message_id: message_id.to_bytes(),
            group_id: group_id.to_string(),
            author_id: my_id.clone(),
            timestamp_ms,
            kind: kind.as_u8(),
            duration_ms,
            nonce,
            ciphertext,
        };
        let payload = void_proto::encode_group_envelope(&envelope);

        let sessions = session::snapshot_sessions(&self.inner);
        let mut any_direct = false;
        for member in state.members.iter() {
            if member.onion_id == my_id {
                continue;
            }
            if let Some((_, tx)) = sessions.iter().find(|(onion, _)| *onion == member.onion_id) {
                if tx
                    .try_send(void_proto::Frame::GroupMsg {
                        envelope: envelope.clone(),
                    })
                    .is_ok()
                {
                    any_direct = true;
                }
            }
        }
        let mut any_relay = false;
        for member in state.members.iter() {
            if member.onion_id == my_id {
                continue;
            }
            if sessions.iter().any(|(onion, _)| *onion == member.onion_id) {
                continue;
            }
            for (relay_onion, tx) in sessions.iter() {
                if relay_onion == &member.onion_id {
                    continue;
                }
                if tx.try_send(void_proto::Frame::Relay {
                    recipient_id: member.onion_id.clone(),
                    kind: void_proto::RELAY_KIND_GROUP,
                    payload: payload.clone(),
                })
                .is_ok()
                {
                    any_relay = true;
                }
            }
        }

        let status = if any_direct || any_relay {
            DmStatus::Sent
        } else {
            DmStatus::Queued
        };
        let message = DmMessage {
            message_id: message_id.to_string(),
            peer_id: group_id.to_string(),
            author_id: my_id,
            body: if kind == MessageKind::Text {
                String::from_utf8_lossy(plaintext).into_owned()
            } else {
                String::new()
            },
            created_ms: timestamp_ms,
            status,
            kind,
            duration_ms,
        };
        self.inner.store.lock().unwrap().insert_message(&record_of(&message))?;
        Ok(message)
    }

    pub fn voice_blob(&self, message_id: &str) -> Option<Vec<u8>> {
        std::fs::read(blob_path(&self.inner.data_dir, message_id)).ok()
    }

    pub fn group_history(
        &self,
        group_id: &str,
        limit: u64,
        before_id: Option<&str>,
    ) -> Result<Vec<DmMessage>> {
        {
            let group_states = self.inner.groups.lock().unwrap();
            if !group_states.iter().any(|g| g.group_id == group_id) {
                bail!("groupe introuvable");
            }
        }
        let records = self
            .inner
            .store
            .lock()
            .unwrap()
            .list_messages(group_id, limit, before_id)?;
        Ok(records.into_iter().map(message_of).collect())
    }

    pub fn presence(&self) -> Vec<PresenceInfo> {
        session::compute_presence(&self.inner)
    }

    pub fn request_ping(&self, peer_onion_id: &str) {
        let sessions = self.inner.sessions.lock().unwrap();
        if let Some(session) = sessions.get(peer_onion_id) {
            let _ = session.tx.try_send(void_proto::Frame::Ping {
                timestamp_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            });
        }
    }

    pub fn identity_info(&self) -> IdentityInfo {
        let profile = self.inner.profile.lock().unwrap();
        let identity = self.inner.identity.lock().unwrap();
        IdentityInfo {
            display_name: profile.display_name.clone(),
            onion: identity.onion_address(),
            fingerprint: identity.fingerprint_short(),
        }
    }

    pub fn set_display_name(&self, name: &str) -> Result<()> {
        self.set_profile(Some(name.to_string()), None, None, None, None)
    }

    pub const AVATAR_MAX_BYTES: usize = 64_000;

    pub fn set_profile(
        &self,
        display_name: Option<String>,
        bio: Option<String>,
        status: Option<String>,
        accent: Option<String>,
        avatar_b64: Option<String>,
    ) -> Result<()> {
        let mut profile = self.inner.profile.lock().unwrap().clone();
        if let Some(name) = display_name {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > 32 {
                bail!("nom d'affichage invalide (1 à 32 caractères)");
            }
            profile.display_name = name.to_string();
        }
        if let Some(bio) = bio {
            let bio = bio.trim();
            if bio.chars().count() > 200 {
                bail!("bio invalide (200 caractères max)");
            }
            profile.bio = bio.to_string();
        }
        if let Some(status) = status {
            let status = status.trim();
            if status.chars().count() > 64 {
                bail!("statut invalide (64 caractères max)");
            }
            profile.status = status.to_string();
        }
        if let Some(accent) = accent {
            let accent = accent.trim();
            let valid = accent.is_empty()
                || (accent.len() == 7
                    && accent.starts_with('#')
                    && accent[1..].chars().all(|c| c.is_ascii_hexdigit()));
            if !valid {
                bail!("couleur d'accent invalide");
            }
            profile.accent = accent.to_string();
        }
        if let Some(avatar) = avatar_b64 {
            use base64::Engine;
            if !avatar.is_empty() {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(&avatar)
                    .map_err(|_| anyhow!("avatar illisible"))?;
                if bytes.len() > Self::AVATAR_MAX_BYTES {
                    bail!("avatar trop volumineux (64 Ko max)");
                }
            }
            profile.avatar_b64 = avatar;
        }
        let json = serde_json::to_string_pretty(&profile)?;
        std::fs::write(self.inner.data_dir.join("profile.json"), json)?;
        *self.inner.profile.lock().unwrap() = profile;

        let frame = {
            let profile = self.inner.profile.lock().unwrap();
            void_proto::Frame::ProfileUpdate {
                display_name: profile.display_name.clone(),
                bio: profile.bio.clone(),
                status: profile.status.clone(),
                accent: profile.accent.clone(),
                avatar_b64: profile.avatar_b64.clone(),
            }
        };
        let sessions = session::snapshot_sessions(&self.inner);
        for (_, tx) in sessions {
            let _ = tx.try_send(frame.clone());
        }
        Ok(())
    }

    pub fn own_profile(&self) -> OwnProfileInfo {
        let profile = self.inner.profile.lock().unwrap();
        OwnProfileInfo {
            display_name: profile.display_name.clone(),
            bio: profile.bio.clone(),
            status: profile.status.clone(),
            accent: profile.accent.clone(),
            has_avatar: !profile.avatar_b64.is_empty(),
        }
    }

    pub fn peer_profile(&self, onion_id: &str) -> Option<PeerProfileInfo> {
        let my_id = self.inner.identity.lock().unwrap().onion_id();
        if onion_id == my_id {
            let profile = self.inner.profile.lock().unwrap();
            let fingerprint = void_crypto::onion_id_to_public(onion_id)
                .map(|public| void_crypto::hex_encode(&public)[..16].to_string())
                .unwrap_or_default();
            return Some(PeerProfileInfo {
                onion_id: onion_id.to_string(),
                display_name: profile.display_name.clone(),
                bio: profile.bio.clone(),
                status: profile.status.clone(),
                accent: profile.accent.clone(),
                has_avatar: !profile.avatar_b64.is_empty(),
                fingerprint,
            });
        }
        let display_name = {
            let contacts = self.inner.contacts.lock().unwrap();
            contacts
                .iter()
                .find(|c| c.onion_id == onion_id)
                .map(|c| c.display_name.clone())
        }?;
        let peer = self
            .inner
            .peer_profiles
            .lock()
            .unwrap()
            .get(onion_id)
            .cloned()
            .unwrap_or_default();
        let fingerprint = void_crypto::onion_id_to_public(onion_id)
            .map(|public| void_crypto::hex_encode(&public)[..16].to_string())
            .unwrap_or_default();
        Some(PeerProfileInfo {
            onion_id: onion_id.to_string(),
            display_name,
            bio: peer.bio,
            status: peer.status,
            accent: peer.accent,
            has_avatar: peer.has_avatar,
            fingerprint,
        })
    }

    pub fn peer_profiles(&self) -> Vec<PeerProfileInfo> {
        let contacts = self.inner.contacts.lock().unwrap().clone();
        let profiles = self.inner.peer_profiles.lock().unwrap().clone();
        contacts
            .iter()
            .map(|contact| {
                let peer = profiles
                    .get(&contact.onion_id)
                    .cloned()
                    .unwrap_or_default();
                let fingerprint =
                    void_crypto::onion_id_to_public(&contact.onion_id)
                        .map(|public| {
                            void_crypto::hex_encode(&public)[..16].to_string()
                        })
                        .unwrap_or_default();
                PeerProfileInfo {
                    onion_id: contact.onion_id.clone(),
                    display_name: contact.display_name.clone(),
                    bio: peer.bio,
                    status: peer.status,
                    accent: peer.accent,
                    has_avatar: peer.has_avatar,
                    fingerprint,
                }
            })
            .collect()
    }

    pub fn avatar_b64(&self, onion_id: Option<&str>) -> Option<String> {
        match onion_id {
            None => {
                let avatar = self.inner.profile.lock().unwrap().avatar_b64.clone();
                if avatar.is_empty() { None } else { Some(avatar) }
            }
            Some(onion_id) => {
                let my_id = self.inner.identity.lock().unwrap().onion_id();
                if onion_id == my_id {
                    let avatar = self.inner.profile.lock().unwrap().avatar_b64.clone();
                    return if avatar.is_empty() { None } else { Some(avatar) };
                }
                let cached = self
                    .inner
                    .peer_profiles
                    .lock()
                    .unwrap()
                    .get(onion_id)
                    .map(|p| p.has_avatar)
                    .unwrap_or(false);
                if !cached {
                    return None;
                }
                let path = self
                    .inner
                    .data_dir
                    .join("avatars")
                    .join(format!("{onion_id}.png"));
                std::fs::read_to_string(path).ok()
            }
        }
    }

    pub fn recovery_phrase(&self) -> Result<String> {
        let seed = self.inner.identity.lock().unwrap().seed();
        Ok(void_crypto::recovery_phrase(&seed)?)
    }

    pub fn recovery_confirmed(&self) -> bool {
        self.inner.recovery.lock().unwrap().confirmed
    }

    pub fn confirm_recovery_phrase(&self) -> Result<()> {
        let state = RecoveryState { confirmed: true };
        let json = serde_json::to_string_pretty(&state)?;
        std::fs::write(self.inner.data_dir.join("recovery.json"), json)?;
        *self.inner.recovery.lock().unwrap() = state;
        Ok(())
    }

    pub async fn restore_from_phrase(&self, phrase: &str) -> Result<IdentityInfo> {
        let seed = void_crypto::seed_from_recovery_phrase(phrase)
            .map_err(|e| anyhow!("phrase invalide: {e}"))?;
        let identity = Identity::from_seed(seed);

        if identity.seed() == self.inner.identity.lock().unwrap().seed() {
            return Ok(self.identity_info());
        }
        if self.inner.bootstrapping.load(Ordering::SeqCst) {
            bail!("un bootstrap tor est déjà en cours, réessayez dans un instant");
        }

        if let Some(handle) = self.inner.tor.lock().await.take() {
            handle
                .shutdown()
                .await
                .map_err(|e| anyhow!("arrêt de tor: {e}"))?;
        }
        session::close_all_sessions(&self.inner);

        seed_store::write_seed(&self.inner.data_dir.join("identity.seed"), &identity.seed())
            .context("écriture de identity.seed")?;
        {
            let mut guard = self.inner.identity.lock().unwrap();
            *guard = identity;
        }
        {
            let state = RecoveryState { confirmed: true };
            let json = serde_json::to_string_pretty(&state)?;
            std::fs::write(self.inner.data_dir.join("recovery.json"), json)?;
            *self.inner.recovery.lock().unwrap() = state;
        }
        *self.inner.socks.lock().unwrap() = None;

        let _ = self
            .inner
            .status_tx
            .send_replace(TorStatus::Starting);
        info!("identité restaurée: {}", self.identity_info().onion);

        let bootstrap = Arc::clone(&self.inner);
        tokio::spawn(async move {
            run_bootstrap(bootstrap).await;
        });

        Ok(self.identity_info())
    }

    pub fn invite_link(&self) -> String {
        let identity = self.inner.identity.lock().unwrap();
        let display_name = self.inner.profile.lock().unwrap().display_name.clone();
        Invite::new(identity.onion_id(), identity.fingerprint_short(), display_name).to_link()
    }

    pub fn invite_qr_svg(&self) -> Result<String> {
        let link = self.invite_link();
        let code = qrcode::QrCode::new(link.as_bytes()).map_err(|e| anyhow!("qr: {e}"))?;
        let svg = code
            .render::<qrcode::render::svg::Color>()
            .min_dimensions(260, 260)
            .dark_color(qrcode::render::svg::Color("#000000"))
            .light_color(qrcode::render::svg::Color("#ffffff"))
            .build();
        Ok(match svg.find("<svg") {
            Some(index) => svg[index..].to_string(),
            None => svg,
        })
    }

    pub fn parse_invite(&self, link: &str) -> Result<PeerInfo> {
        let invite = Invite::parse(link).map_err(|e| anyhow!("{e}"))?;
        Ok(PeerInfo {
            onion_id: invite.onion_id,
            fingerprint: invite.fingerprint,
            display_name: invite.display_name,
            added_at: 0,
        })
    }

    pub fn add_peer(&self, link: &str) -> Result<PeerInfo> {
        let invite = Invite::parse(link).map_err(|e| anyhow!("{e}"))?;
        let my_id = self.inner.identity.lock().unwrap().onion_id();
        if invite.onion_id == my_id {
            bail!("vous ne pouvez pas vous ajouter vous-même");
        }
        let peer = PeerInfo {
            onion_id: invite.onion_id,
            fingerprint: invite.fingerprint,
            display_name: invite.display_name,
            added_at: unix_now(),
        };
        let mut contacts = self.inner.contacts.lock().unwrap();
        if contacts.iter().any(|p| p.onion_id == peer.onion_id) {
            bail!("ce pair est déjà dans vos contacts");
        }
        contacts.push(peer.clone());
        persist_json(&self.inner.data_dir.join("contacts.json"), &*contacts)?;
        drop(contacts);
        session::refresh_presence(&self.inner);
        self.inner.dial_notify.notify_one();
        Ok(peer)
    }

    pub fn peers(&self) -> Vec<PeerInfo> {
        self.inner.contacts.lock().unwrap().clone()
    }

    pub fn pending_requests(&self) -> Vec<PendingRequest> {
        self.inner.requests.lock().unwrap().clone()
    }

    pub fn accept_friend_request(&self, onion_id: &str) -> Result<PeerInfo> {
        let request = {
            let mut requests = self.inner.requests.lock().unwrap();
            let index = requests
                .iter()
                .position(|r| r.onion_id == onion_id)
                .ok_or_else(|| anyhow!("demande introuvable"))?;
            requests.remove(index)
        };
        let _ = persist_json(
            &self.inner.data_dir.join("requests.json"),
            &*self.inner.requests.lock().unwrap(),
        );
        let public = void_crypto::onion_id_to_public(onion_id)
            .ok_or_else(|| anyhow!("adresse oignon invalide"))?;
        let fingerprint = void_crypto::hex_encode(&public)[..16].to_string();
        let peer = PeerInfo {
            onion_id: onion_id.to_string(),
            fingerprint,
            display_name: if request.display_name.is_empty() {
                onion_id[..10].to_string()
            } else {
                request.display_name.clone()
            },
            added_at: unix_now(),
        };
        {
            let mut contacts = self.inner.contacts.lock().unwrap();
            if contacts.iter().any(|p| p.onion_id == peer.onion_id) {
                bail!("déjà dans vos contacts");
            }
            contacts.push(peer.clone());
            persist_json(&self.inner.data_dir.join("contacts.json"), &*contacts)?;
        }
        session::refresh_presence(&self.inner);
        self.inner.dial_notify.notify_one();
        let _ = self.inner.events.send(CoreEvent::FriendRequestHandled {
            peer_id: onion_id.to_string(),
        });
        info!("demande acceptée: {}", peer.display_name);
        Ok(peer)
    }

    pub fn decline_friend_request(&self, onion_id: &str) -> Result<()> {
        {
            let mut requests = self.inner.requests.lock().unwrap();
            let before = requests.len();
            requests.retain(|r| r.onion_id != onion_id);
            if requests.len() == before {
                bail!("demande introuvable");
            }
            persist_json(&self.inner.data_dir.join("requests.json"), &*requests)?;
        }
        let _ = self.inner.events.send(CoreEvent::FriendRequestHandled {
            peer_id: onion_id.to_string(),
        });
        Ok(())
    }

    pub fn remove_peer(&self, onion_id: &str) -> Result<()> {
        {
            let mut contacts = self.inner.contacts.lock().unwrap();
            let before = contacts.len();
            contacts.retain(|p| p.onion_id != onion_id);
            if contacts.len() == before {
                bail!("pair introuvable");
            }
            persist_json(&self.inner.data_dir.join("contacts.json"), &*contacts)?;
        }
        session::close_session(&self.inner, onion_id);
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        session::close_all_sessions(&self.inner);
        if let Some(handle) = self.inner.tor.lock().await.take() {
            handle
                .shutdown()
                .await
                .map_err(|e| anyhow!("arrêt de tor: {e}"))?;
        }
        Ok(())
    }
}

async fn run_bootstrap(inner: Arc<EngineInner>) {
    if inner.bootstrapping.swap(true, Ordering::SeqCst) {
        return;
    }
    let result = bootstrap_with_retries(&inner).await;
    if let Err(e) = result {
        error!("bootstrap tor échoué: {e:#}");
    }
    inner.bootstrapping.store(false, Ordering::SeqCst);
}

const BOOTSTRAP_MAX_ATTEMPTS: u32 = 3;

async fn bootstrap_with_retries(inner: &Arc<EngineInner>) -> Result<()> {
    let fail = |inner: &EngineInner, msg: String| {
        let _ = inner.status_tx.send_replace(TorStatus::Failed { error: msg });
    };
    for attempt in 1..=BOOTSTRAP_MAX_ATTEMPTS {
        match bootstrap_once(inner).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let restartable = e
                    .downcast_ref::<void_tor::TorError>()
                    .map(|te| {
                        matches!(te, void_tor::TorError::ProcessDied(_))
                            || matches!(te, void_tor::TorError::Io(_))
                    })
                    .unwrap_or(false);
                if attempt < BOOTSTRAP_MAX_ATTEMPTS && restartable {
                    warn!(
                        "bootstrap tor échoué (tentative {attempt}/{BOOTSTRAP_MAX_ATTEMPTS}): {e:#} — relance…"
                    );
                    let _ = inner.status_tx.send_replace(TorStatus::Starting);
                    tokio::time::sleep(Duration::from_secs(4)).await;
                    continue;
                }
                let msg = format!("{e:#}");
                fail(inner, msg.clone());
                bail!("{msg}");
            }
        }
    }
    unreachable!()
}

async fn bootstrap_once(inner: &Arc<EngineInner>) -> Result<()> {
    let fail = |inner: &EngineInner, msg: String| {
        let _ = inner.status_tx.send_replace(TorStatus::Failed { error: msg });
    };

    let (key_b64, expected_onion_id, target_port, virtual_port, tor_cfg) = {
        let identity = inner.identity.lock().unwrap();
        (
            identity.onion_service_key_b64(),
            identity.onion_id(),
            inner.app_port,
            session::P2P_VIRTUAL_PORT,
            TorConfig {
                tor_dir: inner.tor_cfg.tor_dir.clone(),
                data_dir: inner.tor_cfg.data_dir.clone(),
            },
        )
    };

    let mut boot = match launch(&tor_cfg).await {
        Ok(boot) => boot,
        Err(e) => {
            bail!(e);
        }
    };

    let deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    loop {
        if !boot.is_alive() {
            let msg = void_tor::TorError::ProcessDied(boot.exit_status());
            bail!(msg);
        }
        match boot.circuit_established().await {
            Ok(true) => break,
            Ok(false) => {}
            Err(e) => {
                if !boot.is_alive() {
                    let msg = void_tor::TorError::ProcessDied(boot.exit_status());
                    bail!(msg);
                }
                warn!("control: {e}");
            }
        }
        if let Ok((progress, _tag)) = boot.bootstrap_progress().await {
            let _ = inner
                .status_tx
                .send_replace(TorStatus::Bootstrapping { progress });
        }
        if tokio::time::Instant::now() >= deadline {
            let msg = "tor n'a pas établi de circuit en 240 secondes".to_string();
            fail(inner, msg.clone());
            bail!("{msg}");
        }
        tokio::time::sleep(Duration::from_millis(700)).await;
    }

    let handle = match boot
        .publish_onion(&key_b64, virtual_port, target_port)
        .await
    {
        Ok(handle) => handle,
        Err(e) => {
            bail!(e);
        }
    };

    if handle.onion_id() != expected_onion_id {
        let msg = "l'adresse oignon publiée ne correspond pas à l'identité locale".to_string();
        fail(inner, msg.clone());
        bail!("{msg}");
    }

    let socks = handle.socks();
    *inner.socks.lock().unwrap() = Some(socks);
    *inner.tor.lock().await = Some(handle);
    let _ = inner.status_tx.send_replace(TorStatus::Online {
        onion: format!("{expected_onion_id}.onion"),
        socks: socks.to_string(),
    });
    info!("void en ligne: {expected_onion_id}.onion (socks {socks})");
    inner.dial_notify.notify_one();
    Ok(())
}

async fn tor_supervisor(inner: Arc<EngineInner>) {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if inner.bootstrapping.load(Ordering::SeqCst) {
            continue;
        }
        let dead = {
            let mut tor = inner.tor.lock().await;
            match tor.as_mut() {
                Some(handle) => !handle.is_alive(),
                None => continue,
            }
        };
        if dead {
            warn!("le processus tor s'est arrêté en cours d'exécution — relance automatique");
            inner.tor.lock().await.take();
            session::close_all_sessions(&inner);
            let _ = inner.status_tx.send_replace(TorStatus::Failed {
                error: "le relais tor s'est arrêté (antivirus ?) — relance automatique en cours…".into(),
            });
            run_bootstrap(Arc::clone(&inner)).await;
        }
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

fn record_of(message: &DmMessage) -> void_store::DmRecord {
    void_store::DmRecord {
        id: message.message_id.clone(),
        peer_id: message.peer_id.clone(),
        author_id: message.author_id.clone(),
        body: message.body.clone(),
        created_ms: message.created_ms,
        status: message.status.as_u8(),
        kind: message.kind.as_u8(),
        duration_ms: message.duration_ms,
    }
}

fn message_of(record: void_store::DmRecord) -> DmMessage {
    DmMessage {
        message_id: record.id,
        peer_id: record.peer_id,
        author_id: record.author_id,
        body: record.body,
        created_ms: record.created_ms,
        status: DmStatus::from_u8(record.status),
        kind: MessageKind::from_u8(record.kind),
        duration_ms: record.duration_ms,
    }
}

pub(crate) const VOICE_MAX_BYTES: usize = 220_000;
pub(crate) const VOICE_MAX_DURATION_MS: u32 = 120_000;

pub(crate) fn blob_path(data_dir: &Path, message_id: &str) -> PathBuf {
    data_dir.join("blobs").join(format!("{message_id}.webm"))
}

pub(crate) fn write_blob(data_dir: &Path, message_id: &str, bytes: &[u8]) -> Result<PathBuf> {
    let path = blob_path(data_dir, message_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, bytes)?;
    Ok(path)
}

pub(crate) fn persist_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)?;
    std::fs::write(path, json)?;
    Ok(())
}

fn load_json_or_default<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> T {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn load_or_create_identity(data_dir: &Path) -> Result<Identity> {
    let path = data_dir.join("identity.seed");
    if path.exists() {
        let seed = seed_store::read_seed(&path)?;
        Ok(Identity::from_seed(seed))
    } else {
        let identity = Identity::generate();
        seed_store::write_seed(&path, &identity.seed()).context("écriture de identity.seed")?;
        info!("nouvelle identité générée: {}", identity.onion_address());
        Ok(identity)
    }
}

pub(crate) mod seed_store {
    use super::*;

    const MAGIC: &[u8] = b"VOIDDP1:";

    pub fn read_seed(path: &Path) -> Result<[u8; 32]> {
        let data = std::fs::read(path).context("lecture de identity.seed")?;
        let legacy = !data.starts_with(MAGIC);
        let seed: [u8; 32] = if let Some(blob) = data.strip_prefix(MAGIC) {
            let plain = unprotect(blob);
            plain
                .try_into()
                .map_err(|_| anyhow!("identity.seed illisible"))?
        } else {
            data.try_into()
                .map_err(|_| anyhow!("identity.seed corrompu"))?
        };
        if legacy {
            info!("migration du seed vers le stockage protégé (DPAPI)");
            let _ = write_seed(path, &seed);
        }
        Ok(seed)
    }

    pub fn write_seed(path: &Path, seed: &[u8; 32]) -> Result<()> {
        let mut out = MAGIC.to_vec();
        out.extend_from_slice(&protect(seed));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, out)?;
        Ok(())
    }

    #[cfg(windows)]
    fn protect(plain: &[u8]) -> Vec<u8> {
        use winapi::shared::minwindef::DWORD;
        use winapi::um::dpapi::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN};
        use winapi::um::winbase::LocalFree;
        use winapi::um::wincrypt::DATA_BLOB;
        unsafe {
            let mut input = DATA_BLOB {
                cbData: plain.len() as DWORD,
                pbData: plain.as_ptr() as *mut u8,
            };
            let mut output: DATA_BLOB = std::mem::zeroed();
            let ok = CryptProtectData(
                &mut input,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            );
            if ok != 0 {
                let encrypted =
                    std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
                LocalFree(output.pbData as _);
                encrypted
            } else {
                warn!("DPAPI indisponible, seed stocké en clair");
                plain.to_vec()
            }
        }
    }

    #[cfg(windows)]
    fn unprotect(blob: &[u8]) -> Vec<u8> {
        use winapi::shared::minwindef::DWORD;
        use winapi::um::dpapi::{CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN};
        use winapi::um::winbase::LocalFree;
        use winapi::um::wincrypt::DATA_BLOB;
        unsafe {
            let mut input = DATA_BLOB {
                cbData: blob.len() as DWORD,
                pbData: blob.as_ptr() as *mut u8,
            };
            let mut output: DATA_BLOB = std::mem::zeroed();
            let ok = CryptUnprotectData(
                &mut input,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            );
            if ok != 0 {
                let plain =
                    std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
                LocalFree(output.pbData as _);
                plain
            } else {
                Vec::new()
            }
        }
    }

    #[cfg(not(windows))]
    fn protect(plain: &[u8]) -> Vec<u8> {
        plain.to_vec()
    }

    #[cfg(not(windows))]
    fn unprotect(blob: &[u8]) -> Vec<u8> {
        blob.to_vec()
    }
}

fn load_profile(data_dir: &Path, identity: &Identity) -> Result<Profile> {
    let path = data_dir.join("profile.json");
    if path.exists() {
        if let Ok(profile) = serde_json::from_slice::<Profile>(&std::fs::read(&path)?) {
            return Ok(profile);
        }
    }
    let fingerprint = identity.fingerprint_short();
    let default_name = format!("void_{}", &fingerprint[..6.min(fingerprint.len())]);
    Ok(Profile {
        display_name: default_name,
        ..Profile::default()
    })
}

#[cfg(test)]
mod tests {
    use super::seed_store;

    #[test]
    fn seed_roundtrip_and_migration() {
        let dir = std::env::temp_dir().join(format!(
            "void-seed-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("identity.seed");
        let seed = [77u8; 32];

        seed_store::write_seed(&path, &seed).unwrap();
        let raw = std::fs::read(&path).unwrap();
        assert_ne!(&raw[..8], &seed[..8]);
        assert_eq!(seed_store::read_seed(&path).unwrap(), seed);

        std::fs::write(&path, seed).unwrap();
        assert_eq!(seed_store::read_seed(&path).unwrap(), seed);
        let migrated = std::fs::read(&path).unwrap();
        assert_ne!(migrated, seed.to_vec());
        assert_eq!(seed_store::read_seed(&path).unwrap(), seed);

        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod peerinfo_tests {
    use super::PeerInfo;

    #[test]
    fn serializes_camel_case() {
        let peer = PeerInfo {
            onion_id: "a".repeat(56),
            fingerprint: "abcd1234".into(),
            display_name: "nova".into(),
            added_at: 42,
        };
        let json = serde_json::to_string(&peer).unwrap();
        assert!(json.contains("onionId"));
        assert!(json.contains("displayName"));
        assert!(!json.contains("onion_id"));
    }

    #[test]
    fn reads_legacy_snake_case_contacts() {
        let json = format!(
            "{{\"onion_id\":\"{}\",\"fingerprint\":\"fp\",\"display_name\":\"x\",\"added_at\":7}}",
            "b".repeat(56)
        );
        let peer: PeerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(peer.onion_id, "b".repeat(56));
        assert_eq!(peer.added_at, 7);
    }
}

#[cfg(test)]
mod dmmessage_json_tests {
    use super::{DmMessage, DmStatus, MessageKind};

    #[test]
    fn field_names_are_camel_case() {
        let message = DmMessage {
            message_id: "01TEST".into(),
            peer_id: "p".into(),
            author_id: "a".into(),
            body: "salut".into(),
            created_ms: 1700000000000,
            status: DmStatus::Sent,
            kind: MessageKind::Text,
            duration_ms: 0,
        };
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("createdAt"), "champ createdAt manquant: {json}");
        assert!(json.contains("messageId"), "champ messageId manquant: {json}");
        assert!(!json.contains("created_ms"), "snake_case fuit: {json}");
    }
}
