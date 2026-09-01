import { useEffect, useRef, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { IdentityInfo, OwnProfileInfo, TorStatus } from "../types";
import {
  confirmRecoveryPhrase,
  getOwnProfile,
  getRecoveryPhrase,
  isRecoveryConfirmed,
  restoreFromPhrase,
  setProfile,
} from "../lib/void";
import TorStatusCard from "./TorStatusCard";
import Avatar, { invalidateAvatarCache } from "./Avatar";
import { useI18n } from "../lib/i18n";

interface Props {
  identity: IdentityInfo | null;
  status: TorStatus;
  onSaved: () => void;
  onIdentityChanged: (identity: IdentityInfo) => void;
}

export default function IdentityPanel({
  identity,
  status,
  onSaved,
  onIdentityChanged,
}: Props) {
  const { t } = useI18n();
  const [name, setName] = useState("");
  const [saving, setSaving] = useState(false);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [phrase, setPhrase] = useState<string[] | null>(null);
  const [confirmed, setConfirmed] = useState(true);
  const [restoreInput, setRestoreInput] = useState("");
  const [restoring, setRestoring] = useState(false);
  const [bio, setBio] = useState("");
  const [statusText, setStatusText] = useState("");
  const [accent, setAccent] = useState("");
  const [hasAvatar, setHasAvatar] = useState(false);
  const [phraseCopied, setPhraseCopied] = useState(false);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    getOwnProfile()
      .then((profile: OwnProfileInfo) => {
        setBio(profile.bio);
        setStatusText(profile.status);
        setAccent(profile.accent);
        setHasAvatar(profile.hasAvatar);
      })
      .catch(() => undefined);
  }, []);

  const ACCENTS = ["", "#f5f5f5", "#e0e0e0", "#cfcfcf", "#b8b8b8", "#a0a0a0", "#9ecfff"];

  const pickAvatar = async (file: File) => {
    setError(null);
    try {
      const dataUrl = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(reader.result as string);
        reader.onerror = () => reject(reader.error);
        reader.readAsDataURL(file);
      });
      const img = new Image();
      await new Promise<void>((resolve, reject) => {
        img.onload = () => resolve();
        img.onerror = () => reject(new Error("invalid"));
        img.src = dataUrl;
      });
      const size = 256;
      const canvas = document.createElement("canvas");
      canvas.width = size;
      canvas.height = size;
      const context = canvas.getContext("2d");
      if (!context) {
        setError(t("identity.avatarInvalid"));
        return;
      }
      context.fillStyle = "#1d1d1d";
      context.fillRect(0, 0, size, size);
      const scale = Math.max(size / img.width, size / img.height);
      const w = img.width * scale;
      const h = img.height * scale;
      context.drawImage(img, (size - w) / 2, (size - h) / 2, w, h);
      let base64 = "";
      for (const quality of [0.85, 0.6, 0.4, 0.2, 0.1]) {
        const jpegUrl = canvas.toDataURL("image/jpeg", quality);
        base64 = jpegUrl.split(",")[1] ?? "";
        if (base64.length < 85_000) break;
      }
      if (base64.length >= 85_000) {
        setError(t("identity.avatarTooLarge"));
        return;
      }
      await setProfile({ avatarB64: base64 });
      invalidateAvatarCache(null);
      setHasAvatar(true);
    } catch {
      setError(t("identity.avatarInvalid"));
    }
  };

  const removeAvatar = async () => {
    try {
      await setProfile({ avatarB64: "" });
      invalidateAvatarCache(null);
      setHasAvatar(false);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    if (identity) {
      setName((current) => current || identity.displayName);
    }
  }, [identity]);

  useEffect(() => {
    isRecoveryConfirmed().then(setConfirmed).catch(() => undefined);
  }, [identity?.onion]);

  const copyOnion = async () => {
    if (!identity) return;
    await writeText(identity.onion);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await setProfile({
        displayName: name.trim(),
        bio,
        status: statusText,
        accent,
      });
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const reveal = async () => {
    try {
      const recovered = await getRecoveryPhrase();
      setPhrase(recovered.split(" "));
    } catch (e) {
      setError(String(e));
    }
  };

  const confirmPhrase = async () => {
    try {
      await confirmRecoveryPhrase();
      setConfirmed(true);
    } catch (e) {
      setError(String(e));
    }
  };

  const restore = async () => {
    if (!window.confirm(t("identity.restoreConfirm"))) {
      return;
    }
    setRestoring(true);
    setError(null);
    try {
      const info = await restoreFromPhrase(restoreInput.trim());
      setPhrase(null);
      setRestoreInput("");
      onIdentityChanged(info);
    } catch (e) {
      setError(String(e));
    } finally {
      setRestoring(false);
    }
  };

  return (
    <div className="relative z-10 flex-1 overflow-y-auto p-10">
      <div className="mx-auto max-w-xl space-y-5">
        <header className="animate-fade-up">
          <h1 className="font-display text-2xl font-bold tracking-tight text-mist-1">
            {t("identity.title")}
          </h1>
          <p className="mt-2 text-[15px] leading-relaxed text-mist-2">
            {t("identity.subtitle")}
          </p>
        </header>

        <section
          className="rounded-2xl border border-white/[0.06] bg-void-4/70 p-5 animate-fade-up"
          style={{ animationDelay: "50ms" }}
        >
          <div className="flex items-center gap-4">
            <Avatar onionId={null} name={name || "V"} size={64} />
            <div className="flex flex-col gap-1.5">
              <button
                onClick={() => fileInputRef.current?.click()}
                className="btn-press rounded-xl bg-void-2 px-3.5 py-1.5 text-[13px] font-medium text-mist-1 hover:bg-void-5"
              >
                {t("identity.changeAvatar")}
              </button>
              {hasAvatar && (
                <button
                  onClick={() => void removeAvatar()}
                  className="btn-press self-start rounded-lg px-2 py-1 text-[12px] text-mist-3 hover:text-alerte"
                >
                  {t("identity.removeAvatar")}
                </button>
              )}
              <input
                ref={fileInputRef}
                type="file"
                accept="image/*"
                className="hidden"
                onChange={(event) => {
                  const file = event.target.files?.[0];
                  event.target.value = "";
                  if (file) void pickAvatar(file);
                }}
              />
            </div>
          </div>
          <div className="mt-4 flex gap-2">
            <input
              value={name}
              onChange={(event) => setName(event.target.value)}
              maxLength={32}
              placeholder={t("identity.displayNamePlaceholder")}
              className="focus-glow w-full rounded-xl border border-white/[0.07] bg-void-2 px-3.5 py-2.5 text-[15px] text-mist-1 outline-none placeholder:text-mist-3"
            />
            <button
              onClick={save}
              disabled={saving || !name.trim()}
              className="btn-press focus-glow shrink-0 rounded-xl bg-nebula px-4 py-2 text-[14px] font-semibold text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi disabled:opacity-50 disabled:shadow-none"
            >
              {t("identity.save")}
            </button>
          </div>
          {error && <p className="mt-2 text-[13px] text-alerte">{error}</p>}
          <div className="mt-3 grid grid-cols-2 gap-2">
            <input
              value={statusText}
              onChange={(event) => setStatusText(event.target.value)}
              maxLength={64}
              placeholder={t("identity.statusPlaceholder")}
              className="focus-glow rounded-xl border border-white/[0.07] bg-void-2 px-3.5 py-2 text-[14px] text-mist-1 outline-none placeholder:text-mist-3"
            />
            <input
              value={bio}
              onChange={(event) => setBio(event.target.value)}
              maxLength={200}
              placeholder={t("identity.bioPlaceholder")}
              className="focus-glow rounded-xl border border-white/[0.07] bg-void-2 px-3.5 py-2 text-[14px] text-mist-1 outline-none placeholder:text-mist-3"
            />
          </div>
          <div className="mt-3">
            <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
              {t("identity.accent")}
            </p>
            <div className="mt-2 flex gap-2">
              {ACCENTS.map((color) => (
                <button
                  key={color || "none"}
                  onClick={() => setAccent(color)}
                  className={`btn-press h-7 w-7 rounded-full border ${
                    accent === color ? "border-white" : "border-white/15"
                  }`}
                  style={{
                    backgroundColor: color || "transparent",
                    backgroundImage: color
                      ? undefined
                      : "linear-gradient(45deg, #2a2a2a 25%, transparent 25%, transparent 75%, #2a2a2a 75%)",
                    backgroundSize: color ? undefined : "8px 8px",
                  }}
                  title={color || "—"}
                  aria-label={color || "none"}
                />
              ))}
            </div>
          </div>
        </section>

        <section
          className="rounded-2xl border border-white/[0.06] bg-void-4/70 p-5 animate-fade-up"
          style={{ animationDelay: "100ms" }}
        >
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
              {t("identity.onionAddress")}
            </span>
            <button
              onClick={copyOnion}
              disabled={!identity}
              className="btn-press rounded-lg bg-void-2 px-3 py-1 text-[13px] font-medium text-mist-1 hover:bg-void-5 disabled:opacity-50"
            >
              {copied ? t("common.copied") : t("common.copy")}
            </button>
          </div>
          <p className="mt-2.5 break-all rounded-xl bg-void-2 px-3.5 py-2.5 font-mono text-[14px] leading-relaxed text-nova/90">
            {identity?.onion ?? t("identity.onionGenerating")}
          </p>
          {identity && (
            <p className="mt-2 font-mono text-[12px] text-mist-3">
              {t("identity.fingerprint", { fp: identity.fingerprint })}
            </p>
          )}
        </section>

        <section
          className="rounded-2xl border border-white/[0.06] bg-void-4/70 p-5 animate-fade-up"
          style={{ animationDelay: "150ms" }}
        >
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
              {t("identity.recovery")}
            </span>
            {confirmed && (
              <span className="font-display text-[10px] font-bold uppercase tracking-[0.12em] text-nova">
                {t("identity.saved")}
              </span>
            )}
          </div>
          <p className="mt-2.5 text-[13px] leading-relaxed text-mist-2">
            {t("identity.recoveryDesc")}
          </p>
          {phrase ? (
            <div className="animate-fade-up">
              <div className="mt-3 grid grid-cols-4 gap-1.5 rounded-xl bg-void-2 p-3.5">
                {phrase.map((word, index) => (
                  <span key={index} className="text-[13px] text-mist-1">
                    <span className="mr-1 font-display text-[11px] text-mist-3">
                      {index + 1}.
                    </span>
                    {word}
                  </span>
                ))}
              </div>
              <div className="mt-2 flex items-center gap-2">
                <p className="flex-1 text-[12px] text-ambre">
                  {t("identity.paperWarning")}
                </p>
                <button
                  onClick={async () => {
                    await writeText(phrase.join(" "));
                    setPhraseCopied(true);
                    setTimeout(() => setPhraseCopied(false), 1500);
                  }}
                  className="btn-press shrink-0 rounded-lg bg-void-2 px-3 py-1 text-[12px] font-medium text-mist-1 hover:bg-void-5"
                >
                  {phraseCopied ? t("common.copied") : t("common.copy")}
                </button>
              </div>
              {!confirmed && (
                <button
                  onClick={confirmPhrase}
                  className="btn-press focus-glow mt-3 w-full rounded-xl bg-nebula px-4 py-2.5 text-[14px] font-semibold text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi"
                >
                  {t("identity.confirmed")}
                </button>
              )}
            </div>
          ) : (
            <button
              onClick={reveal}
              className="btn-press mt-3 rounded-xl bg-void-2 px-4 py-2 text-[14px] font-medium text-mist-1 hover:bg-void-5"
            >
              {t("identity.showPhrase")}
            </button>
          )}
        </section>

        <section
          className="rounded-2xl border border-white/[0.06] bg-void-4/70 p-5 animate-fade-up"
          style={{ animationDelay: "200ms" }}
        >
          <span className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
            {t("identity.restore")}
          </span>
          <p className="mt-2.5 text-[13px] leading-relaxed text-mist-2">
            {t("identity.restoreDesc")}
          </p>
          <textarea
            value={restoreInput}
            onChange={(event) => setRestoreInput(event.target.value)}
            rows={3}
            placeholder={t("identity.restorePlaceholder")}
            className="focus-glow mt-3 w-full rounded-xl border border-white/[0.07] bg-void-2 px-3.5 py-2.5 text-[14px] text-mist-1 outline-none placeholder:text-mist-3"
          />
          <button
            onClick={restore}
            disabled={restoring || restoreInput.trim().split(/\s+/).length < 24}
            className="btn-press mt-3 rounded-xl bg-ambre/90 px-4 py-2 text-[14px] font-bold text-void-0 hover:bg-ambre disabled:opacity-50"
          >
            {restoring ? t("identity.restoring") : t("identity.restoreButton")}
          </button>
        </section>

        <div style={{ animationDelay: "250ms" }} className="animate-fade-up">
          <TorStatusCard status={status} />
        </div>
      </div>
    </div>
  );
}
