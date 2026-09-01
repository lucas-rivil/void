import { useState } from "react";
import type { GroupInfo, PresenceInfo } from "../types";
import { addGroupMember } from "../lib/void";
import Modal from "./Modal";
import { useI18n } from "../lib/i18n";

interface Props {
  group: GroupInfo;
  onlinePresence: PresenceInfo[];
  onClose: () => void;
  onAdded: () => void;
}

export default function InviteGroupModal({
  group,
  onlinePresence,
  onClose,
  onAdded,
}: Props) {
  const { t } = useI18n();
  const memberIds = new Set(group.members.map((m) => m.onionId));
  const candidates = onlinePresence.filter((p) => !memberIds.has(p.onionId));
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const invite = async (onionId: string) => {
    setBusy(onionId);
    setError(null);
    try {
      await addGroupMember(group.groupId, onionId);
      onAdded();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Modal
      title={t("inviteGroup.title", { name: group.name })}
      onClose={onClose}
      width={460}
    >
      {candidates.length === 0 ? (
        <p className="rounded-xl bg-void-2 p-4 text-[14px] leading-relaxed text-mist-3">
          {t("inviteGroup.none")}
        </p>
      ) : (
        <div className="max-h-64 space-y-1 overflow-y-auto pr-1">
          {candidates.map((peer) => (
            <button
              key={peer.onionId}
              onClick={() => void invite(peer.onionId)}
              disabled={busy !== null}
              className="btn-press flex w-full items-center gap-2.5 rounded-xl px-3 py-2 text-left text-[14px] text-mist-1 hover:bg-white/[0.04] disabled:opacity-50"
            >
              <span className="truncate">{peer.displayName}</span>
              <span className="ml-auto font-display text-[11px] font-bold uppercase tracking-wider text-nova">
                {busy === peer.onionId
                  ? t("inviteGroup.inviting")
                  : t("inviteGroup.inviteAction")}
              </span>
            </button>
          ))}
        </div>
      )}
      {error && <p className="mt-3 text-[13px] text-alerte">{error}</p>}
    </Modal>
  );
}
