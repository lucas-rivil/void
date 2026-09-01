import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppInfo,
  CoreEvent,
  DmMessage,
  GroupInfo,
  IdentityInfo,
  OwnProfileInfo,
  PendingRequest,
  PeerInfo,
  PeerProfileInfo,
  PresenceInfo,
  Settings,
  TorStatus,
} from "../types";

export const getIdentity = () => invoke<IdentityInfo>("get_identity");

export const getTorStatus = () => invoke<TorStatus>("get_tor_status");

export const setDisplayName = (name: string) =>
  invoke<void>("set_display_name", { name });

export const setProfile = (patch: {
  displayName?: string;
  bio?: string;
  status?: string;
  accent?: string;
  avatarB64?: string;
}) =>
  invoke<void>("set_profile", {
    displayName: patch.displayName ?? null,
    bio: patch.bio ?? null,
    status: patch.status ?? null,
    accent: patch.accent ?? null,
    avatarB64: patch.avatarB64 ?? null,
  });

export const getOwnProfile = () => invoke<OwnProfileInfo>("get_own_profile");

export const listPeerProfiles = () =>
  invoke<PeerProfileInfo[]>("list_peer_profiles");

export const getPeerProfile = (onionId: string) =>
  invoke<PeerProfileInfo>("get_peer_profile", { onionId });

export const getAvatar = (onionId: string | null) =>
  invoke<string>("get_avatar", { onionId });

export const getRecoveryPhrase = () => invoke<string>("get_recovery_phrase");

export const isRecoveryConfirmed = () => invoke<boolean>("is_recovery_confirmed");

export const confirmRecoveryPhrase = () =>
  invoke<void>("confirm_recovery_phrase");

export const restoreFromPhrase = (phrase: string) =>
  invoke<IdentityInfo>("restore_from_phrase", { phrase });

export const getInviteLink = () => invoke<string>("get_invite_link");

export const getInviteQr = () => invoke<string>("get_invite_qr");

export const parseInviteLink = (link: string) =>
  invoke<PeerInfo>("parse_invite_link", { link });

export const addPeer = (link: string) => invoke<PeerInfo>("add_peer", { link });

export const listPeers = () => invoke<PeerInfo[]>("list_peers");

export const listRequests = () => invoke<PendingRequest[]>("list_requests");

export const acceptRequest = (onionId: string) =>
  invoke<PeerInfo>("accept_request", { onionId });

export const declineRequest = (onionId: string) =>
  invoke<void>("decline_request", { onionId });

export const removePeer = (onionId: string) =>
  invoke<void>("remove_peer", { onionId });

export const getPresence = () => invoke<PresenceInfo[]>("get_presence");

export const sendPing = (onionId: string) =>
  invoke<void>("send_ping", { onionId });

export const sendDm = (onionId: string, text: string) =>
  invoke<DmMessage>("send_dm", { onionId, text });

export const sendVoiceDm = (onionId: string, data: string, durationMs: number) =>
  invoke<DmMessage>("send_voice_dm", { onionId, data, durationMs });

export const sendVoiceGroup = (groupId: string, data: string, durationMs: number) =>
  invoke<DmMessage>("send_voice_group", { groupId, data, durationMs });

export const getVoiceBlob = (messageId: string) =>
  invoke<string>("get_voice_blob", { messageId });

export const dmHistory = (
  onionId: string,
  limit = 100,
  beforeId?: string
) => invoke<DmMessage[]>("dm_history", { onionId, limit, beforeId });

export const listGroups = () => invoke<GroupInfo[]>("list_groups");

export const createGroup = (name: string, members: string[]) =>
  invoke<GroupInfo>("create_group", { name, members });

export const addGroupMember = (groupId: string, onionId: string) =>
  invoke<GroupInfo>("add_group_member", { groupId, onionId });

export const removeGroupMember = (groupId: string, onionId: string) =>
  invoke<GroupInfo>("remove_group_member", { groupId, onionId });

export const leaveGroup = (groupId: string) =>
  invoke<void>("leave_group", { groupId });

export const sendGroupMessage = (groupId: string, text: string) =>
  invoke<DmMessage>("send_group_message", { groupId, text });

export const groupHistory = (
  groupId: string,
  limit = 100,
  beforeId?: string
) => invoke<DmMessage[]>("group_history", { groupId, limit, beforeId });

export const getSettings = () => invoke<Settings>("get_settings");

export const setSettings = (settings: Settings) =>
  invoke<void>("set_settings", { settings });

export const getAppInfo = () => invoke<AppInfo>("get_app_info");

export const onTorStatus = (cb: (status: TorStatus) => void) =>
  listen<TorStatus>("tor:status", (event) => cb(event.payload));

export const onPresenceChanged = (cb: (presence: PresenceInfo[]) => void) =>
  listen<PresenceInfo[]>("presence:changed", (event) => cb(event.payload));

export const onIdentityReady = (cb: (identity: IdentityInfo) => void) =>
  listen<IdentityInfo>("identity:ready", (event) => cb(event.payload));

export const onIdentityChanged = (cb: (identity: IdentityInfo) => void) =>
  listen<IdentityInfo>("identity:changed", (event) => cb(event.payload));

export const onCoreEvent = (cb: (event: CoreEvent) => void) =>
  listen<CoreEvent>("core:event", (event) => cb(event.payload));
