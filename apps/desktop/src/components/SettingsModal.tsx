import { useEffect, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { AppInfo, Settings } from "../types";
import type { UpdateState } from "../App";
import { getAppInfo } from "../lib/void";
import { autostartEnabled, setAutostart } from "../lib/notify";
import { checkForUpdates } from "../lib/update";
import type { Update } from "@tauri-apps/plugin-updater";
import Modal from "./Modal";
import { useI18n, type Locale } from "../lib/i18n";
import {
  getPreferredMic,
  setPreferredMic,
  getPreferredSpeaker,
  setPreferredSpeaker,
} from "../lib/audio";

interface Props {
  settings: Settings;
  onSettingsChanged: (settings: Settings) => void;
  onClose: () => void;
  updateState: UpdateState;
  setUpdateState: (state: UpdateState) => void;
}

export default function SettingsModal({
  settings,
  onSettingsChanged,
  onClose,
  updateState,
  setUpdateState,
}: Props) {
  const { t, locale, setLocale } = useI18n();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [autostart, setAutostartState] = useState(false);
  const [copied, setCopied] = useState(false);
  const [busy, setBusy] = useState(false);
  const [mics, setMics] = useState<{ deviceId: string; label: string }[]>([]);
  const [speakers, setSpeakers] = useState<{ deviceId: string; label: string }[]>(
    []
  );

  useEffect(() => {
    getAppInfo().then(setInfo).catch(() => undefined);
    autostartEnabled().then(setAutostartState).catch(() => undefined);
    const media = navigator.mediaDevices;
    if (!media?.enumerateDevices) return;
    // Returns true once device labels are visible (only after mic permission).
    const loadDevices = async (): Promise<boolean> => {
      const devices = await media.enumerateDevices();
      const inputs = devices.filter((d) => d.kind === "audioinput");
      const outputs = devices.filter((d) => d.kind === "audiooutput");
      setMics(
        inputs.map((d, i) => ({
          deviceId: d.deviceId,
          label: d.label || `${t("settings.microphone")} ${i + 1}`,
        }))
      );
      setSpeakers(
        outputs.map((d, i) => ({
          deviceId: d.deviceId,
          label: d.label || `${t("settings.speaker")} ${i + 1}`,
        }))
      );
      return [...inputs, ...outputs].some((d) => d.label !== "");
    };
    void (async () => {
      const hasLabels = await loadDevices();
      // Labels stay hidden until mic permission is granted; grab it once (the
      // WebView auto-grants, so no prompt) to reveal real device names.
      if (!hasLabels && media.getUserMedia) {
        try {
          const stream = await media.getUserMedia({ audio: true });
          stream.getTracks().forEach((track) => track.stop());
          await loadDevices();
        } catch {
          // permission denied — keep the generic labels
        }
      }
    })().catch(() => undefined);
  }, [t]);

  const toggleAutostart = async () => {
    setBusy(true);
    try {
      await setAutostart(!autostart);
      setAutostartState(!autostart);
    } catch {
      void 0;
    } finally {
      setBusy(false);
    }
  };

  const checkUpdates = async () => {
    setUpdateState({ phase: "checking", progress: 0, version: null, error: null });
    try {
      const update = await checkForUpdates();
      if (update) {
        setUpdateState({
          phase: "available",
          progress: 0,
          version: update.version,
          error: null,
        });
      } else {
        setUpdateState({ phase: "none", progress: 0, version: null, error: null });
      }
    } catch {
      setUpdateState({
        phase: "error",
        progress: 0,
        version: null,
        error: t("settings.updateError"),
      });
    }
  };

  const installUpdate = async (update: Update) => {
    setUpdateState({
      phase: "downloading",
      progress: 0,
      version: update.version,
      error: null,
    });
    try {
      let contentLength = 0;
      let downloaded = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            break;
          case "Progress": {
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              setUpdateState({
                phase: "downloading",
                progress: downloaded / contentLength,
                version: update.version,
                error: null,
              });
            }
            break;
          }
          case "Finished":
            setUpdateState({
              phase: "installing",
              progress: 1,
              version: update.version,
              error: null,
            });
            break;
        }
      });
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch {
      setUpdateState({
        phase: "error",
        progress: 0,
        version: null,
        error: t("settings.installError"),
      });
    }
  };

  const switchLocale = (next: Locale) => {
    setLocale(next);
  };

  const Toggle = ({
    checked,
    disabled,
    onChange,
    label,
    hint,
  }: {
    checked: boolean;
    disabled?: boolean;
    onChange: () => void;
    label: string;
    hint: string;
  }) => (
    <div className="flex items-center justify-between gap-4 rounded-xl border border-white/[0.06] bg-void-2 p-4">
      <div>
        <p className="text-[15px] font-medium text-mist-1">{label}</p>
        <p className="mt-0.5 text-[13px] leading-relaxed text-mist-3">{hint}</p>
      </div>
      <button
        onClick={onChange}
        disabled={disabled}
        className={`btn-press relative h-6 w-11 shrink-0 rounded-full transition-colors ${
          checked ? "bg-nebula shadow-lg shadow-nebula/30" : "bg-void-6"
        } disabled:opacity-50`}
        aria-label={label}
      >
        <span
          className={`absolute top-1 h-4 w-4 rounded-full shadow transition-all duration-200 ${
            checked ? "left-6 bg-void-0" : "left-1 bg-mist-1"
          }`}
        />
      </button>
    </div>
  );

  const isUpdating =
    updateState.phase === "downloading" || updateState.phase === "installing";

  return (
    <Modal title={t("settings.title")} onClose={onClose} width={560}>
      <div className="space-y-3">
        <div className="flex items-center justify-between gap-4 rounded-xl border border-white/[0.06] bg-void-2 p-4">
          <div>
            <p className="text-[15px] font-medium text-mist-1">
              {t("settings.language")}
            </p>
            <p className="mt-0.5 text-[13px] text-mist-3">English · Français</p>
          </div>
          <div className="flex gap-1 rounded-xl bg-void-4 p-1">
            {(["en", "fr"] as Locale[]).map((lang) => (
              <button
                key={lang}
                onClick={() => switchLocale(lang)}
                className={`btn-press rounded-lg px-3.5 py-1.5 font-display text-[12px] font-bold uppercase tracking-wider transition-colors ${
                  locale === lang
                    ? "bg-nebula text-void-0"
                    : "text-mist-3 hover:text-mist-1"
                }`}
              >
                {lang}
              </button>
            ))}
          </div>
        </div>

        <Toggle
          checked={settings.notificationsEnabled}
          onChange={() =>
            onSettingsChanged({
              ...settings,
              notificationsEnabled: !settings.notificationsEnabled,
            })
          }
          label={t("settings.notifications")}
          hint={t("settings.notificationsHint")}
        />
        <Toggle
          checked={autostart}
          disabled={busy}
          onChange={() => void toggleAutostart()}
          label={t("settings.autostart")}
          hint={t("settings.autostartHint")}
        />

        {(mics.length > 0 || speakers.length > 0) && (
          <div className="space-y-3 rounded-xl border border-white/[0.06] bg-void-2 p-4">
            {mics.length > 0 && (
              <div>
                <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
                  {t("settings.microphone")}
                </p>
                <select
                  className="focus-glow mt-2 w-full rounded-lg border border-white/[0.07] bg-void-4 px-3 py-2 text-[14px] text-mist-1 outline-none"
                  onChange={(event) => setPreferredMic(event.target.value)}
                  defaultValue={getPreferredMic() ?? ""}
                >
                  <option value="">{t("settings.microphoneDefault")}</option>
                  {mics.map((mic) => (
                    <option key={mic.deviceId} value={mic.deviceId}>
                      {mic.label}
                    </option>
                  ))}
                </select>
              </div>
            )}
            {speakers.length > 0 && (
              <div>
                <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
                  {t("settings.speaker")}
                </p>
                <select
                  className="focus-glow mt-2 w-full rounded-lg border border-white/[0.07] bg-void-4 px-3 py-2 text-[14px] text-mist-1 outline-none"
                  onChange={(event) => setPreferredSpeaker(event.target.value)}
                  defaultValue={getPreferredSpeaker() ?? ""}
                >
                  <option value="">{t("settings.speakerDefault")}</option>
                  {speakers.map((speaker) => (
                    <option key={speaker.deviceId} value={speaker.deviceId}>
                      {speaker.label}
                    </option>
                  ))}
                </select>
              </div>
            )}
          </div>
        )}

        <div className="rounded-xl border border-white/[0.06] bg-void-2 p-4">
          <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
            {t("settings.update")}
          </p>
          <div className="mt-3 flex items-center justify-between gap-4">
            <p className="text-[13px] leading-relaxed text-mist-2">
              {updateState.phase === "available" && (
                <span className="text-mist-1">
                  {t("settings.updateAvailable", {
                    version: updateState.version ?? "?",
                  })}
                </span>
              )}
              {updateState.phase === "none" &&
                t("settings.upToDate", { version: info?.version ?? "…" })}
              {updateState.phase === "checking" && t("settings.checking")}
              {updateState.phase === "downloading" && (
                <span className="flex items-center gap-2 text-mist-1">
                  {t("update.downloading", {
                    percent: Math.round(updateState.progress * 100),
                  })}
                </span>
              )}
              {updateState.phase === "installing" && t("update.installing")}
              {updateState.phase === "error" && (
                <span className="text-alerte">{updateState.error}</span>
              )}
              {(updateState.phase === "idle" || updateState.phase === "error") &&
                t("settings.checkGithub")}
            </p>
            {!isUpdating && updateState.phase !== "available" && (
              <button
                onClick={() => void checkUpdates()}
                disabled={updateState.phase === "checking"}
                className="btn-press focus-glow shrink-0 rounded-xl bg-nebula px-3.5 py-2 font-display text-[12px] font-bold uppercase tracking-wider text-void-0 shadow-lg shadow-nebula/20 hover:bg-nebula-hi disabled:opacity-50 disabled:shadow-none"
              >
                {updateState.phase === "checking" ? "…" : t("settings.check")}
              </button>
            )}
          </div>
          {updateState.phase === "downloading" && (
            <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-void-4">
              <div
                className="h-full rounded-full bg-nebula transition-all duration-300"
                style={{ width: `${updateState.progress * 100}%` }}
              />
            </div>
          )}
          {updateState.phase === "available" && updateState.version && (
            <button
              onClick={async () => {
                setBusy(true);
                try {
                  const update = await checkForUpdates();
                  if (update) await installUpdate(update);
                } finally {
                  setBusy(false);
                }
              }}
              disabled={busy}
              className="btn-press focus-glow mt-3 w-full rounded-xl bg-nebula px-4 py-2.5 font-display text-[13px] font-bold uppercase tracking-wider text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi disabled:opacity-50"
            >
              {t("settings.installRestart")}
            </button>
          )}
          <p className="mt-2 text-[11px] leading-relaxed text-mist-3">
            {t("settings.manualNote")}
          </p>
        </div>

        <div className="rounded-xl border border-white/[0.06] bg-void-2 p-4">
          <p className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
            {t("settings.about")}
          </p>
          <div className="mt-3 space-y-2 text-[13px]">
            <div className="flex justify-between gap-4">
              <span className="text-mist-3">{t("settings.version")}</span>
              <span className="font-display font-bold text-mist-1">
                Void {info?.version ?? "…"}
              </span>
            </div>
            <div className="flex justify-between gap-4">
              <span className="text-mist-3">{t("settings.transport")}</span>
              <span className="text-mist-1">{t("settings.transportValue")}</span>
            </div>
            <div className="flex justify-between gap-4">
              <span className="text-mist-3">{t("settings.relayQueue")}</span>
              <span className="font-display font-bold text-mist-1">
                {info
                  ? t("settings.relayQueueValue", { n: info.relayQueue })
                  : "…"}
              </span>
            </div>
            <div className="flex items-center justify-between gap-4">
              <span className="text-mist-3">{t("settings.data")}</span>
              <button
                onClick={async () => {
                  if (!info) return;
                  await writeText(info.dataDir);
                  setCopied(true);
                  setTimeout(() => setCopied(false), 1500);
                }}
                className="btn-press max-w-[300px] truncate rounded-lg bg-void-4 px-2.5 py-1 font-mono text-[12px] text-mist-1 hover:bg-void-5"
                title={info?.dataDir}
              >
                {copied ? t("common.copied") : info?.dataDir || "…"}
              </button>
            </div>
          </div>
        </div>
      </div>
    </Modal>
  );
}
