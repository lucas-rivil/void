import { useCallback, useEffect, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { DmMessage, PeerProfileInfo, PresenceInfo } from "../types";
import { dmHistory, onCoreEvent, sendDm, sendPing, sendVoiceDm } from "../lib/void";
import MessageList from "./MessageList";
import Composer from "./Composer";
import { useI18n } from "../lib/i18n";

interface Props {
  entry: PresenceInfo;
  myOnionId: string | null;
  online: boolean;
  peerProfiles: PeerProfileInfo[];
  onOpenProfile: (onionId: string) => void;
}

export default function ChatPanel({ entry, myOnionId, online, peerProfiles, onOpenProfile }: Props) {
  const { t } = useI18n();
  const [messages, setMessages] = useState<DmMessage[]>([]);
  const [copied, setCopied] = useState(false);
  const [pinged, setPinged] = useState(false);

  const merge = useCallback((incoming: DmMessage) => {
    setMessages((current) => {
      const index = current.findIndex((m) => m.messageId === incoming.messageId);
      if (index >= 0) {
        const next = [...current];
        next[index] = incoming;
        return next;
      }
      return [...current, incoming].sort((a, b) =>
        (a.messageId ?? "").localeCompare(b.messageId ?? "")
      );
    });
  }, []);

  useEffect(() => {
    dmHistory(entry.onionId, 100)
      .then(setMessages)
      .catch(() => undefined);
  }, [entry.onionId]);

  useEffect(() => {
    const unlisten = onCoreEvent((event) => {
      if (event.type === "dmNew" && event.message.peerId === entry.onionId) {
        merge(event.message);
      } else if (event.type === "dmStatus" && event.peerId === entry.onionId) {
        setMessages((current) =>
          current.map((m) =>
            m.messageId === event.messageId ? { ...m, status: event.status } : m
          )
        );
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [entry.onionId, merge]);

  const resolveAuthor = useCallback(
    (authorId: string) =>
      authorId === myOnionId
        ? { name: t("common.you"), mine: true, color: "#f5f5f5" }
        : { name: entry.displayName, mine: false, color: "#cfcfcf" },
    [myOnionId, entry.displayName, t]
  );

  const copyOnion = async () => {
    await writeText(`${entry.onionId}.onion`);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const ping = async () => {
    setPinged(true);
    setTimeout(() => setPinged(false), 1200);
    try {
      await sendPing(entry.onionId);
    } catch {
      void 0;
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="relative z-10 flex h-14 shrink-0 items-center gap-2.5 border-b border-white/[0.05] px-6">
        <span className="text-lg text-mist-3">@</span>
        <span className="font-display text-[15px] font-bold text-mist-1">
          {entry.displayName}
        </span>
        <span
          className={`h-2 w-2 rounded-full ${
            entry.online ? "bg-nova text-nova animate-glow-pulse" : "bg-void-6"
          }`}
        />
        {entry.online && entry.rttMs !== null && (
          <span
            className={`rounded-lg bg-nova/10 px-2 py-0.5 font-display text-[11px] font-bold text-nova transition-transform ${
              pinged ? "scale-125" : ""
            }`}
          >
            {entry.rttMs} ms
          </span>
        )}
        <div className="ml-auto flex items-center gap-1">
          {entry.online && (
            <button
              onClick={() => void ping()}
              className="btn-press rounded-lg px-2.5 py-1.5 font-display text-[11px] font-bold uppercase tracking-wider text-mist-3 hover:bg-white/5 hover:text-nova"
            >
              {t("chat.ping")}
            </button>
          )}
          <button
            onClick={copyOnion}
            className="btn-press rounded-lg px-2.5 py-1.5 font-display text-[11px] font-bold uppercase tracking-wider text-mist-3 hover:bg-white/5 hover:text-nebula-hi"
            title={t("chat.copyOnion")}
          >
            {copied ? t("common.copied") : ".onion"}
          </button>
        </div>
      </div>

      <MessageList
        messages={messages}
        resolveAuthor={resolveAuthor}
        showStatus
        emptyTitle={t("chat.emptyTitle", { name: entry.displayName })}
        emptyHint={t("chat.emptyHint")}
        peerProfiles={peerProfiles}
        onOpenProfile={onOpenProfile}
      />

      <div className="relative z-10">
        <Composer
          placeholder={
            entry.online
              ? t("chat.writeTo", { name: entry.displayName })
              : t("chat.offlineRelay", { name: entry.displayName })
          }
          disabled={!online}
          disabledPlaceholder={t("chat.torConnecting")}
          onSubmit={async (text) => {
            const message = await sendDm(entry.onionId, text);
            merge(message);
          }}
          onVoice={async (base64, durationMs) => {
            const message = await sendVoiceDm(entry.onionId, base64, durationMs);
            merge(message);
          }}
          footnote={t("chat.footnote")}
        />
      </div>
    </div>
  );
}
