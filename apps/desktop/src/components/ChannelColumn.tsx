import type { GroupInfo, IdentityInfo, PendingRequest, PeerInfo, PresenceInfo } from "../types";
import type { View } from "../App";
import { useI18n } from "../lib/i18n";
import { acceptRequest, declineRequest } from "../lib/void";
import Avatar from "./Avatar";

interface Props {
  identity: IdentityInfo | null;
  online: boolean;
  view: View;
  selectedPeer: string | null;
  onSelect: (view: View) => void;
  onSelectPeer: (onionId: string) => void;
  peers: PeerInfo[];
  presence: PresenceInfo[];
  requests: PendingRequest[];
  activeGroup: GroupInfo | null;
  onAddPeer: () => void;
  onRemovePeer: (onionId: string) => void;
  onRequestsChanged: () => void;
}

export default function ChannelColumn({
  identity,
  online,
  view,
  selectedPeer,
  onSelect,
  onSelectPeer,
  peers,
  presence,
  requests,
  activeGroup,
  onAddPeer,
  onRemovePeer,
  onRequestsChanged,
}: Props) {
  const { t } = useI18n();
  const name = identity?.displayName ?? "…";

  if (activeGroup) {
    return (
      <div className="flex w-60 shrink-0 flex-col bg-void-2">
        <div className="flex h-14 items-center gap-2 border-b border-white/[0.05] px-5">
          <span className="truncate font-display text-[14px] font-bold tracking-wide text-mist-1">
            {activeGroup.name}
          </span>
          <span className="ml-auto font-display text-[11px] font-medium text-mist-3">
            {activeGroup.members.length}
          </span>
        </div>
        <div className="flex-1 overflow-y-auto px-3 py-4">
          <p className="px-2 pb-2 text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
            {t("channel.textChannels")}
          </p>
          <div className="flex w-full items-center gap-2 rounded-xl bg-white/[0.07] px-3 py-2 text-left text-[15px] text-mist-1">
            <span className="text-mist-3">#</span> {t("channel.general")}
          </div>
        </div>
        <UserBar name={name} online={online} />
      </div>
    );
  }

  const presenceByOnion = new Map(presence.map((p) => [p.onionId, p]));
  const entries = peers.map(
    (peer) =>
      presenceByOnion.get(peer.onionId) ?? {
        onionId: peer.onionId,
        displayName: peer.displayName || (peer.onionId ?? "").slice(0, 12),
        online: false,
        direction: null,
        connectedSince: null,
        rttMs: null,
      }
  );
  const onlineCount = entries.filter((e) => e.online).length;

  return (
    <div className="flex w-60 shrink-0 flex-col bg-void-2">
      <div className="flex h-14 items-center gap-2 border-b border-white/[0.05] px-5">
        <span className="h-1.5 w-1.5 rounded-full bg-nebula shadow-[0_0_8px_rgba(255,255,255,0.8)]" />
        <span className="font-display text-[14px] font-bold uppercase tracking-[0.14em] text-mist-1">
          Void
        </span>
        <span
          className={`ml-auto font-display text-[10px] font-bold tracking-widest ${
            online ? "text-nova" : "text-ambre"
          }`}
        >
          {online ? "TOR" : "…"}
        </span>
      </div>

      <div className="flex-1 overflow-y-auto px-3 py-4">
        <p className="px-2 pb-2 text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
          {t("channel.home")}
        </p>
        <ChannelItem
          active={view === "welcome"}
          icon="#"
          label={t("channel.welcome")}
          onClick={() => onSelect("welcome")}
        />
        <ChannelItem
          active={view === "identity"}
          icon="@"
          label={t("channel.myIdentity")}
          onClick={() => onSelect("identity")}
          className="mb-5"
        />

        {requests.length > 0 && (
          <>
            <div className="mb-5">
              <p className="flex items-center gap-2 px-2 pb-2 text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
                {t("friend.requests")} — {requests.length}
                <span className="h-1.5 w-1.5 rounded-full bg-alerte text-alerte animate-glow-pulse" />
              </p>
              {requests.map((request) => (
                <div
                  key={request.onionId}
                  className="group flex w-full items-center gap-2 rounded-xl bg-alerte/[0.06] px-3 py-2 text-left text-[14px] text-mist-1"
                  title={`${request.onionId}.onion`}
                >
                  <span className="relative text-base leading-none text-mist-3">
                    @
                    <span className="absolute -bottom-1 -right-1 h-2.5 w-2.5 rounded-full border border-void-2 bg-ambre text-ambre" />
                  </span>
                  <span className="truncate">{request.displayName}</span>
                  <div className="ml-auto flex items-center gap-1.5">
                    <button
                      onClick={() => {
                        acceptRequest(request.onionId)
                          .then(onRequestsChanged)
                          .catch(() => undefined);
                      }}
                      title={t("friend.accept")}
                      className="btn-press flex h-6 w-6 items-center justify-center rounded-lg bg-nova/15 text-[13px] font-bold text-nova hover:bg-nova/25"
                    >
                      ✓
                    </button>
                    <button
                      onClick={() => {
                        declineRequest(request.onionId)
                          .then(onRequestsChanged)
                          .catch(() => undefined);
                      }}
                      title={t("friend.decline")}
                      className="btn-press flex h-6 w-6 items-center justify-center rounded-lg bg-void-4 text-[13px] text-mist-3 hover:text-alerte"
                    >
                      ✕
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}

        <div className="flex items-center justify-between px-2 pb-2">
          <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
            {t("channel.messages")} — {onlineCount}/{peers.length}
          </p>
          <button
            onClick={onAddPeer}
            title={t("channel.addPeer")}
            className="btn-press text-[16px] leading-none text-mist-3 hover:text-nebula-hi"
          >
            +
          </button>
        </div>

        {entries.length === 0 && (
          <div className="rounded-xl px-3 py-2 text-[14px] text-mist-3/70">
            <span className="mr-2 text-lg leading-none align-middle">@</span>
            {t("channel.noPeers")}
          </div>
        )}

        {entries.map((entry) => (
          <div
            key={entry.onionId}
            className={`group flex w-full items-center gap-2 rounded-xl px-3 py-2 text-left text-[15px] transition-colors ${
              view === "peer" && selectedPeer === entry.onionId
                ? "bg-white/[0.08] text-mist-1"
                : "text-mist-2 hover:bg-white/[0.04] hover:text-mist-1"
            }`}
          >
            <button
              className="flex min-w-0 flex-1 items-center gap-2"
              onClick={() => onSelectPeer(entry.onionId)}
              title={`${entry.onionId}.onion`}
            >
              <span className="relative text-base leading-none text-mist-3">
                @
                <span
                  className={`absolute -bottom-1 -right-1 h-2.5 w-2.5 rounded-full border border-void-2 ${
                    entry.online
                      ? "bg-nova text-nova animate-glow-pulse"
                      : "bg-void-6"
                  }`}
                />
              </span>
              <span className="truncate">{entry.displayName}</span>
              {entry.online && entry.rttMs !== null && entry.rttMs < 60000 && (
                <span className="ml-auto font-display text-[10px] font-medium text-nova/80">
                  {entry.rttMs}ms
                </span>
              )}
            </button>
            <button
              onClick={() => onRemovePeer(entry.onionId)}
              title={t("channel.removePeer")}
              className="hidden text-[13px] text-mist-3 hover:text-alerte group-hover:block"
            >
              ✕
            </button>
          </div>
        ))}
      </div>

      <UserBar name={name} online={online} />
    </div>
  );
}

function UserBar({ name, online }: { name: string; online: boolean }) {
  const { t } = useI18n();
  return (
    <div className="flex items-center gap-2.5 border-t border-white/[0.05] bg-void-1/60 px-3 py-3">
      <div className="relative shrink-0">
        <Avatar onionId={null} name={name} size={36} />
        <span
          className={`absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-void-2 ${
            online ? "bg-nova text-nova animate-glow-pulse" : "bg-ambre"
          }`}
        />
      </div>
      <div className="min-w-0 flex-1 leading-tight">
        <p className="truncate text-[13px] font-semibold text-mist-1">{name}</p>
        <p className="truncate text-[11px] text-mist-3">{t("channel.viaTor")}</p>
      </div>
    </div>
  );
}

function ChannelItem({
  active,
  icon,
  label,
  onClick,
  className = "",
}: {
  active: boolean;
  icon: string;
  label: string;
  onClick: () => void;
  className?: string;
}) {
  return (
    <button
      onClick={onClick}
      className={`mb-0.5 flex w-full items-center gap-2 rounded-xl px-3 py-2 text-left text-[15px] transition-colors ${className} ${
        active
          ? "bg-white/[0.08] text-mist-1"
          : "text-mist-2 hover:bg-white/[0.04] hover:text-mist-1"
      }`}
    >
      <span className="text-lg leading-none text-mist-3">{icon}</span> {label}
    </button>
  );
}
