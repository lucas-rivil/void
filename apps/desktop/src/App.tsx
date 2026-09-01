import { useEffect, useRef, useState } from "react";
import type {
  GroupInfo,
  IdentityInfo,
  PendingRequest,
  PeerInfo,
  PresenceInfo,
  Settings,
  TorStatus,
} from "./types";
import {
  getIdentity,
  getPresence,
  getSettings,
  getTorStatus,
  listGroups,
  listPeerProfiles,
  listPeers,
  listRequests,
  leaveGroup as leaveGroupApi,
  onCoreEvent,
  onIdentityChanged,
  onIdentityReady,
  onPresenceChanged,
  onTorStatus,
  removeGroupMember,
  removePeer,
  setSettings as setSettingsApi,
} from "./lib/void";
import type { PeerProfileInfo } from "./types";
import { useI18n } from "./lib/i18n";
import { notify } from "./lib/notify";
import { useDisableBrowserFeatures, useGlobalContextMenu } from "./lib/contextMenu";
import TitleBar from "./components/TitleBar";
import Rail from "./components/Rail";
import ChannelColumn from "./components/ChannelColumn";
import MembersColumn from "./components/MembersColumn";
import WelcomePanel from "./components/WelcomePanel";
import IdentityPanel from "./components/IdentityPanel";
import ChatPanel from "./components/ChatPanel";
import GroupChatPanel from "./components/GroupChatPanel";
import AddPeerModal from "./components/AddPeerModal";
import CreateGroupModal from "./components/CreateGroupModal";
import InviteGroupModal from "./components/InviteGroupModal";
import SettingsModal from "./components/SettingsModal";
import Starfield from "./components/Starfield";
import Fade from "./components/Fade";
import ProfileCard from "./components/ProfileCard";
import { invalidateAvatarCache } from "./components/Avatar";

export type View = "welcome" | "identity" | "peer" | "group";

export type UpdatePhase = "idle" | "checking" | "available" | "downloading" | "installing" | "error" | "none";

export interface UpdateState {
  phase: UpdatePhase;
  progress: number;
  version: string | null;
  error: string | null;
}

export default function App() {
  const { t } = useI18n();
  useDisableBrowserFeatures();
  const contextMenu = useGlobalContextMenu();
  const [status, setStatus] = useState<TorStatus>({ kind: "starting" });
  const [updateState, setUpdateState] = useState<UpdateState>({
    phase: "idle",
    progress: 0,
    version: null,
    error: null,
  });
  const [identity, setIdentity] = useState<IdentityInfo | null>(null);
  const [peers, setPeers] = useState<PeerInfo[]>([]);
  const [requests, setRequests] = useState<PendingRequest[]>([]);
  const [presence, setPresence] = useState<PresenceInfo[]>([]);
  const [groups, setGroups] = useState<GroupInfo[]>([]);
  const [view, setView] = useState<View>("welcome");
  const [selectedPeer, setSelectedPeer] = useState<string | null>(null);
  const [activeGroupId, setActiveGroupId] = useState<string | null>(null);
  const [needsOnboarding, setNeedsOnboarding] = useState(false);
  const [addPeerOpen, setAddPeerOpen] = useState(false);
  const [createGroupOpen, setCreateGroupOpen] = useState(false);
  const [inviteGroupFor, setInviteGroupFor] = useState<string | null>(null);
  const [settings, setSettingsState] = useState<Settings>({
    notificationsEnabled: true,
    language: "en",
  });
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [profileCardFor, setProfileCardFor] = useState<string | null>(null);
  const [peerProfiles, setPeerProfiles] = useState<PeerProfileInfo[]>([]);
  const settingsRef = useRef(settings);
  settingsRef.current = settings;
  const presenceRef = useRef<PresenceInfo[]>([]);
  presenceRef.current = presence;
  const groupsRef = useRef<GroupInfo[]>([]);
  groupsRef.current = groups;

  const refreshPeers = () => {
    listPeers().then(setPeers).catch(() => undefined);
  };

  const refreshRequests = () => {
    listRequests().then(setRequests).catch(() => undefined);
  };

  const refreshPeerProfiles = () => {
    listPeerProfiles().then(setPeerProfiles).catch(() => undefined);
  };

  const refreshGroups = () => {
    listGroups().then(setGroups).catch(() => undefined);
  };

  useEffect(() => {
    getTorStatus().then(setStatus).catch(() => undefined);
    getPresence().then(setPresence).catch(() => undefined);
    listGroups().then(setGroups).catch(() => undefined);
    getSettings().then(setSettingsState).catch(() => undefined);
    refreshRequests();
    refreshPeerProfiles();
    getIdentity()
      .then((info) => {
        setIdentity(info);
        if (!info.displayName) {
          setNeedsOnboarding(true);
          setView("identity");
        }
      })
      .catch(() => undefined);
    refreshPeers();

    const unlistenStatus = onTorStatus(setStatus);
    const unlistenPresence = onPresenceChanged(setPresence);
    const unlistenIdentity = onIdentityReady(setIdentity);
    const unlistenChanged = onIdentityChanged(setIdentity);
    const unlistenCore = onCoreEvent((event) => {
      if (
        event.type === "groupNew" ||
        event.type === "groupUpdated" ||
        event.type === "groupRemoved"
      ) {
        refreshGroups();
        if (event.type === "groupRemoved" && event.groupId === activeGroupId) {
          setActiveGroupId(null);
          setView("welcome");
        }
      }
      if (
        event.type === "friendRequest" ||
        event.type === "friendRequestHandled"
      ) {
        refreshRequests();
        if (event.type === "friendRequestHandled") {
          refreshPeers();
        }
      }
      if (event.type === "profileUpdated") {
        invalidateAvatarCache(event.peerId);
        refreshPeerProfiles();
      }
      if (
        event.type === "friendRequest" &&
        settingsRef.current.notificationsEnabled
      ) {
        void notify(
          t("notify.friendRequest", { name: event.displayName }),
          `${event.peerId.slice(0, 16)}…`
        );
      }
      const incoming =
        event.type === "dmNew" || event.type === "groupMessage"
          ? event.message
          : null;
      if (
        incoming &&
        !document.hasFocus() &&
        settingsRef.current.notificationsEnabled
      ) {
        let author = t("notify.newMessage");
        if (event.type === "groupMessage") {
          const group = groupsRef.current.find(
            (g) => g.groupId === incoming.peerId
          );
          const member = group?.members.find(
            (m) => m.onionId === incoming.authorId
          );
          author = member
            ? `${member.displayName} · ${group?.name ?? "groupe"}`
            : t("notify.group");
        } else {
          const peer = presenceRef.current.find(
            (p) => p.onionId === incoming.authorId
          );
          author = peer ? peer.displayName : "Void";
        }
        const excerpt =
          incoming.body.length > 80
            ? `${incoming.body.slice(0, 80)}…`
            : incoming.body;
        void notify(author, excerpt);
      }
    });

    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenPresence.then((fn) => fn());
      unlistenIdentity.then((fn) => fn());
      unlistenChanged.then((fn) => fn());
      unlistenCore.then((fn) => fn());
    };
  }, [activeGroupId]);

  const online = status.kind === "online";
  const activeGroup = groups.find((g) => g.groupId === activeGroupId) ?? null;
  const selectedPresence = presence.find((p) => p.onionId === selectedPeer);
  const myOnionId = identity ? identity.onion.replace(/\.onion$/, "") : null;
  const onlinePresence = presence.filter((p) => p.online);

  const selectPeer = (onionId: string) => {
    setSelectedPeer(onionId);
    setActiveGroupId(null);
    setView("peer");
  };

  const selectGroup = (groupId: string) => {
    setActiveGroupId(groupId);
    setView("group");
  };

  const selectHome = () => {
    setActiveGroupId(null);
    setView("welcome");
  };

  const fadeKey =
    view === "peer" ? `peer:${selectedPeer}` : view === "group" ? `group:${activeGroupId}` : view;

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-void-0">
      <TitleBar online={online} />
      <div className="flex min-h-0 flex-1">
        <Rail
          groups={groups}
          activeGroupId={activeGroupId}
          homeActive={view !== "group"}
          onSelectHome={selectHome}
          onSelectGroup={selectGroup}
          onCreateGroup={() => setCreateGroupOpen(true)}
          onOpenSettings={() => setSettingsOpen(true)}
        />
        <ChannelColumn
          identity={identity}
          online={online}
          view={view}
          selectedPeer={selectedPeer}
          onSelect={setView}
          onSelectPeer={selectPeer}
          peers={peers}
          presence={presence}
          requests={requests}
          activeGroup={activeGroup}
          onAddPeer={() => setAddPeerOpen(true)}
          onRequestsChanged={refreshRequests}
          onRemovePeer={(onionId) => {
            removePeer(onionId)
              .then(() => {
                refreshPeers();
                if (selectedPeer === onionId) {
                  setSelectedPeer(null);
                  setView("welcome");
                }
              })
              .catch(() => refreshPeers());
          }}
        />
        <main className="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-void-3">
          <div className="nebula-a pointer-events-none absolute inset-0 z-0 animate-drift-a" />
          <div className="nebula-b pointer-events-none absolute inset-0 z-0 animate-drift-b" />
          <Starfield />

          <Fade id={fadeKey}>
            {view === "identity" || !identity ? (
              <IdentityPanel
                identity={identity}
                status={status}
                onSaved={() => {
                  setNeedsOnboarding(false);
                  setView("welcome");
                  getIdentity()
                    .then(setIdentity)
                    .catch(() => undefined);
                }}
                onIdentityChanged={setIdentity}
              />
          ) : view === "peer" && selectedPresence ? (
            <ChatPanel
              entry={selectedPresence}
              myOnionId={myOnionId}
              online={online}
              peerProfiles={peerProfiles}
              onOpenProfile={(onionId) => setProfileCardFor(onionId)}
            />
          ) : view === "group" && activeGroup ? (
            <GroupChatPanel
              group={activeGroup}
              myOnionId={myOnionId}
              online={online}
              peerProfiles={peerProfiles}
              onOpenProfile={(onionId) => setProfileCardFor(onionId)}
                onInvite={() => setInviteGroupFor(activeGroup.groupId)}
                onLeave={() => {
                  if (
                    window.confirm(
                      t("group.leaveConfirm", { name: activeGroup.name })
                    )
                  ) {
                    leaveGroupApi(activeGroup.groupId)
                      .then(() => {
                        setActiveGroupId(null);
                        setView("welcome");
                        refreshGroups();
                      })
                      .catch(() => undefined);
                  }
                }}
              />
            ) : (
              <WelcomePanel
                identity={identity}
                status={status}
                needsOnboarding={needsOnboarding}
                peers={peers}
                onOpenIdentity={() => setView("identity")}
                onAddPeer={() => setAddPeerOpen(true)}
                onCreateGroup={() => setCreateGroupOpen(true)}
              />
            )}
          </Fade>
        </main>
        <MembersColumn
          identity={identity}
          online={online}
          presence={presence}
          group={activeGroup}
          myOnionId={myOnionId}
          isOwner={activeGroup !== null && activeGroup.ownerId === myOnionId}
          onRemoveMember={(onionId) => {
            if (!activeGroup) return;
            removeGroupMember(activeGroup.groupId, onionId)
              .then(refreshGroups)
              .catch(() => undefined);
          }}
          onOpenProfile={(onionId) => setProfileCardFor(onionId)}
        />
      </div>
      {addPeerOpen && (
        <AddPeerModal
          onClose={() => setAddPeerOpen(false)}
          onAdded={() => refreshPeers()}
        />
      )}
      {createGroupOpen && (
        <CreateGroupModal
          onlinePresence={onlinePresence}
          onClose={() => setCreateGroupOpen(false)}
          onCreated={refreshGroups}
        />
      )}
      {inviteGroupFor && activeGroup && (
        <InviteGroupModal
          group={activeGroup}
          onlinePresence={onlinePresence}
          onClose={() => setInviteGroupFor(null)}
          onAdded={refreshGroups}
        />
      )}
      {profileCardFor && (
        <ProfileCard
          onionId={profileCardFor}
          onClose={() => setProfileCardFor(null)}
          onMessage={
            profileCardFor === myOnionId
              ? undefined
              : (onionId) => selectPeer(onionId)
          }
        />
      )}
      {(updateState.phase === "downloading" || updateState.phase === "installing") && (
        <div className="fixed bottom-0 left-0 right-0 z-[90] border-t border-nebula/30 bg-void-1/95 backdrop-blur-md">
          <div className="flex items-center gap-3 px-6 py-3">
            {updateState.phase === "downloading" ? (
              <>
                <span className="h-3 w-3 shrink-0 animate-orbit rounded-full border-2 border-nebula-hi border-t-transparent" />
                <span className="font-display text-[13px] font-bold text-mist-1">
                  {t("update.downloading", {
                    percent: Math.round(updateState.progress * 100),
                  })}
                </span>
                <div className="mx-4 h-1.5 flex-1 overflow-hidden rounded-full bg-void-4">
                  <div
                    className="h-full rounded-full bg-nebula transition-all duration-300"
                    style={{ width: `${updateState.progress * 100}%` }}
                  />
                </div>
              </>
            ) : (
              <>
                <span className="h-3 w-3 shrink-0 animate-pulse rounded-full bg-nebula" />
                <span className="font-display text-[13px] font-bold text-mist-1">
                  {t("update.installing")}
                </span>
              </>
            )}
          </div>
        </div>
      )}
      {settingsOpen && (
        <SettingsModal
          settings={settings}
          onSettingsChanged={(next) => {
            setSettingsState(next);
            setSettingsApi(next).catch(() => undefined);
          }}
          onClose={() => setSettingsOpen(false)}
          updateState={updateState}
          setUpdateState={setUpdateState}
        />
      )}
      {contextMenu}
    </div>
  );
}
