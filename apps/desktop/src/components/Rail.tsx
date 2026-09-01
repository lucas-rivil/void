import type { GroupInfo } from "../types";
import { authorColor } from "../lib/color";
import VoidLogo from "./VoidLogo";
import { useI18n } from "../lib/i18n";

interface Props {
  groups: GroupInfo[];
  activeGroupId: string | null;
  homeActive: boolean;
  onSelectHome: () => void;
  onSelectGroup: (groupId: string) => void;
  onCreateGroup: () => void;
  onOpenSettings: () => void;
}

export default function Rail({
  groups,
  activeGroupId,
  homeActive,
  onSelectHome,
  onSelectGroup,
  onCreateGroup,
  onOpenSettings,
}: Props) {
  const { t } = useI18n();
  return (
    <nav className="flex w-[76px] shrink-0 flex-col items-center gap-2.5 overflow-y-auto bg-void-1 py-3.5">
      <div className="relative">
        <button
          onClick={onSelectHome}
          title={t("rail.dms")}
          className={`btn-press flex h-12 w-12 items-center justify-center transition-all ${
            homeActive
              ? "rounded-2xl bg-nebula/15 text-nebula-hi shadow-lg shadow-nebula/20"
              : "rounded-3xl bg-void-4 text-mist-2 hover:rounded-2xl hover:text-nebula-hi"
          }`}
        >
          <VoidLogo size={30} orbit={false} />
        </button>
        {homeActive && (
          <span className="absolute -left-[15px] top-1/2 h-9 w-[3px] -translate-y-1/2 rounded-full bg-nebula shadow-[0_0_12px_rgba(255,255,255,0.8)]" />
        )}
      </div>

      {groups.length > 0 && <div className="h-px w-8 rounded bg-white/[0.07]" />}

      {groups.map((group) => {
        const active = group.groupId === activeGroupId;
        const anyOnline = group.members.some((m) => m.online);
        return (
          <div key={group.groupId} className="relative">
            <button
              onClick={() => onSelectGroup(group.groupId)}
              title={group.name}
              className={`btn-press flex h-12 w-12 items-center justify-center font-display text-sm font-bold text-void-0 transition-all ${
                active
                  ? "rounded-2xl shadow-lg"
                  : "rounded-3xl opacity-80 hover:rounded-2xl hover:opacity-100"
              }`}
              style={{
                backgroundColor: authorColor(group.groupId),
                boxShadow: active
                  ? `0 6px 20px ${authorColor(group.groupId)}55`
                  : undefined,
              }}
            >
              {group.name
                .split(/\s+/)
                .map((w) => w[0])
                .join("")
                .slice(0, 2)
                .toUpperCase()}
            </button>
            {active && (
              <span className="absolute -left-[15px] top-1/2 h-9 w-[3px] -translate-y-1/2 rounded-full bg-white/80" />
            )}
            <span
              className={`absolute -bottom-0.5 -right-0.5 h-3.5 w-3.5 rounded-full border-2 border-void-1 ${
                anyOnline ? "bg-nova text-nova animate-glow-pulse" : "bg-void-6"
              }`}
              title={anyOnline ? t("rail.membersOnline") : t("rail.membersOffline")}
            />
          </div>
        );
      })}

      <button
        onClick={onCreateGroup}
        title={t("rail.createGroup")}
        className="btn-press flex h-12 w-12 items-center justify-center rounded-3xl bg-void-4 text-2xl font-light text-mist-2 transition-all hover:rounded-2xl hover:bg-nebula hover:text-void-0"
      >
        +
      </button>

      <div className="mt-auto" />
      <button
        onClick={onOpenSettings}
        title={t("rail.settings")}
        className="btn-press flex h-12 w-12 items-center justify-center rounded-3xl bg-void-4 text-mist-2 transition-all hover:rounded-2xl hover:bg-void-5 hover:text-mist-1"
      >
        <svg width="19" height="19" viewBox="0 0 20 20" fill="currentColor">
          <path d="M10 6.5A3.5 3.5 0 1 0 10 13.5a3.5 3.5 0 0 0 0-7zm0 2a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3z" />
          <path d="M9.1 1h1.8l.3 2.1 1.7.7 1.8-1.2 1.3 1.3-1.2 1.8.7 1.7 2.1.3v1.8l-2.1.3-.7 1.7 1.2 1.8-1.3 1.3-1.8-1.2-1.7.7-.3 2.1H9.1l-.3-2.1-1.7-.7-1.8 1.2-1.3-1.3 1.2-1.8-.7-1.7L2 10.9V9.1l2.1-.3.7-1.7L3.6 5.3l1.3-1.3 1.8 1.2 1.7-.7L9.1 1z" />
        </svg>
      </button>
    </nav>
  );
}
