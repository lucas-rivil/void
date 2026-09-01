import { useCallback, useEffect, useMemo, useState } from "react";
import type { DmMessage, GroupInfo, PeerProfileInfo } from "../types";
import { groupHistory, onCoreEvent, sendGroupMessage, sendVoiceGroup } from "../lib/void";
import MessageList from "./MessageList";
import Composer from "./Composer";
import { authorColor } from "../lib/color";
import { useI18n } from "../lib/i18n";

interface Props {
  group: GroupInfo;
  myOnionId: string | null;
  online: boolean;
  peerProfiles: PeerProfileInfo[];
  onOpenProfile: (onionId: string) => void;
  onInvite: () => void;
  onLeave: () => void;
}

export default function GroupChatPanel({
  group,
  myOnionId,
  online,
  peerProfiles,
  onOpenProfile,
  onInvite,
  onLeave,
}: Props) {
  const { t } = useI18n();
  const [messages, setMessages] = useState<DmMessage[]>([]);

  useEffect(() => {
    groupHistory(group.groupId, 100)
      .then(setMessages)
      .catch(() => undefined);
  }, [group.groupId]);

  useEffect(() => {
    const unlisten = onCoreEvent((event) => {
      if (
        (event.type === "groupMessage" || event.type === "dmNew") &&
        event.message.peerId === group.groupId
      ) {
        setMessages((current) => {
          if (current.some((m) => m.messageId === event.message.messageId)) {
            return current;
          }
          return [...current, event.message].sort((a, b) =>
            (a.messageId ?? "").localeCompare(b.messageId ?? "")
          );
        });
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [group.groupId]);

  const membersByOnion = useMemo(
    () => new Map(group.members.map((m) => [m.onionId, m])),
    [group.members]
  );

  const resolveAuthor = useCallback(
    (authorId: string) => {
      if (authorId === myOnionId) {
        return { name: t("common.you"), mine: true, color: "#f5f5f5" };
      }
      const member = membersByOnion.get(authorId);
      const name = member?.displayName || `${authorId.slice(0, 10)}…`;
      return { name, mine: false, color: authorColor(authorId) };
    },
    [myOnionId, membersByOnion, t]
  );

  const onlineCount = group.members.filter((m) => m.online).length;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="relative z-10 flex h-14 shrink-0 items-center gap-2.5 border-b border-white/[0.05] px-6">
        <span className="text-lg text-mist-3">#</span>
        <span className="font-display text-[15px] font-bold text-mist-1">
          {group.name}
        </span>
        <span className="font-display text-[11px] font-medium text-mist-3">
          {t("group.header", { total: group.members.length, online: onlineCount })}
        </span>
        <div className="ml-auto flex items-center gap-1">
          <button
            onClick={onInvite}
            className="btn-press rounded-lg px-2.5 py-1.5 font-display text-[11px] font-bold uppercase tracking-wider text-mist-3 hover:bg-white/5 hover:text-nebula-hi"
          >
            {t("group.invite")}
          </button>
          <button
            onClick={onLeave}
            className="btn-press rounded-lg px-2.5 py-1.5 font-display text-[11px] font-bold uppercase tracking-wider text-alerte/70 hover:bg-white/5 hover:text-alerte"
          >
            {t("group.leave")}
          </button>
        </div>
      </div>

      <MessageList
        messages={messages}
        resolveAuthor={resolveAuthor}
        showStatus={false}
        emptyTitle={t("group.welcomeTitle", { name: group.name })}
        emptyHint={t("group.emptyHint")}
        peerProfiles={peerProfiles}
        onOpenProfile={onOpenProfile}
      />

      <div className="relative z-10">
        <Composer
          placeholder={t("group.writeIn", { name: group.name })}
          disabled={!online}
          disabledPlaceholder={t("chat.torConnecting")}
          onSubmit={async (text) => {
            const message = await sendGroupMessage(group.groupId, text);
            setMessages((current) =>
              current.some((m) => m.messageId === message.messageId)
                ? current
                : [...current, message].sort((a, b) =>
                    (a.messageId ?? "").localeCompare(b.messageId ?? "")
                  )
            );
          }}
          onVoice={async (base64, durationMs) => {
            const message = await sendVoiceGroup(
              group.groupId,
              base64,
              durationMs
            );
            setMessages((current) =>
              current.some((m) => m.messageId === message.messageId)
                ? current
                : [...current, message].sort((a, b) =>
                    (a.messageId ?? "").localeCompare(b.messageId ?? "")
                  )
            );
          }}
          footnote={t("group.footnote")}
        />
      </div>
    </div>
  );
}
