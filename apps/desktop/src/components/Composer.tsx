import { useEffect, useRef, useState, type ReactNode } from "react";
import { useI18n } from "../lib/i18n";
import { getPreferredMic, applyAudioSink } from "../lib/audio";

interface Props {
  placeholder: string;
  disabled: boolean;
  disabledPlaceholder?: string;
  onSubmit: (text: string) => Promise<void>;
  onVoice: (base64: string, durationMs: number) => Promise<void>;
  footnote?: ReactNode;
}

type RecorderMode = "idle" | "recording" | "paused" | "preview";

const VOICE_MAX_MS = 60_000;

function formatClock(ms: number) {
  const total = Math.floor(ms / 1000);
  return `${String(Math.floor(total / 60)).padStart(2, "0")}:${String(total % 60).padStart(2, "0")}`;
}

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const buffer = reader.result as ArrayBuffer;
      const bytes = new Uint8Array(buffer);
      let binary = "";
      const chunk = 0x8000;
      for (let i = 0; i < bytes.length; i += chunk) {
        binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
      }
      resolve(btoa(binary));
    };
    reader.onerror = () => reject(reader.error);
    reader.readAsArrayBuffer(blob);
  });
}

export default function Composer({
  placeholder,
  disabled,
  disabledPlaceholder,
  onSubmit,
  onVoice,
  footnote,
}: Props) {
  const { t } = useI18n();
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mode, setMode] = useState<RecorderMode>("idle");
  const [elapsedMs, setElapsedMs] = useState(0);
  const [finalMs, setFinalMs] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [playProgress, setPlayProgress] = useState(0);

  const recorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const elapsedRef = useRef(0);
  const segmentStartRef = useRef(0);
  const timerRef = useRef<number | null>(null);
  const cancelledRef = useRef(false);
  const previewUrlRef = useRef<string | null>(null);
  const audioElRef = useRef<HTMLAudioElement | null>(null);

  const micSupported =
    typeof navigator !== "undefined" &&
    !!navigator.mediaDevices?.getUserMedia &&
    typeof window.MediaRecorder !== "undefined";

  useEffect(() => {
    return () => {
      if (timerRef.current !== null) window.clearInterval(timerRef.current);
      if (audioElRef.current) audioElRef.current.pause();
      if (previewUrlRef.current) URL.revokeObjectURL(previewUrlRef.current);
    };
  }, []);

  const stopTimer = () => {
    if (timerRef.current !== null) {
      window.clearInterval(timerRef.current);
      timerRef.current = null;
    }
  };

  const resetToIdle = () => {
    stopTimer();
    if (audioElRef.current) {
      audioElRef.current.pause();
    }
    if (previewUrlRef.current) {
      URL.revokeObjectURL(previewUrlRef.current);
      previewUrlRef.current = null;
    }
    setMode("idle");
    setElapsedMs(0);
    setFinalMs(0);
    setPlaying(false);
    setPlayProgress(0);
  };

  const startRecording = async () => {
    if (mode !== "idle" || !micSupported) return;
    setError(null);
    try {
      const constraints: MediaStreamConstraints = { audio: true };
      const micId = getPreferredMic();
      if (micId) {
        constraints.audio = { deviceId: { ideal: micId } };
      }
      const stream = await navigator.mediaDevices.getUserMedia(constraints);
      const mimeType = ["audio/webm;codecs=opus", "audio/webm"].find((type) =>
        MediaRecorder.isTypeSupported(type)
      );
      const recorder = new MediaRecorder(
        stream,
        mimeType ? { mimeType, audioBitsPerSecond: 16000 } : undefined
      );
      chunksRef.current = [];
      cancelledRef.current = false;
      elapsedRef.current = 0;
      recorder.ondataavailable = (event) => {
        if (event.data.size > 0) chunksRef.current.push(event.data);
      };
      recorder.onstop = () => {
        stream.getTracks().forEach((track) => track.stop());
        stopTimer();
        if (cancelledRef.current) {
          setMode("idle");
          setElapsedMs(0);
          return;
        }
        const duration = elapsedRef.current;
        const blob = new Blob(chunksRef.current, {
          type: recorder.mimeType || "audio/webm",
        });
        if (blob.size === 0 || duration < 400) {
          setMode("idle");
          setElapsedMs(0);
          return;
        }
        previewUrlRef.current = URL.createObjectURL(blob);
        setFinalMs(Math.min(duration, VOICE_MAX_MS));
        setMode("preview");
      };
      recorderRef.current = recorder;
      segmentStartRef.current = Date.now();
      recorder.start(250);
      setMode("recording");
      timerRef.current = window.setInterval(() => {
        const total = elapsedRef.current + (Date.now() - segmentStartRef.current);
        setElapsedMs(total);
        if (total >= VOICE_MAX_MS && recorderRef.current) {
          elapsedRef.current = total;
          recorderRef.current.stop();
        }
      }, 200);
    } catch {
      setError(t("composer.micDenied"));
    }
  };

  const pauseRecording = () => {
    if (mode !== "recording" || !recorderRef.current) return;
    elapsedRef.current += Date.now() - segmentStartRef.current;
    recorderRef.current.pause();
    setMode("paused");
  };

  const resumeRecording = () => {
    if (mode !== "paused" || !recorderRef.current) return;
    segmentStartRef.current = Date.now();
    recorderRef.current.resume();
    setMode("recording");
  };

  const stopRecording = () => {
    if (mode !== "recording" && mode !== "paused") return;
    if (mode === "recording") {
      elapsedRef.current += Date.now() - segmentStartRef.current;
    }
    recorderRef.current?.stop();
  };

  const cancelAll = () => {
    cancelledRef.current = true;
    recorderRef.current?.stop();
    resetToIdle();
  };

  const sendPreview = async () => {
    if (mode !== "preview" || !previewUrlRef.current || sending) return;
    setSending(true);
    setError(null);
    try {
      const response = await fetch(previewUrlRef.current);
      const blob = await response.blob();
      const base64 = await blobToBase64(blob);
      await onVoice(base64, finalMs);
      resetToIdle();
    } catch (e) {
      setError(String(e));
    } finally {
      setSending(false);
    }
  };

  const togglePreviewPlayback = () => {
    const el = audioElRef.current;
    if (!el) return;
    if (!el.paused) {
      el.pause();
      setPlaying(false);
    } else {
      el.currentTime = 0;
      void applyAudioSink(el).finally(() => {
        el
          .play()
          .then(() => setPlaying(true))
          .catch(() => setPlaying(false));
      });
    }
  };

  const submit = async () => {
    const trimmed = text.trim();
    if (!trimmed || sending || disabled) return;
    setSending(true);
    setError(null);
    try {
      await onSubmit(trimmed);
      setText("");
    } catch (e) {
      setError(String(e));
    } finally {
      setSending(false);
    }
  };

  if (mode !== "idle") {
    const isRecording = mode === "recording";
    const isPaused = mode === "paused";
    const isPreview = mode === "preview";
    return (
      <div className="shrink-0 px-6 pb-6">
        {error && <p className="pb-2 text-[13px] text-alerte">{error}</p>}
        <div className="flex items-center gap-3 rounded-2xl border border-white/[0.08] bg-void-4/80 px-4 py-3 animate-fade-up">
          {isPreview ? (
            <button
              onClick={togglePreviewPlayback}
              className="btn-press focus-glow flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-nebula text-void-0 shadow-lg shadow-nebula/20 hover:bg-nebula-hi"
              aria-label={playing ? "pause" : "play"}
            >
              {playing ? (
                <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
                  <rect x="1.5" y="1" width="3" height="10" rx="1" />
                  <rect x="7.5" y="1" width="3" height="10" rx="1" />
                </svg>
              ) : (
                <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
                  <path d="M2.5 1l8 5-8 5z" />
                </svg>
              )}
            </button>
          ) : (
            <span
              className={`h-2.5 w-2.5 shrink-0 rounded-full ${
                isRecording
                  ? "bg-alerte text-alerte animate-glow-pulse"
                  : "bg-ambre text-ambre"
              }`}
            />
          )}

          <span className="font-display text-[14px] font-bold text-mist-1 tabular-nums">
            {isPreview
              ? formatClock(finalMs)
              : `${formatClock(elapsedMs)} / 00:60`}
          </span>

          {isPreview && (
            <>
              <audio
                ref={audioElRef}
                src={previewUrlRef.current ?? undefined}
                onEnded={() => {
                  setPlaying(false);
                  setPlayProgress(0);
                }}
                onTimeUpdate={(e) => {
                  const el = e.currentTarget;
                  if (finalMs > 0) {
                    setPlayProgress(
                      Math.min(1, el.currentTime / (finalMs / 1000))
                    );
                  }
                }}
                className="hidden"
              />
              <div className="hidden min-w-0 flex-1 items-center gap-1.5 sm:flex">
                {Array.from({ length: 24 }).map((_, index) => (
                  <span
                    key={index}
                    className={`w-[3px] rounded-full transition-colors duration-150 ${
                      index / 24 <= playProgress ? "bg-nebula-hi" : "bg-white/15"
                    }`}
                    style={{ height: `${4 + ((index * 7919) % 13)}px` }}
                  />
                ))}
              </div>
            </>
          )}
          {isPreview && <span className="flex-1" />}

          <div className="ml-auto flex items-center gap-2">
            {(isRecording || isPaused) && (
              <button
                onClick={() => (isRecording ? pauseRecording() : resumeRecording())}
                title={isRecording ? t("composer.pause") : t("composer.resume")}
                className="btn-press flex h-9 w-9 items-center justify-center rounded-xl bg-void-2 text-mist-1 hover:bg-void-5"
                aria-label={isRecording ? t("composer.pause") : t("composer.resume")}
              >
                {isRecording ? (
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
                    <rect x="1.5" y="1" width="3" height="10" rx="1" />
                    <rect x="7.5" y="1" width="3" height="10" rx="1" />
                  </svg>
                ) : (
                  <svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor">
                    <path d="M2.5 1l8 5-8 5z" />
                  </svg>
                )}
              </button>
            )}
            {(isRecording || isPaused) && (
              <button
                onClick={stopRecording}
                title={t("composer.stop")}
                className="btn-press flex h-9 w-9 items-center justify-center rounded-xl bg-void-2 text-mist-1 hover:bg-void-5"
                aria-label={t("composer.stop")}
              >
                <svg width="11" height="11" viewBox="0 0 12 12" fill="currentColor">
                  <rect x="1.5" y="1.5" width="9" height="9" rx="1.5" />
                </svg>
              </button>
            )}
            <button
              onClick={cancelAll}
              title={t("common.cancel")}
              className="btn-press flex h-9 w-9 items-center justify-center rounded-xl bg-void-2 text-mist-2 hover:text-alerte"
              aria-label={t("common.cancel")}
            >
              ✕
            </button>
            {isPreview && (
              <button
                onClick={() => void sendPreview()}
                disabled={sending}
                className="btn-press focus-glow rounded-xl bg-nebula px-4 py-2 font-display text-[12px] font-bold uppercase tracking-wider text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi disabled:opacity-50"
              >
                {sending ? "…" : t("composer.send")}
              </button>
            )}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="shrink-0 px-6 pb-6">
      {error && <p className="pb-2 text-[13px] text-alerte animate-fade-up">{error}</p>}
      <div className="composer-glow flex items-end gap-2 rounded-2xl border border-white/[0.07] bg-void-4/80">
        <textarea
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey) {
              event.preventDefault();
              void submit();
            }
          }}
          rows={1}
          maxLength={4000}
          disabled={disabled}
          placeholder={disabled ? (disabledPlaceholder ?? "…") : placeholder}
          className="max-h-40 min-h-[48px] flex-1 resize-none bg-transparent px-4 py-3 text-[15px] leading-relaxed text-mist-1 outline-none placeholder:text-mist-3 disabled:cursor-not-allowed"
        />
        {micSupported && !disabled && (
          <button
            onClick={() => void startRecording()}
            disabled={sending}
            title={t("composer.record")}
            className="btn-press my-2.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-void-2 text-mist-2 hover:bg-void-5 hover:text-alerte disabled:opacity-40"
            aria-label={t("composer.record")}
          >
            <svg width="15" height="15" viewBox="0 0 16 16" fill="currentColor">
              <rect x="5.5" y="1.5" width="5" height="8" rx="2.5" />
              <path
                d="M3 7.5a5 5 0 0 0 10 0"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.4"
                strokeLinecap="round"
              />
              <rect x="7.4" y="12" width="1.2" height="2.5" rx="0.6" />
            </svg>
          </button>
        )}
        <button
          onClick={() => void submit()}
          disabled={disabled || sending || !text.trim()}
          className="btn-press focus-glow my-2.5 mr-2.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-nebula text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi disabled:opacity-40 disabled:shadow-none"
          aria-label={t("composer.send")}
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
            <path d="M1 8l14-6-4 6 4 6z" />
          </svg>
        </button>
      </div>
      {footnote && <p className="px-1 pt-1.5 text-[11px] text-mist-3">{footnote}</p>}
    </div>
  );
}
