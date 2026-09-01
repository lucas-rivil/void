import { useEffect, useState } from "react";
import type { PeerInfo } from "../types";
import { addPeer, parseInviteLink } from "../lib/void";
import Modal from "./Modal";
import { useI18n } from "../lib/i18n";

interface Props {
  onClose: () => void;
  onAdded: (peer: PeerInfo) => void;
}

export default function AddPeerModal({ onClose, onAdded }: Props) {
  const { t } = useI18n();
  const [link, setLink] = useState("");
  const [preview, setPreview] = useState<PeerInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    if (!link.trim()) {
      setPreview(null);
      setError(null);
      return;
    }
    let cancelled = false;
    parseInviteLink(link.trim())
      .then((peer) => {
        if (cancelled) return;
        setPreview(peer);
        setError(null);
      })
      .catch((e) => {
        if (cancelled) return;
        setPreview(null);
        setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [link]);

  const submit = async () => {
    setAdding(true);
    setError(null);
    try {
      const peer = await addPeer(link.trim());
      onAdded(peer);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setAdding(false);
    }
  };

  const label = preview?.displayName
    ? preview.displayName
    : preview
      ? t("addPeer.peerPrefix", { id: (preview.onionId ?? "").slice(0, 10) })
      : "";

  return (
    <Modal title={t("addPeer.title")} onClose={onClose} width={520}>
      <p className="text-[14px] leading-relaxed text-mist-2">
        {t("addPeer.subtitle")}
      </p>

      <input
        value={link}
        onChange={(event) => setLink(event.target.value)}
        placeholder="void://invite?onion=…&fp=…&n=…"
        autoFocus
        className="focus-glow mt-4 w-full rounded-xl border border-white/[0.07] bg-void-2 px-3.5 py-2.5 font-mono text-[13px] text-mist-1 outline-none placeholder:text-mist-3"
      />

      {error && <p className="mt-2 break-words text-[13px] text-alerte">{error}</p>}

      {preview && (
        <div className="mt-3 animate-fade-up rounded-xl border border-white/[0.06] bg-void-2 p-4">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-nebula/20 font-display text-sm font-bold text-nebula-hi">
              {label.slice(0, 1).toUpperCase()}
            </div>
            <div className="min-w-0 flex-1">
              <p className="truncate text-[15px] font-semibold text-mist-1">{label}</p>
              <p className="font-mono text-[11px] text-mist-3">
                fp {preview.fingerprint}
              </p>
            </div>
            <span className="font-display text-[10px] font-bold uppercase tracking-[0.12em] text-nova">
              {t("addPeer.validOnion")}
            </span>
          </div>
        </div>
      )}

      <div className="mt-5 flex items-center justify-end gap-3">
        <button
          onClick={onClose}
          className="btn-press rounded-xl px-4 py-2 text-[14px] font-medium text-mist-2 hover:text-mist-1"
        >
          {t("common.cancel")}
        </button>
        <button
          onClick={() => void submit()}
          disabled={!preview || adding}
          className="btn-press focus-glow rounded-xl bg-nebula px-4 py-2 text-[14px] font-semibold text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi disabled:opacity-50 disabled:shadow-none"
        >
          {adding ? t("addPeer.adding") : t("addPeer.addButton")}
        </button>
      </div>
    </Modal>
  );
}
