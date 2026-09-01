import { useEffect, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { getPeerProfile } from "../lib/void";
import type { PeerProfileInfo } from "../types";
import Avatar from "./Avatar";
import Modal from "./Modal";
import { useI18n } from "../lib/i18n";

interface Props {
  onionId: string;
  onClose: () => void;
  onMessage?: (onionId: string) => void;
}

export default function ProfileCard({ onionId, onClose, onMessage }: Props) {
  const { t } = useI18n();
  const [profile, setProfile] = useState<PeerProfileInfo | null>(null);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getPeerProfile(onionId)
      .then(setProfile)
      .catch((e) => setError(String(e)));
  }, [onionId]);

  const copyAddress = async () => {
    await writeText(`${onionId}.onion`);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <Modal
      title={t("profile.title", {
        name: profile?.displayName ?? "…",
      })}
      onClose={onClose}
      width={440}
    >
      {error && <p className="text-[13px] text-alerte">{error}</p>}
      {!error && !profile && (
        <p className="text-[13px] text-mist-3">{t("profile.loading")}</p>
      )}
      {profile && (
        <div className="space-y-4">
          <div className="flex items-center gap-4">
            <Avatar onionId={onionId} name={profile.displayName} size={72} />
            <div className="min-w-0 flex-1">
              <p
                className="truncate font-display text-xl font-bold"
                style={{ color: profile.accent || "#f5f5f5" }}
              >
                {profile.displayName}
              </p>
              {profile.status && (
                <p className="mt-0.5 truncate text-[13px] italic text-mist-2">
                  {profile.status}
                </p>
              )}
              <p className="mt-1 font-mono text-[11px] text-mist-3">
                {profile.fingerprint}
              </p>
            </div>
          </div>

          {profile.bio && (
            <div className="rounded-xl border border-white/[0.06] bg-void-2 p-4">
              <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
                {t("profile.bio")}
              </p>
              <p className="mt-1.5 whitespace-pre-wrap break-words text-[14px] leading-relaxed text-mist-1">
                {profile.bio}
              </p>
            </div>
          )}

          <div className="rounded-xl border border-white/[0.06] bg-void-2 p-4">
            <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
              {t("profile.address")}
            </p>
            <p className="mt-1.5 break-all font-mono text-[12px] leading-relaxed text-nova/90">
              {onionId}.onion
            </p>
          </div>

          <div className="flex items-center justify-end gap-2">
            <button
              onClick={() => void copyAddress()}
              className="btn-press rounded-xl bg-void-2 px-4 py-2 text-[13px] font-medium text-mist-1 hover:bg-void-5"
            >
              {copied ? t("common.copied") : t("profile.copyAddress")}
            </button>
            {onMessage && (
              <button
                onClick={() => {
                  onMessage(onionId);
                  onClose();
                }}
                className="btn-press focus-glow rounded-xl bg-nebula px-4 py-2 font-display text-[12px] font-bold uppercase tracking-wider text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi"
              >
                {t("profile.message")}
              </button>
            )}
          </div>
        </div>
      )}
    </Modal>
  );
}
