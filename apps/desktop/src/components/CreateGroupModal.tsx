import { useState } from "react";
import type { PresenceInfo } from "../types";
import { createGroup } from "../lib/void";
import Modal from "./Modal";
import { useI18n } from "../lib/i18n";

interface Props {
  onlinePresence: PresenceInfo[];
  onClose: () => void;
  onCreated: () => void;
}

export default function CreateGroupModal({
  onlinePresence,
  onClose,
  onCreated,
}: Props) {
  const { t } = useI18n();
  const [name, setName] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const toggle = (onionId: string) => {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(onionId)) {
        next.delete(onionId);
      } else {
        next.add(onionId);
      }
      return next;
    });
  };

  const submit = async () => {
    setCreating(true);
    setError(null);
    try {
      await createGroup(name.trim(), Array.from(selected));
      onCreated();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  };

  return (
    <Modal title={t("createGroup.title")} onClose={onClose} width={520}>
      <p className="text-[14px] leading-relaxed text-mist-2">
        {t("createGroup.subtitle")}
      </p>

      <div className="mt-5">
        <label className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
          {t("createGroup.name")}
        </label>
        <input
          value={name}
          onChange={(event) => setName(event.target.value)}
          maxLength={32}
          autoFocus
          placeholder={t("createGroup.namePlaceholder")}
          className="focus-glow mt-2 w-full rounded-xl border border-white/[0.07] bg-void-2 px-3.5 py-2.5 text-[15px] text-mist-1 outline-none placeholder:text-mist-3"
        />
      </div>

      <div className="mt-5">
        <label className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
          {t("createGroup.members")}
        </label>
        {onlinePresence.length === 0 ? (
          <p className="mt-2.5 rounded-xl bg-void-2 p-3.5 text-[13px] leading-relaxed text-mist-3">
            {t("createGroup.noOnline")}
          </p>
        ) : (
          <div className="mt-2.5 max-h-52 space-y-1 overflow-y-auto pr-1">
            {onlinePresence.map((peer) => {
              const active = selected.has(peer.onionId);
              return (
                <button
                  key={peer.onionId}
                  onClick={() => toggle(peer.onionId)}
                  className={`btn-press flex w-full items-center gap-2.5 rounded-xl px-3 py-2 text-left text-[14px] ${
                    active
                      ? "bg-nebula/15 text-mist-1 shadow-[inset_0_0_0_1px_rgba(255,255,255,0.3)]"
                      : "text-mist-2 hover:bg-white/[0.04] hover:text-mist-1"
                  }`}
                >
                  <span className={active ? "text-nebula-hi" : "text-mist-3"}>
                    {active ? "◉" : "○"}
                  </span>
                  <span className="truncate">{peer.displayName}</span>
                </button>
              );
            })}
          </div>
        )}
      </div>

      {error && <p className="mt-3 text-[13px] text-alerte">{error}</p>}

      <div className="mt-5 flex items-center justify-end gap-3">
        <button
          onClick={onClose}
          className="btn-press rounded-xl px-4 py-2 text-[14px] font-medium text-mist-2 hover:text-mist-1"
        >
          {t("common.cancel")}
        </button>
        <button
          onClick={() => void submit()}
          disabled={creating || !name.trim()}
          className="btn-press focus-glow rounded-xl bg-nebula px-4 py-2 text-[14px] font-semibold text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi disabled:opacity-50 disabled:shadow-none"
        >
          {creating ? t("createGroup.creating") : t("createGroup.createButton")}
        </button>
      </div>
    </Modal>
  );
}
