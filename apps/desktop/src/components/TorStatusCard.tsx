import type { TorStatus } from "../types";
import { useI18n } from "../lib/i18n";

export default function TorStatusCard({ status }: { status: TorStatus }) {
  const { t } = useI18n();
  return (
    <section className="rounded-2xl border border-white/[0.06] bg-void-4/70 p-5">
      <span className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
        {t("tor.network")}
      </span>
      <div className="mt-3">
        {status.kind === "starting" && (
          <div className="flex items-center gap-3 text-[15px] text-mist-1">
            <OrbitRing className="text-ambre" />
            {t("tor.starting")}
          </div>
        )}
        {status.kind === "bootstrapping" && (
          <div className="animate-fade-up">
            <div className="flex items-center gap-3">
              <OrbitRing className="text-ambre" />
              <div className="flex-1">
                <div className="flex items-center justify-between text-[13px] text-mist-2">
                  <span>{t("tor.circuits")}</span>
                  <span className="font-display font-bold text-ambre">
                    {status.progress}%
                  </span>
                </div>
                <div className="mt-2 h-1 overflow-hidden rounded-full bg-white/[0.06]">
                  <div
                    className="h-full rounded-full bg-gradient-to-r from-nebula-deep to-nebula-hi transition-all duration-500"
                    style={{ width: `${Math.max(status.progress, 3)}%` }}
                  />
                </div>
              </div>
            </div>
          </div>
        )}
        {status.kind === "online" && (
          <div className="flex items-center gap-3 text-[15px] text-mist-1">
            <span className="h-2.5 w-2.5 rounded-full bg-nova text-nova animate-glow-pulse" />
            {t("tor.connected")}
            <span className="ml-auto rounded-lg bg-void-2 px-2.5 py-1 font-display text-[11px] font-medium text-mist-3">
              {t("tor.socks", { addr: status.socks })}
            </span>
          </div>
        )}
        {status.kind === "failed" && (
          <div className="text-[15px] text-alerte animate-fade-up">
            <p className="font-semibold">{t("tor.failed")}</p>
            <p className="mt-1 break-words text-[13px] text-alerte/80">{status.error}</p>
          </div>
        )}
      </div>
    </section>
  );
}

function OrbitRing({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`h-5 w-5 shrink-0 animate-orbit ${className}`}
      viewBox="0 0 24 24"
      fill="none"
    >
      <circle cx="12" cy="12" r="9" stroke="currentColor" strokeOpacity="0.25" strokeWidth="2" />
      <circle
        cx="12"
        cy="12"
        r="9"
        stroke="currentColor"
        strokeWidth="2"
        strokeDasharray="42 14"
        strokeLinecap="round"
      />
    </svg>
  );
}
