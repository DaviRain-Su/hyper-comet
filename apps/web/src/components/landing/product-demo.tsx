import { useState } from "react";
import { Link } from "@tanstack/react-router";
import { ArrowRight, ArrowUp, Check, Plus } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { LeanCode } from "@/lib/lean-highlight";
import { cn } from "@/lib/cn";

type DemoView = "desktop" | "empty" | "session";

export function ProductDemo({ compact }: { compact?: boolean }) {
  const { t, locale } = useI18n();
  const [view, setView] = useState<DemoView>("desktop");
  const zh = locale === "zh";

  return (
    <div className={cn("relative", compact && "pointer-events-none select-none")}>
      {!compact && (
        <div className="mb-4 flex flex-wrap items-center gap-2">
          {(
            [
              ["desktop", t.showcaseTabDesktop],
              ["empty", t.showcaseTabEmpty],
              ["session", t.showcaseTabSession],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              data-demo-view={id}
              onClick={() => setView(id)}
              className={cn(
                "rounded-full border px-3 py-1.5 text-[12.5px] font-medium transition-colors",
                view === id
                  ? "border-accent/40 bg-accent/15 text-fg"
                  : "border-border bg-surface text-fg-muted hover:border-border-strong hover:text-fg",
              )}
            >
              {label}
            </button>
          ))}
        </div>
      )}

      <div className="relative overflow-hidden rounded-[var(--radius-2xl)] border border-border-strong bg-surface shadow-[var(--shadow-soft)]">
        {view === "desktop" && !compact ? (
          <DesktopShot />
        ) : (
          <>
            <div className="flex h-10 items-center gap-2 border-b border-border px-4">
              <span className="size-2.5 rounded-full bg-white/15" />
              <span className="size-2.5 rounded-full bg-white/15" />
              <span className="size-2.5 rounded-full bg-white/15" />
              <span className="ml-3 font-mono text-[11px] text-fg-subtle">
                {zh ? "app.proofship · Sessions · 远程面板" : "app.proofship · Sessions · remote panel"}
              </span>
            </div>
            {view === "empty" || compact ? <EmptyChrome zh={zh} compact={compact} /> : <SessionChrome zh={zh} />}
          </>
        )}
      </div>
    </div>
  );
}

function Lamp({
  label,
  value,
  on,
}: {
  label: string;
  value: string;
  on?: boolean;
}) {
  return (
    <span className="inline-flex items-center gap-1.5 rounded-md border border-border bg-bg px-2 py-1 font-mono text-[10px] uppercase tracking-wider text-fg-subtle">
      <span className={cn("size-1.5 rounded-full", on ? "bg-success" : "bg-fg-subtle")} />
      {label}
      <span className="normal-case tracking-normal text-fg">{value}</span>
    </span>
  );
}

function EmptyChrome({ zh, compact }: { zh: boolean; compact?: boolean }) {
  return (
    <div
      className={cn(
        "grid bg-bg",
        compact
          ? "min-h-[280px] grid-cols-[160px_minmax(0,1fr)]"
          : "min-h-[420px] lg:grid-cols-[200px_minmax(0,1fr)_220px] lg:min-h-[500px]",
      )}
    >
      <aside className="hidden border-r border-border bg-surface p-4 sm:block">
        <p className="wordmark font-display text-[1.35rem] italic leading-none text-fg">ProofShip</p>
        <div className="mt-5 flex h-9 items-center justify-center gap-1.5 rounded-[var(--radius-md)] bg-accent text-[12px] font-semibold text-accent-fg">
          <Plus className="size-3.5" />
          {zh ? "新会话" : "New session"}
        </div>
        <div className="mt-3 space-y-1.5">
          {(zh
            ? ["RWA 份额登记", "金库金流", "新会话"]
            : ["RWA share registry", "Treasury flow", "New session"]
          ).map((title, i) => (
            <div
              key={title}
              className={cn(
                "rounded-[var(--radius-md)] px-3 py-2.5 text-[12.5px]",
                i === 2 ? "bg-bg text-fg" : "text-fg-muted",
              )}
            >
              <span
                className={cn(
                  "mr-2 inline-block size-1.5 rounded-full align-middle",
                  i === 0 ? "bg-success" : "bg-fg-subtle",
                )}
              />
              {title}
            </div>
          ))}
        </div>
      </aside>

      <div className="flex min-w-0 flex-col">
        <div className="flex flex-wrap items-center gap-2 border-b border-border px-4 py-2.5">
          <span className="text-[13px] font-medium">Sessions</span>
          <div className="ml-auto hidden items-center gap-1.5 md:flex">
            <Lamp label={zh ? "桌面" : "Desktop"} value={zh ? "等待中" : "waiting"} />
            <Lamp label="Relay" value={zh ? "待命" : "idle"} />
            <Lamp label={zh ? "密钥" : "Keys"} value={zh ? "不在此" : "never here"} />
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 border-b border-border bg-surface px-4 py-2">
          <Lamp label={zh ? "桌面" : "Desktop"} value={zh ? "等待中" : "waiting"} />
          <Lamp label="Relay" value={zh ? "待命" : "idle"} />
          <span className="ml-auto hidden rounded-[var(--radius-md)] bg-accent px-3 py-1.5 text-[11px] font-semibold text-accent-fg sm:inline">
            {zh ? "连接桌面" : "Attach desktop"}
          </span>
        </div>
        <div className="min-h-0 flex-1 space-y-4 overflow-hidden px-5 py-6">
          <p className="text-[11px] font-semibold tracking-[0.14em] text-accent">Sessions</p>
          <h3 className="font-display text-[1.65rem] leading-tight text-fg sm:text-[1.85rem]">
            {zh ? "远程驱动你的电脑。" : "Drive the machine in front of you."}
          </h3>
          <p className="max-w-[46ch] text-[13px] leading-relaxed text-fg-muted">
            {zh
              ? "网页是远程工作台。Agent 和门禁留在桌面。点开始即可登录进入。"
              : "Web is the remote workspace. Agent and gate stay on your desktop. Get started to sign in."}
          </p>
          {!compact && (
            <>
              <div className="rounded-[var(--radius-xl)] border border-border bg-surface p-4">
                <p className="text-[11px] font-semibold tracking-[0.12em] text-accent">
                  {zh ? "本地优先" : "Local first"}
                </p>
                <p className="mt-1.5 font-display text-[1.25rem] text-fg">
                  {zh ? "这个页面只是远程面板。" : "This page is a remote panel."}
                </p>
                <ol className="mt-3 space-y-1.5 text-[12.5px] text-fg-muted">
                  <li>1. {zh ? "下载桌面版 ProofShip" : "Download desktop ProofShip"}</li>
                  <li>2. {zh ? "在本机运行 comet agent url，打开链接" : "Run comet agent url, then open the link"}</li>
                  <li>3. {zh ? "桌面灯亮后，提示发到本机" : "When Desktop is on, prompts go local"}</li>
                </ol>
              </div>
              <div className="grid gap-2 sm:grid-cols-3">
                {(zh
                  ? [
                      ["RwaShareRegistry", "RWA 份额登记"],
                      ["TimeLockPayout", "时间锁支付"],
                      ["StateCell", "状态单元"],
                    ]
                  : [
                      ["RwaShareRegistry", "RWA share registry"],
                      ["TimeLockPayout", "Time-lock payout"],
                      ["StateCell", "State cell"],
                    ]
                ).map(([mod, title]) => (
                  <div key={mod} className="rounded-[var(--radius-lg)] border border-border bg-surface px-3 py-2.5">
                    <p className="font-mono text-[10px] text-accent">{mod}</p>
                    <p className="mt-1 text-[12.5px] text-fg">{title}</p>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
        <div className="border-t border-border px-4 py-3">
          <div className="flex items-center gap-2 rounded-2xl border border-border bg-surface px-3 py-2">
            <span className="flex-1 text-[13px] text-fg-subtle">
              {zh ? "先连接桌面再发送…" : "Attach desktop to send…"}
            </span>
            <span className="grid size-8 place-items-center rounded-xl bg-accent/40 text-accent-fg">
              <ArrowUp className="size-3.5" />
            </span>
          </div>
        </div>
      </div>

      {!compact && (
        <aside className="hidden border-l border-border bg-surface p-4 lg:block">
          <p className="text-[11px] font-semibold tracking-wide text-accent uppercase">
            {zh ? "门禁" : "Gate"}
          </p>
          <p className="mt-3 text-[12px] leading-relaxed text-fg-muted">
            {zh ? "还没有跑过。真正的 check → build → inspect 在桌面跑。" : "No run yet. The real check → build → inspect stays on desktop."}
          </p>
          <p className="mt-6 text-[12px] text-fg-subtle">{zh ? "先过门禁。" : "Pass the gate first."}</p>
        </aside>
      )}
    </div>
  );
}

function SessionChrome({ zh }: { zh: boolean }) {
  return (
    <div className="grid min-h-[420px] bg-bg lg:grid-cols-[200px_minmax(0,1fr)_220px] lg:min-h-[500px]">
      <aside className="hidden border-r border-border bg-surface p-4 lg:block">
        <p className="wordmark font-display text-[1.35rem] italic leading-none text-fg">ProofShip</p>
        <div className="mt-5 flex h-9 items-center justify-center gap-1.5 rounded-[var(--radius-md)] bg-accent text-[12px] font-semibold text-accent-fg">
          <Plus className="size-3.5" />
          {zh ? "新会话" : "New session"}
        </div>
        <div className="mt-3 space-y-1.5">
          {(zh
            ? ["RWA 份额登记", "金库金流", "新会话"]
            : ["RWA share registry", "Treasury flow", "New session"]
          ).map((title, i) => (
            <div
              key={title}
              className={cn(
                "rounded-[var(--radius-md)] px-3 py-2.5 text-[12.5px]",
                i === 0 ? "bg-bg text-fg" : "text-fg-muted",
              )}
            >
              <span
                className={cn(
                  "mr-2 inline-block size-1.5 rounded-full align-middle",
                  i === 0 ? "bg-success" : "bg-fg-subtle",
                )}
              />
              {title}
            </div>
          ))}
        </div>
      </aside>

      <div className="flex min-w-0 flex-col">
        <div className="flex flex-wrap items-center gap-2 border-b border-border px-4 py-2.5">
          <span className="truncate text-[13px] font-medium">
            {zh ? "RWA 份额登记" : "RWA share registry"}
          </span>
          <div className="ml-auto hidden items-center gap-1.5 sm:flex">
            <Lamp label={zh ? "桌面" : "Desktop"} value={zh ? "在线" : "online"} on />
            <Lamp label="Relay" value={zh ? "已连" : "live"} on />
            <Lamp label={zh ? "密钥" : "Keys"} value={zh ? "不在此" : "never here"} />
          </div>
        </div>
        <div className="min-h-0 flex-1 space-y-3 overflow-hidden px-4 py-4 sm:px-5">
          <div className="ml-auto max-w-[78%] rounded-2xl bg-surface px-4 py-2.5 text-[13px] text-fg">
            {zh ? "起草一个带锁定期的 RWA 份额登记合约。" : "Draft an RWA share registry with a lockup."}
          </div>
          <div className="rounded-[var(--radius-xl)] border border-border bg-surface p-3">
            <div className="flex items-center justify-between text-[11px] font-semibold tracking-wide text-fg-subtle uppercase">
              Gate
              <span className="rounded-md border border-success/30 bg-success/10 px-1.5 py-0.5 text-success">
                pass
              </span>
            </div>
            <p className="mt-2 font-mono text-[12px] text-fg-muted">check → build → inspect</p>
          </div>
          <div className="overflow-hidden rounded-[var(--radius-xl)] border border-border">
            <div className="border-b border-border px-3 py-1.5 font-mono text-[11px] text-fg-subtle">
              RwaShareRegistry.lean
            </div>
            <LeanCode
              className="bg-surface-elevated/50 p-3"
              maxLines={4}
              source={`program RwaShareRegistry where
  state shares : UInt64
  entry lock(amt : UInt64)`}
            />
          </div>
        </div>
        <div className="border-t border-border px-4 py-3">
          <div className="flex items-center gap-2 rounded-2xl border border-border bg-surface px-3 py-2">
            <span className="flex-1 text-[13px] text-fg-subtle">
              {zh ? "向本机 agent 发提示…" : "Prompt your local agent…"}
            </span>
            <span className="grid size-8 place-items-center rounded-xl bg-accent text-accent-fg">
              <ArrowUp className="size-3.5" />
            </span>
          </div>
        </div>
      </div>

      <aside className="hidden border-l border-border bg-surface p-4 lg:block">
        <p className="text-[11px] font-semibold tracking-wide text-accent uppercase">
          {zh ? "门禁" : "Gate"}
        </p>
        <ol className="mt-3 space-y-2 text-[12px] text-fg-muted">
          {["check", "build", "inspect"].map((step) => (
            <li key={step} className="rounded-[var(--radius-md)] border border-border bg-bg px-3 py-2">
              <Check className="mr-2 inline size-3.5 text-success" />
              <span className="font-mono text-fg">{step}</span>
            </li>
          ))}
        </ol>
        <div className="mt-4 flex h-9 items-center justify-center rounded-[var(--radius-md)] bg-accent text-[12px] font-semibold text-accent-fg">
          {zh ? "让桌面去部署" : "Ask desktop to deploy"}
        </div>
      </aside>
    </div>
  );
}

function DesktopShot() {
  const { t } = useI18n();
  return (
    <figure>
      <div className="flex h-10 items-center gap-2 border-b border-border px-4">
        <span className="size-2.5 rounded-full bg-white/15" />
        <span className="size-2.5 rounded-full bg-white/15" />
        <span className="size-2.5 rounded-full bg-white/15" />
        <span className="ml-3 font-mono text-[11px] text-fg-subtle">Desktop · ProofShip</span>
      </div>
      <img
        src="/assets/app-screenshot.jpg"
        alt={t.showcaseShotAlt}
        className="block w-full object-cover object-top outline outline-1 -outline-offset-1 outline-white/10"
      />
      <figcaption className="border-t border-border px-4 py-3 text-center text-[13px] text-fg-subtle">
        {t.showcaseShotCaption}
      </figcaption>
    </figure>
  );
}

export function HeroProductPreview() {
  const { t } = useI18n();
  return (
    <div className="relative hidden w-full max-w-[540px] lg:block">
      <div className="absolute -inset-8 rounded-[2rem] bg-[radial-gradient(60%_60%_at_50%_40%,rgba(240,90,40,0.18),transparent_70%)]" />
      <div className="relative origin-top scale-[0.92] xl:scale-100">
        <ProductDemo compact />
      </div>
      <p className="relative mt-3 text-center text-[12px] text-white/70">{t.showcaseHeroHint}</p>
    </div>
  );
}

export function ShowcaseEnterButton() {
  const { t } = useI18n();
  return (
    <Button asChild>
      <Link to="/login" search={{ redirect: "/sessions" }}>
        {t.showcaseEnter}
        <ArrowRight className="size-4" />
      </Link>
    </Button>
  );
}
