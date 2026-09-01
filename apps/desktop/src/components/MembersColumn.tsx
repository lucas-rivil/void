import type { GroupInfo, IdentityInfo, PresenceInfo } from "../types";
import { useI18n } from "../lib/i18n";
import Avatar from "./Avatar";

interface Props {
  identity: IdentityInfo | null;
  online: boolean;
  presence: PresenceInfo[];
  group: GroupInfo | null;
  myOnionId: string | null;
  isOwner: boolean;
  onRemoveMember: (onionId: string) => void;
  onOpenProfile: (onionId: string) => void;
}

export default function MembersColumn({
  identity,
  online,
  presence,
  group,
  myOnionId,
  isOwner,
  onRemoveMember,
  onOpenProfile,
}: Props) {
  const { t } = useI18n();
  const myName = identity?.displayName ?? "…";

  if (group) {
    const onlineMembers = group.members.filter(
      (m) => m.online || (m.onionId === myOnionId && online)
    );
    const offlineMembers = group.members.filter((m) => !onlineMembers.includes(m));
    return (
      <aside className="hidden w-60 shrink-0 flex-col overflow-y-auto bg-void-2 px-3 py-4 lg:flex">
        <p className="px-2 pb-2 text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
          {t("members.inOrbit", { n: onlineMembers.length })}
        </p>
        {onlineMembers.map((member) => {
          const mine = member.onionId === myOnionId;
          return (
            <div
              key={member.onionId}
              className="group flex items-center gap-2.5 rounded-xl px-2 py-1.5 transition-colors hover:bg-white/[0.04]"
            >
              <button
                className="relative shrink-0"
                onClick={() => onOpenProfile(member.onionId)}
                title={member.displayName}
              >
                <Avatar onionId={member.onionId} name={member.displayName} size={32} />
                <span
                  className={`absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-void-2 ${
                    member.onionId === group.ownerId
                      ? "bg-ambre text-ambre animate-glow-pulse"
                      : "bg-nova text-nova animate-glow-pulse"
                  }`}
                />
              </button>
              <div className="min-w-0 flex-1">
                <p className="truncate text-[15px] font-medium text-mist-1">
                  {member.displayName}
                  {mine ? ` (${t("common.you")})` : ""}
                </p>
                <p className="text-[11px] text-mist-3">
                  {member.onionId === group.ownerId
                    ? t("members.owner")
                    : t("members.member")}
                </p>
              </div>
              {isOwner && !mine && (
                <button
                  onClick={() => onRemoveMember(member.onionId)}
                  title={t("channel.removePeer")}
                  className="hidden text-[13px] text-mist-3 hover:text-alerte group-hover:block"
                >
                  ✕
                </button>
              )}
            </div>
          );
        })}

        {offlineMembers.length > 0 && (
          <p className="mt-5 px-2 pb-2 text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
            {t("members.inTheVoid", { n: offlineMembers.length })}
          </p>
        )}
        {offlineMembers.map((member) => (
          <button
            key={member.onionId}
            onClick={() => onOpenProfile(member.onionId)}
            className="flex w-full items-center gap-2.5 rounded-xl px-2 py-1.5 text-left opacity-40 transition-opacity hover:bg-white/[0.04] hover:opacity-70"
          >
            <div className="relative shrink-0">
              <Avatar onionId={member.onionId} name={member.displayName} size={32} />
              <span className="absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-void-2 bg-void-6" />
            </div>
            <p className="truncate text-[15px] font-medium text-mist-1">
              {member.displayName}
              {member.onionId === myOnionId ? ` (${t("common.you")})` : ""}
            </p>
          </button>
        ))}
      </aside>
    );
  }

  const onlinePeers = presence.filter((p) => p.online);
  const offlinePeers = presence.filter((p) => !p.online);

  return (
    <aside className="hidden w-60 shrink-0 flex-col overflow-y-auto bg-void-2 px-3 py-4 lg:flex">
      <p className="px-2 pb-2 text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
        {t("members.inOrbit", { n: onlinePeers.length + 1 })}
      </p>

      <div className="flex items-center gap-2.5 rounded-xl px-2 py-1.5 transition-colors hover:bg-white/[0.04]">
        <button
          className="relative shrink-0"
          onClick={() => myOnionId && onOpenProfile(myOnionId)}
          title={myName}
        >
          <Avatar onionId={null} name={myName} size={32} />
          <span
            className={`absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-void-2 ${
              online ? "bg-nova text-nova animate-glow-pulse" : "bg-ambre"
            }`}
          />
        </button>
        <button
          className="min-w-0 flex-1 text-left"
          onClick={() => myOnionId && onOpenProfile(myOnionId)}
        >
          <p className="truncate text-[15px] font-medium text-mist-1">{myName}</p>
          <p className="text-[11px] text-mist-3">{t("members.youPeerOnion")}</p>
        </button>
        {identity && (
          <span
            className="text-[11px] text-nova"
            title={t("members.fingerprintVerified", { fp: identity.fingerprint })}
          >
            ✓
          </span>
        )}
      </div>

      {onlinePeers.map((peer) => (
        <div
          key={peer.onionId}
          className="flex items-center gap-2.5 rounded-xl px-2 py-1.5 transition-colors hover:bg-white/[0.04]"
        >
          <button
            className="relative shrink-0"
            onClick={() => onOpenProfile(peer.onionId)}
            title={peer.displayName}
          >
            <Avatar onionId={peer.onionId} name={peer.displayName} size={32} />
            <span className="absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-void-2 bg-nova text-nova animate-glow-pulse" />
          </button>
          <button
            className="min-w-0 flex-1 text-left"
            onClick={() => onOpenProfile(peer.onionId)}
          >
            <p className="truncate text-[15px] font-medium text-mist-1">
              {peer.displayName}
            </p>
            <p className="font-display text-[11px] font-medium text-mist-3">
              {peer.rttMs !== null ? `${peer.rttMs} ms · ` : ""}
              {peer.direction === "outgoing"
                ? t("members.outgoing")
                : t("members.incoming")}
            </p>
          </button>
        </div>
      ))}

      {offlinePeers.length > 0 && (
        <p className="mt-5 px-2 pb-2 text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
          {t("members.inTheVoid", { n: offlinePeers.length })}
        </p>
      )}
      {offlinePeers.map((peer) => (
        <button
          key={peer.onionId}
          onClick={() => onOpenProfile(peer.onionId)}
          className="flex w-full items-center gap-2.5 rounded-xl px-2 py-1.5 text-left opacity-40 transition-opacity hover:bg-white/[0.04] hover:opacity-70"
        >
          <div className="relative shrink-0">
            <Avatar onionId={peer.onionId} name={peer.displayName} size={32} />
            <span className="absolute -bottom-0.5 -right-0.5 h-3 w-3 rounded-full border-2 border-void-2 bg-void-6" />
          </div>
          <p className="truncate text-[15px] font-medium text-mist-1">
            {peer.displayName}
          </p>
        </button>
      ))}
    </aside>
  );
}
