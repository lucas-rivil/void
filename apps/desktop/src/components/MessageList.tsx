import { useMemo, useEffect, useRef, useState } from "react";
import type { DmMessage, PeerProfileInfo } from "../types";
import { useI18n, dateLocale } from "../lib/i18n";
import { getVoiceBlob } from "../lib/void";
import { applyAudioSink } from "../lib/audio";
import Avatar from "./Avatar";

export interface MessageAuthor {
  name: string;
  mine: boolean;
  color: string;
}

interface Props {
  messages: DmMessage[];
  resolveAuthor: (authorId: string) => MessageAuthor;
  showStatus: boolean;
  emptyTitle: string;
  emptyHint: string;
  peerProfiles?: PeerProfileInfo[];
  onOpenProfile?: (onionId: string) => void;
}

interface Group {
  key: string;
  mine: boolean;
  items: DmMessage[];
}

const GROUP_GAP_MS = 5 * 60 * 1000;

function sameDay(a: number, b: number) {
  const da = new Date(a);
  const db = new Date(b);
  return (
    da.getFullYear() === db.getFullYear() &&
    da.getMonth() === db.getMonth() &&
    da.getDate() === db.getDate()
  );
}

export function formatTime(ms: number, locale: string) {
  const value = Number(ms);
  if (!Number.isFinite(value) || value <= 0) return "";
  return new Date(value).toLocaleTimeString(locale, {
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatDay(ms: number, locale: string, todayLabel: string, yesterdayLabel: string) {
  const value = Number(ms);
  if (!Number.isFinite(value) || value <= 0) return "";
  const date = new Date(value);
  const today = new Date();
  const yesterday = new Date(today.getTime() - 86400000);
  if (sameDay(value, today.getTime())) return todayLabel;
  if (sameDay(value, yesterday.getTime())) return yesterdayLabel;
  return date.toLocaleDateString(locale, {
    weekday: "long",
    day: "numeric",
    month: "long",
  });
}

function EmptyOrbit() {
  return (
    <svg
      width="120"
      height="72"
      viewBox="0 0 120 72"
      fill="none"
      className="mx-auto mb-4 text-mist-3/60"
    >
      <ellipse cx="60" cy="36" rx="52" ry="20" stroke="currentColor" strokeOpacity="0.4" strokeDasharray="3 6" />
      <ellipse cx="60" cy="36" rx="30" ry="12" stroke="currentColor" strokeOpacity="0.6" />
      <circle cx="60" cy="36" r="7" fill="currentColor" fillOpacity="0.5" />
      <circle cx="112" cy="36" r="2.5" fill="#f5f5f5" />
      <circle cx="30" cy="12" r="1.2" fill="currentColor" />
      <circle cx="96" cy="60" r="1.4" fill="currentColor" />
      <circle cx="14" cy="52" r="1" fill="currentColor" />
    </svg>
  );
}

function formatVoiceDuration(ms: number) {
  const total = Math.ceil(ms / 1000);
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
}

function base64ToBlob(base64: string, mime: string): Blob {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new Blob([bytes], { type: mime });
}

function VoiceBubble({ message }: { message: DmMessage }) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [playing, setPlaying] = useState(false);
  const [progress, setProgress] = useState(0);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    return () => {
      audioRef.current?.pause();
      if (audioRef.current?.src.startsWith("blob:")) {
        URL.revokeObjectURL(audioRef.current.src);
      }
    };
  }, []);

  const toggle = async () => {
    let audio = audioRef.current;
    if (audio && !audio.paused) {
      audio.pause();
      setPlaying(false);
      return;
    }
    if (!audio) {
      setLoading(true);
      try {
        const base64 = await getVoiceBlob(message.messageId);
        const blob = base64ToBlob(base64, "audio/webm");
        audio = new Audio(URL.createObjectURL(blob));
        audio.onended = () => {
          setPlaying(false);
          setProgress(0);
        };
        audio.ontimeupdate = () => {
          if (audio && message.durationMs > 0) {
            setProgress(
              Math.min(1, audio.currentTime / (message.durationMs / 1000))
            );
          }
        };
        audioRef.current = audio;
      } catch {
        setLoading(false);
        return;
      }
      setLoading(false);
    }
    await applyAudioSink(audio);
    try {
      await audio.play();
      setPlaying(true);
    } catch {
      setPlaying(false);
    }
  };

  return (
    <div className="mt-1 flex max-w-md items-center gap-3 rounded-2xl border border-white/[0.06] bg-void-4/60 px-4 py-3">
      <button
        onClick={() => void toggle()}
        disabled={loading}
        className="btn-press focus-glow flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-nebula text-void-0 shadow-lg shadow-nebula/20 hover:bg-nebula-hi disabled:opacity-50"
        aria-label={playing ? "pause" : "play"}
      >
        {loading ? (
          <span className="h-3 w-3 animate-orbit rounded-full border-2 border-void-0 border-t-transparent" />
        ) : playing ? (
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
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          {Array.from({ length: 28 }).map((_, index) => {
            const active = index / 28 <= progress;
            return (
              <span
                key={index}
                className={`w-[3px] rounded-full transition-all duration-150 ${
                  active ? "bg-nebula-hi" : "bg-white/15"
                }`}
                style={{
                  height: `${4 + ((index * 7919) % 13)}px`,
                }}
              />
            );
          })}
        </div>
      </div>
      <span className="shrink-0 font-display text-[11px] font-bold text-mist-3 tabular-nums">
        {formatVoiceDuration(message.durationMs)}
      </span>
    </div>
  );
}

export default function MessageList({
  messages,
  resolveAuthor,
  showStatus,
  emptyTitle,
  emptyHint,
  peerProfiles = [],
  onOpenProfile,
}: Props) {
  const { t, locale } = useI18n();
  const scrollRef = useRef<HTMLDivElement>(null);
  const profileByOnion = useMemo(
    () => new Map(peerProfiles.map((p) => [p.onionId, p])),
    [peerProfiles]
  );

  useEffect(() => {
    const container = scrollRef.current;
    if (container) {
      container.scrollTo({ top: container.scrollHeight, behavior: "smooth" });
    }
  }, [messages.length]);

  const groups = useMemo(() => {
    const out: Group[] = [];
    for (const message of messages) {
      const author = resolveAuthor(message.authorId);
      const last = out[out.length - 1];
      if (
        last &&
        last.mine === author.mine &&
        message.createdAt - last.items[last.items.length - 1].createdAt < GROUP_GAP_MS
      ) {
        last.items.push(message);
      } else {
        out.push({
          key: message.messageId,
          mine: author.mine,
          items: [message],
        });
      }
    }
    return out;
  }, [messages, resolveAuthor]);

  return (
    <div ref={scrollRef} className="relative z-10 flex-1 overflow-y-auto px-6 py-5">
      {messages.length === 0 && (
        <div className="mt-16 text-center animate-fade-up">
          <EmptyOrbit />
          <p className="font-display text-[16px] font-bold text-mist-1">{emptyTitle}</p>
          <p className="mx-auto mt-2 max-w-md text-[13px] leading-relaxed text-mist-3">
            {emptyHint}
          </p>
        </div>
      )}

      {groups.map((group, groupIndex) => {
        const first = group.items[0];
        const previous = groups[groupIndex - 1];
        const showDay =
          !previous || !sameDay(first.createdAt, previous.items[0].createdAt);
        const lastItem = group.items[group.items.length - 1];
        const author = resolveAuthor(first.authorId);

        return (
          <div
            key={group.key}
            className="animate-fade-up"
            style={{ animationDelay: `${Math.min(groupIndex, 8) * 24}ms` }}
          >
            {showDay && (
              <div className="my-5 flex items-center gap-3">
                <div className="h-px flex-1 bg-white/[0.07]" />
                <span className="font-display text-[10px] font-bold uppercase tracking-[0.16em] text-mist-3">
                  {formatDay(
                    first.createdAt,
                    dateLocale(locale),
                    t("messages.today"),
                    t("messages.yesterday")
                  )}
                </span>
                <div className="h-px flex-1 bg-white/[0.07]" />
              </div>
            )}
            <div
              className="-mx-4 mb-0.5 rounded-lg px-4 py-1 transition-colors duration-100 hover:bg-white/[0.03] group/message"
            >
              <div className="mb-3 flex gap-3.5">
                <button
                  onClick={() => onOpenProfile?.(first.authorId)}
                  className="mt-0.5 shrink-0 cursor-pointer opacity-80 transition-opacity group-hover/message:opacity-100"
                  disabled={!onOpenProfile}
                >
                  <Avatar
                    onionId={first.authorId}
                    name={author.name}
                    size={40}
                  />
                </button>
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline gap-2.5">
                    <button
                      onClick={() => onOpenProfile?.(first.authorId)}
                      disabled={!onOpenProfile}
                      className="text-[15px] font-semibold hover:underline disabled:no-underline"
                      style={{
                        color:
                          profileByOnion.get(first.authorId)?.accent ||
                          author.color,
                      }}
                    >
                      {author.name}
                    </button>
                    <span className="font-display text-[11px] font-medium text-mist-3">
                      {formatTime(first.createdAt, dateLocale(locale))}
                    </span>
                  </div>
                  {group.items.map((message) =>
                    message.kind === "voice" ? (
                      <VoiceBubble key={message.messageId} message={message} />
                    ) : (
                      <p
                        key={message.messageId}
                        className="selectable whitespace-pre-wrap break-words text-[15px] leading-[1.5] text-mist-1/95"
                      >
                        {message.body}
                      </p>
                    )
                  )}
                  {showStatus && group.mine && (
                    <p className="mt-1 font-display text-[10px] font-bold uppercase tracking-[0.1em] text-mist-3">
                      {lastItem.status === "delivered"
                        ? t("messages.delivered")
                        : lastItem.status === "sent"
                          ? t("messages.sent")
                          : t("messages.awaitingRelay")}
                    </p>
                  )}
                </div>
              </div>
            </div>
          </div>
        );
      })}
      <div />
    </div>
  );
}
