import { useEffect } from "react";
import { toast } from "sonner";
import { Input } from "@/components/ui/input";
import { pick, useLocale } from "@/lib/i18n";
import { cn } from "@/lib/cn";
import { PROOFSHIP_RELEASES } from "@/lib/links";
import type { useDesktopLink } from "@/lib/use-desktop-link";

type Link = ReturnType<typeof useDesktopLink>;

function Lamp({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "on" | "off" | "run" | "fail";
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-md border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-[0.08em]",
        tone === "on" ? "text-success" : tone === "fail" ? "text-red-300" : "text-fg-subtle",
      )}
    >
      <span
        className={cn(
          "size-1.5 rounded-full",
          tone === "on" && "bg-success",
          tone === "run" && "lamp-run bg-accent",
          tone === "fail" && "bg-red-400",
          tone === "off" && "bg-fg-subtle",
        )}
      />
      {label}
      <span className="normal-case tracking-normal text-fg">{value}</span>
    </span>
  );
}

export function DesktopLinkBar({
  link,
  sessionId,
}: {
  link: Link;
  sessionId?: string;
}) {
  const { locale } = useLocale();

  const setRoomId = link.setRoomId;
  const roomId = link.roomId;

  useEffect(() => {
    if (sessionId && !roomId) setRoomId(sessionId);
  }, [sessionId, roomId, setRoomId]);

  const desktopTone = link.desktopOnline ? "on" : link.status === "connecting" ? "run" : "off";
  const platformTone = link.platformOnline ? "on" : "off";
  const relayTone =
    link.status === "live"
      ? "on"
      : link.status === "error"
        ? "fail"
        : link.status === "connecting"
          ? "run"
          : "off";

  return (
    <div className="border-b border-border bg-surface/80 px-3 py-2.5 sm:px-4">
      <div className="flex flex-wrap items-center gap-2">
        <Lamp
          label={pick(locale, "Desktop", "桌面")}
          value={
            link.desktopOnline
              ? link.desktop?.deviceId?.slice(0, 10) || pick(locale, "online", "在线")
              : pick(locale, "offline", "离线")
          }
          tone={desktopTone}
        />
        <Lamp
          label={pick(locale, "Platform", "平台")}
          value={link.platformOnline ? pick(locale, "online", "在线") : pick(locale, "offline", "离线")}
          tone={platformTone}
        />
        <Lamp
          label="Relay"
          value={
            link.status === "live"
              ? pick(locale, "live", "已连")
              : link.status === "connecting"
                ? pick(locale, "connecting", "连接中")
                : link.status === "error"
                  ? pick(locale, "down", "断开")
                  : pick(locale, "idle", "待命")
          }
          tone={relayTone}
        />
        <span className="inline-flex items-center gap-1.5 rounded-md border border-border px-2 py-1 font-mono text-[10px] uppercase tracking-[0.08em] text-fg-subtle">
          <span className="size-1.5 rounded-full bg-fg-subtle" />
          {pick(locale, "Keys never here", "密钥不在此")}
        </span>

        <div className="ml-auto flex flex-wrap items-center gap-2">
          <select
            value={link.harness}
            onChange={(e) => link.setHarness(e.target.value)}
            className="h-8 rounded-[var(--radius-md)] border border-border bg-bg px-2 font-mono text-[11px] text-fg"
            aria-label={pick(locale, "Code agent", "代码 Agent")}
          >
            {link.harnesses.map((h) => (
              <option key={h.id} value={h.id}>
                {h.name}
                {h.installed === false ? " · off" : ""}
              </option>
            ))}
          </select>
          <select
            value={link.executor}
            onChange={(e) => link.setExecutor(e.target.value as "user" | "platform")}
            className="h-8 rounded-[var(--radius-md)] border border-border bg-bg px-2 font-mono text-[11px] text-fg"
            aria-label={pick(locale, "Executor", "执行器")}
          >
            <option value="user">{pick(locale, "My desktop", "我的桌面")}</option>
            <option value="platform">{pick(locale, "Platform sandbox", "平台沙箱")}</option>
          </select>
          <Input
            value={link.relayUrl}
            onChange={(e) => link.setRelayUrl(e.target.value)}
            className="h-8 w-[min(100%,200px)] font-mono text-[11px]"
            aria-label="Relay"
          />
          <Input
            value={link.roomId}
            onChange={(e) => link.setRoomId(e.target.value)}
            placeholder={sessionId || "desktop-…"}
            className="h-8 w-[min(100%,160px)] font-mono text-[11px]"
            aria-label={pick(locale, "Room", "房间")}
          />
          {link.status === "live" ? (
            <button
              type="button"
              className="h-8 rounded-[var(--radius-md)] border border-border px-3 text-[12px] text-fg hover:border-border-strong"
              onClick={() => link.disconnect()}
            >
              {pick(locale, "Disconnect", "断开")}
            </button>
          ) : (
            <button
              type="button"
              className="h-8 rounded-[var(--radius-md)] bg-accent px-3 text-[12px] font-semibold text-accent-fg hover:bg-accent-hover"
              onClick={() => {
                const room = link.roomId || sessionId;
                if (!room) {
                  toast.error(pick(locale, "Open or paste a room id first.", "先打开会话或粘贴房间 id。"));
                  return;
                }
                if (!link.roomId && sessionId) link.setRoomId(sessionId);
                link.connect(room);
              }}
            >
              {pick(locale, "Connect", "连接")}
            </button>
          )}
        </div>
      </div>
      {link.lastError && <p className="mt-2 font-mono text-[11px] text-red-300">{link.lastError}</p>}
    </div>
  );
}

export function PairingCard({
  link,
  sessionId,
}: {
  link: Link;
  sessionId?: string;
}) {
  const { locale } = useLocale();
  const room = link.roomId || sessionId || "—";
  return (
    <div className="rounded-[var(--radius-2xl)] border border-border bg-surface p-5 sm:p-6">
      <p className="text-[11px] font-semibold tracking-[0.14em] text-accent">
        {pick(locale, "Local first", "本地优先")}
      </p>
      <h2 className="mt-2 font-display text-[1.65rem] italic tracking-tight text-fg">
        {pick(locale, "This page is a remote panel.", "这个页面只是远程面板。")}
      </h2>
      <p className="mt-2 max-w-[52ch] text-[13.5px] leading-relaxed text-fg-muted">
        {pick(
          locale,
          "Same shape as desktop Sessions: watch the transcript and enqueue prompts. Without an online executor this page stays read-only. Code and keys stay on the machine.",
          "和桌面 Sessions 同一套形状：看记录、排队提示。没有在线执行器时页面只读。代码和密钥留在那台机器上。",
        )}
      </p>
      <ol className="mt-4 space-y-2 text-[13px] text-fg-muted">
        <li>
          1.{" "}
          <a href={PROOFSHIP_RELEASES} className="font-medium text-accent hover:text-accent-hover">
            {pick(locale, "Download desktop ProofShip", "下载桌面版 ProofShip")}
          </a>
          {pick(locale, " — or cargo run -p comet.", " — 或 cargo run -p comet。")}
        </li>
        <li>
          2.{" "}
          {pick(
            locale,
            "Paste the desktop room (desktop-…) or open a shared ?relay=&session= link, then Connect.",
            "粘贴桌面房间号（desktop-…）或打开共享的 ?relay=&session= 链接，然后点连接。",
          )}
        </li>
        <li>
          3.{" "}
          {pick(
            locale,
            "When Desktop is online, Send / Steer / Comment go to that machine.",
            "桌面在线后，发送 / 纠偏 / 批注会发到那台机器。",
          )}
        </li>
      </ol>
      <p className="mt-4 font-mono text-[11.5px] text-fg-subtle">
        {pick(locale, "Room", "房间")} {room}
        {link.viewers ? ` · ${link.viewers} viewers` : ""}
      </p>
    </div>
  );
}
