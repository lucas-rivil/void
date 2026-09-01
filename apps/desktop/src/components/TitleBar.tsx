import { getCurrentWindow } from "@tauri-apps/api/window";
import VoidLogo from "./VoidLogo";
import { useI18n } from "../lib/i18n";

const win = getCurrentWindow();

export default function TitleBar({ online }: { online: boolean }) {
  const { t } = useI18n();
  return (
    <div
      data-tauri-drag-region
      className="flex h-10 shrink-0 select-none items-center justify-between bg-void-1/80 pl-4 backdrop-blur-md"
    >
      <div className="flex items-center gap-2.5" data-tauri-drag-region>
        <VoidLogo size={20} orbit={false} className="text-nebula-hi" />
        <span
          data-tauri-drag-region
          className="font-display text-[13px] font-bold tracking-[0.2em] text-mist-2"
        >
          VOID
        </span>
        <span
          className={`h-1.5 w-1.5 rounded-full ${
            online ? "bg-nova text-nova animate-glow-pulse" : "bg-ambre text-ambre"
          }`}
          title={online ? t("titlebar.connectedTor") : t("titlebar.connectingTor")}
        />
      </div>
      <div className="flex h-full">
        <button
          onClick={() => win.minimize()}
          className="btn-press flex h-full w-12 items-center justify-center text-mist-3 hover:bg-white/5 hover:text-mist-1"
          aria-label={t("titlebar.minimize")}
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <rect y="4.5" width="10" height="1" fill="currentColor" />
          </svg>
        </button>
        <button
          onClick={() => win.toggleMaximize()}
          className="btn-press flex h-full w-12 items-center justify-center text-mist-3 hover:bg-white/5 hover:text-mist-1"
          aria-label={t("titlebar.maximize")}
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" />
          </svg>
        </button>
        <button
          onClick={() => win.close()}
          className="btn-press flex h-full w-12 items-center justify-center text-mist-3 hover:bg-alerte hover:text-void-0"
          aria-label={t("titlebar.close")}
        >
          <svg width="10" height="10" viewBox="0 0 10 10">
            <path d="M1 1 L9 9 M9 1 L1 9" stroke="currentColor" strokeWidth="1.2" />
          </svg>
        </button>
      </div>
    </div>
  );
}
