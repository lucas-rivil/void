import { useEffect, useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { IdentityInfo, PeerInfo, TorStatus } from "../types";
import { getInviteLink, getInviteQr } from "../lib/void";
import TorStatusCard from "./TorStatusCard";
import VoidLogo from "./VoidLogo";
import { useI18n } from "../lib/i18n";

interface Props {
  identity: IdentityInfo;
  status: TorStatus;
  needsOnboarding: boolean;
  peers: PeerInfo[];
  onOpenIdentity: () => void;
  onAddPeer: () => void;
  onCreateGroup: () => void;
}

export default function WelcomePanel({
  identity,
  status,
  needsOnboarding,
  peers,
  onOpenIdentity,
  onAddPeer,
  onCreateGroup,
}: Props) {
  const { t } = useI18n();
  const [copied, setCopied] = useState(false);
  const [inviteLink, setInviteLink] = useState<string | null>(null);
  const [qrSvg, setQrSvg] = useState<string | null>(null);

  useEffect(() => {
    getInviteLink().then(setInviteLink).catch(() => undefined);
  }, [identity.onion]);

  const copyInvite = async () => {
    if (!inviteLink) return;
    await writeText(inviteLink);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const toggleQr = async () => {
    if (qrSvg) {
      setQrSvg(null);
      return;
    }
    try {
      setQrSvg(await getInviteQr());
    } catch {
      setQrSvg(null);
    }
  };

  return (
    <div className="relative z-10 flex-1 overflow-y-auto">
      <div className="flex h-14 shrink-0 items-center gap-2 border-b border-white/[0.05] px-6">
        <span className="text-lg text-mist-3">#</span>
        <span className="font-display text-[15px] font-bold text-mist-1">
          {t("welcome.channelName")}
        </span>
        {status.kind === "online" && (
          <span className="ml-auto rounded-lg bg-nova/10 px-2.5 py-1 font-display text-[10px] font-bold uppercase tracking-[0.12em] text-nova">
            {t("welcome.p2pTor")}
          </span>
        )}
      </div>

      <div className="mx-auto max-w-2xl space-y-5 px-8 py-12">
        <div className="text-center animate-fade-up">
          <div className="mx-auto w-fit rounded-3xl bg-nebula/10 p-5 text-nebula-hi shadow-[0_0_60px_rgba(255,255,255,0.12)]">
            <VoidLogo size={72} className="text-shadow-void" />
          </div>
          <h1 className="mt-6 font-display text-3xl font-bold tracking-tight text-mist-1">
            {t("welcome.title", { name: identity.displayName })}
          </h1>
          <p className="mx-auto mt-3 max-w-md text-[15px] leading-relaxed text-mist-2">
            {t("welcome.subtitle")}
          </p>
        </div>

        {needsOnboarding && (
          <div className="animate-fade-up rounded-2xl border border-ambre/30 bg-ambre/[0.07] p-4 text-[14px] text-ambre">
            {t("welcome.onboarding")}{" "}
            <button onClick={onOpenIdentity} className="font-semibold underline">
              {t("welcome.configureIt")}
            </button>
            .
          </div>
        )}

        <section
          className="rounded-2xl border border-white/[0.06] bg-void-4/70 p-5 animate-fade-up"
          style={{ animationDelay: "60ms" }}
        >
          <span className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
            {t("welcome.signal")}
          </span>
          <p className="mt-3 break-all rounded-xl bg-void-2 px-3.5 py-2.5 font-mono text-[12px] leading-relaxed text-nova/90">
            {inviteLink ?? "…"}
          </p>
          <div className="mt-3 flex gap-2">
            <button
              onClick={copyInvite}
              disabled={!inviteLink}
              className="btn-press focus-glow rounded-xl bg-nebula px-4 py-2 text-[14px] font-semibold text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi disabled:opacity-50 disabled:shadow-none"
            >
              {copied ? t("common.copied") : t("welcome.copyLink")}
            </button>
            <button
              onClick={toggleQr}
              disabled={!inviteLink}
              className="btn-press rounded-xl bg-void-2 px-4 py-2 text-[14px] font-medium text-mist-1 hover:bg-void-5 disabled:opacity-50"
            >
              {qrSvg ? t("welcome.hideQr") : t("welcome.showQr")}
            </button>
          </div>
          {qrSvg && (
            <div
              className="mx-auto mt-4 w-[240px] animate-fade-up rounded-2xl bg-white p-3 shadow-2xl [&>svg]:h-auto [&>svg]:w-full"
              dangerouslySetInnerHTML={{ __html: qrSvg }}
            />
          )}
        </section>

        <section
          className="rounded-2xl border border-white/[0.06] bg-void-4/70 p-5 animate-fade-up"
          style={{ animationDelay: "120ms" }}
        >
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-bold uppercase tracking-[0.12em] text-mist-3">
              {t("welcome.peers")} — {peers.length}
            </span>
            <button
              onClick={onAddPeer}
              className="btn-press focus-glow rounded-xl bg-nebula px-3.5 py-1.5 text-[13px] font-semibold text-void-0 shadow-lg shadow-nebula/25 hover:bg-nebula-hi"
            >
              {t("welcome.addPeer")}
            </button>
          </div>
          {peers.length === 0 ? (
            <p className="mt-3 text-[14px] leading-relaxed text-mist-2">
              {t("welcome.noPeers")}
            </p>
          ) : (
            <p className="mt-3 text-[14px] leading-relaxed text-mist-2">
              {t("welcome.clickPeer")}
            </p>
          )}
          <button
            onClick={onCreateGroup}
            className="btn-press mt-3 rounded-xl bg-void-2 px-4 py-2 text-[14px] font-medium text-mist-1 hover:bg-void-5"
          >
            {t("welcome.createGroup")}
          </button>
        </section>

        <div style={{ animationDelay: "180ms" }} className="animate-fade-up">
          <TorStatusCard status={status} />
        </div>
      </div>
    </div>
  );
}
