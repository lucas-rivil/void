export type TorStatus =
  | { kind: "starting" }
  | { kind: "bootstrapping"; progress: number }
  | { kind: "online"; onion: string; socks: string }
  | { kind: "failed"; error: string };

export interface IdentityInfo {
  displayName: string;
  onion: string;
  fingerprint: string;
}

export interface PeerInfo {
  onionId: string;
  fingerprint: string;
  displayName: string;
  addedAt: number;
}

export interface PendingRequest {
  onionId: string;
  displayName: string;
  receivedAt: number;
}

export interface OwnProfileInfo {
  displayName: string;
  bio: string;
  status: string;
  accent: string;
  hasAvatar: boolean;
}

export interface PeerProfileInfo {
  onionId: string;
  displayName: string;
  bio: string;
  status: string;
  accent: string;
  hasAvatar: boolean;
  fingerprint: string;
}

export type Direction = "outgoing" | "incoming";

export interface PresenceInfo {
  onionId: string;
  displayName: string;
  online: boolean;
  direction: Direction | null;
  connectedSince: number | null;
  rttMs: number | null;
}

export type DmStatus = "queued" | "sent" | "delivered";
export type MessageKind = "text" | "voice";

export interface DmMessage {
  messageId: string;
  peerId: string;
  authorId: string;
  body: string;
  createdAt: number;
  status: DmStatus;
  kind: MessageKind;
  durationMs: number;
}

export type DmEvent =
  | { type: "dmNew"; message: DmMessage }
  | { type: "dmStatus"; peerId: string; messageId: string; status: DmStatus };

export interface GroupMemberInfo {
  onionId: string;
  displayName: string;
  online: boolean;
}

export interface GroupInfo {
  groupId: string;
  name: string;
  ownerId: string;
  members: GroupMemberInfo[];
  createdAt: number;
}

export type CoreEvent =
  | { type: "dmNew"; message: DmMessage }
  | { type: "dmStatus"; peerId: string; messageId: string; status: DmStatus }
  | { type: "groupNew"; group: GroupInfo }
  | { type: "groupUpdated"; group: GroupInfo }
  | { type: "groupRemoved"; groupId: string }
  | { type: "groupMessage"; message: DmMessage }
  | { type: "friendRequest"; peerId: string; displayName: string }
  | { type: "friendRequestHandled"; peerId: string }
  | { type: "profileUpdated"; peerId: string };

export interface Settings {
  notificationsEnabled: boolean;
  language: "en" | "fr";
}

export interface AppInfo {
  version: string;
  dataDir: string;
  relayQueue: number;
}
