import { useState } from "react";
import { Check, Copy, Download, X } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { pick, useLocale } from "@/lib/i18n";
import { copyText, downloadText } from "@/lib/download";
import { healthUrl, mcpUrl } from "@/lib/relay";
import type { SessionRow } from "@/lib/sessions";
import type { useDesktopLink } from "@/lib/use-desktop-link";

type Link = ReturnType<typeof useDesktopLink>;

export function GateRail({
  session,
  link,
  onRegate,
  onDeploy,
  desktopOnline,
  busy,
}: {
  session: SessionRow | null;
  link?: Link;
  onRegate?: () => void;
  onDeploy?: (opts: { networkId: string; module: string; digest?: string }) => void;
  desktopOnline?: boolean;
  busy?: boolean;
}) {
  const { locale } = useLocale();
  const [tab, setTab] = useState<"gate" | "source" | "ops">("gate");
  const [network, setNetwork] = useState("xlayer-testnet");
  const [moduleName, setModuleName] = useState(session?.moduleName ?? "");
  const [digest, setDigest] = useState(session?.gate?.digest ?? "");
  const gate = session?.gate ?? null;
  const module = session?.moduleName ?? "program";
  const relay = link?.relayUrl ?? "";

  return (
    <aside className="flex h-full min-h-0 w-full flex-col border-l border-border bg-surface">
      <div className="flex h-14 items-center gap-1 border-b border-border px-3">
        {(["gate", "source", "ops"] as const).map((id) => (
          <button
            key={id}
            type="button"
            onClick={() => setTab(id)}
            className={`rounded-md px-2.5 py-1.5 text-[12px] ${
              tab === id ? "bg-bg text-fg" : "text-fg-subtle hover:text-fg"
            }`}
          >
            {id === "gate"
              ? pick(locale, "Gate", "门禁")
              : id === "source"
                ? pick(locale, "Source", "源码")
                : pick(locale, "Ops", "运维")}
          </button>
        ))}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {tab === "gate" && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-[13px] font-medium text-fg">check → build → inspect</h2>
              {gate && (
                <Badge variant={gate.passed ? "pass" : "fail"}>{gate.passed ? "pass" : "closed"}</Badge>
              )}
            </div>
            {!gate ? (
              <p className="text-[13px] leading-relaxed text-fg-muted">
                {pick(
                  locale,
                  "No run yet. The real check → build → inspect runs on your desktop.",
                  "还没有跑过。真正的 check → build → inspect 在桌面跑。",
                )}
              </p>
            ) : (
              <ol className="space-y-2">
                {gate.steps.map((step, i) => (
                  <li key={step.id} className="rounded-[var(--radius-md)] border border-border bg-bg p-3">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-[10px] text-fg-subtle">{String(i + 1).padStart(2, "0")}</span>
                      {step.status === "pass" ? (
                        <Check className="size-3.5 text-success" />
                      ) : step.status === "fail" ? (
                        <X className="size-3.5 text-red-300" />
                      ) : (
                        <span className="size-3.5 rounded-full border border-fg-subtle" />
                      )}
                      <span className="font-mono text-[12px]">{step.label}</span>
                    </div>
                    <p className="mt-1.5 text-[12px] text-fg-muted">{step.detail}</p>
                  </li>
                ))}
              </ol>
            )}
            {onRegate && session?.source && (
              <button
                type="button"
                disabled={busy}
                onClick={onRegate}
                className="h-9 w-full rounded-[var(--radius-md)] border border-border text-[12.5px] text-fg hover:border-border-strong disabled:opacity-40"
              >
                {pick(locale, "Run gate again", "再跑一次门禁")}
              </button>
            )}
          </div>
        )}

        {tab === "source" &&
          (session?.source ? (
            <div>
              <div className="mb-3 flex items-center gap-2">
                <span className="min-w-0 flex-1 truncate font-mono text-[12px]">{module}.lean</span>
                <button
                  type="button"
                  className="grid size-8 place-items-center text-fg-subtle hover:text-fg"
                  aria-label={pick(locale, "Copy", "复制")}
                  onClick={() => {
                    void copyText(session.source);
                    toast(pick(locale, "Copied source", "已复制源码"));
                  }}
                >
                  <Copy className="size-3.5" />
                </button>
                <button
                  type="button"
                  className="grid size-8 place-items-center text-fg-subtle hover:text-fg"
                  aria-label={pick(locale, "Download", "下载")}
                  onClick={() => downloadText(`${module}.lean`, session.source)}
                >
                  <Download className="size-3.5" />
                </button>
              </div>
              <pre className="whitespace-pre-wrap break-all font-mono text-[11.5px] leading-relaxed text-fg-muted">
                {session.source}
              </pre>
            </div>
          ) : (
            <p className="text-[13px] text-fg-muted">{pick(locale, "No source yet.", "还没有源码。")}</p>
          ))}

        {tab === "ops" && (
          <div className="space-y-5">
            <section>
              <p className="mb-2 text-[11px] font-semibold tracking-[0.1em] text-fg-subtle uppercase">
                {pick(locale, "Deploy", "部署")}
              </p>
              <p className="mb-3 text-[12.5px] leading-relaxed text-fg-muted">
                {pick(
                  locale,
                  "Sends cmd.deploy to your desktop. Platform refuses keyed deploy. Keys never transit the relay.",
                  "把 cmd.deploy 发到桌面。平台沙箱拒绝带密钥部署。密钥不经中继。",
                )}
              </p>
              <div className="space-y-2">
                <Input
                  value={network}
                  onChange={(e) => setNetwork(e.target.value)}
                  className="h-8 font-mono text-[11px]"
                  aria-label={pick(locale, "Network id", "网络")}
                />
                <Input
                  value={moduleName || module}
                  onChange={(e) => setModuleName(e.target.value)}
                  className="h-8 font-mono text-[11px]"
                  aria-label={pick(locale, "Module", "模块")}
                />
                <Input
                  value={digest}
                  onChange={(e) => setDigest(e.target.value)}
                  placeholder={pick(locale, "Expected digest (optional)", "期望摘要（可选）")}
                  className="h-8 font-mono text-[11px]"
                />
              </div>
              {onDeploy && desktopOnline && gate?.passed ? (
                <button
                  type="button"
                  className="mt-3 flex h-9 w-full items-center justify-center rounded-[var(--radius-md)] bg-accent text-[12px] font-semibold text-accent-fg hover:bg-accent-hover"
                  onClick={() =>
                    onDeploy({
                      networkId: network,
                      module: moduleName || module,
                      digest: digest || gate.digest || undefined,
                    })
                  }
                >
                  {pick(locale, "Ask desktop to deploy", "让桌面去部署")}
                </button>
              ) : (
                <p className="mt-3 text-[12.5px] text-fg-muted">
                  {pick(locale, "Pass the gate and attach desktop first.", "先过门禁并连接桌面。")}
                </p>
              )}
            </section>

            <section>
              <p className="mb-2 text-[11px] font-semibold tracking-[0.1em] text-fg-subtle uppercase">
                Snapshot
              </p>
              <pre className="max-h-40 overflow-auto rounded-[var(--radius-md)] border border-border bg-bg p-3 font-mono text-[10.5px] leading-relaxed text-fg-muted">
                {JSON.stringify(link?.snapshot ?? {}, null, 2)}
              </pre>
            </section>

            <section>
              <p className="mb-2 text-[11px] font-semibold tracking-[0.1em] text-fg-subtle uppercase">
                {pick(locale, "Event tail", "事件尾")}
              </p>
              <ul className="max-h-36 space-y-1 overflow-auto font-mono text-[10.5px] text-fg-muted">
                {(link?.events ?? []).slice(-12).map((e, i) => (
                  <li key={`${e.kind}-${e.seq ?? i}`} className="truncate">
                    <span className="text-fg-subtle">{e.kind}</span>
                  </li>
                ))}
                {!link?.events?.length && <li>{pick(locale, "No events yet.", "还没有事件。")}</li>}
              </ul>
            </section>

            <section>
              <p className="mb-2 text-[11px] font-semibold tracking-[0.1em] text-fg-subtle uppercase">
                {pick(locale, "Contract interact", "合约交互")}
              </p>
              <p className="text-[12.5px] leading-relaxed text-fg-muted">
                {pick(
                  locale,
                  "Views via RPC eth_call. Writes use window.ethereum when present. ABI fills from a sealed snapshot.",
                  "读走 RPC eth_call。写走本机 window.ethereum。ABI 从密封快照填充。",
                )}
              </p>
            </section>

            {relay ? (
              <section>
                <p className="mb-2 text-[11px] font-semibold tracking-[0.1em] text-fg-subtle uppercase">
                  ProofForge MCP
                </p>
                <p className="mb-2 text-[12.5px] leading-relaxed text-fg-muted">
                  {pick(
                    locale,
                    "Not the main conversation surface. Remote IDE agents can attach this HTTP MCP. Compile + gate + deploy stay on the executor.",
                    "不是主对话面。远程 IDE agent 可挂这个 HTTP MCP。编译、门禁、部署仍在执行器上。",
                  )}
                </p>
                <button
                  type="button"
                  className="flex h-9 w-full items-center justify-center gap-2 rounded-[var(--radius-md)] border border-border text-[12px] text-fg hover:border-border-strong"
                  onClick={() => {
                    void copyText(mcpUrl(relay));
                    toast(pick(locale, "MCP URL copied", "已复制 MCP URL"));
                  }}
                >
                  <Copy className="size-3.5" />
                  {pick(locale, "Copy MCP URL", "复制 MCP URL")}
                </button>
                <a
                  href={healthUrl(relay)}
                  target="_blank"
                  rel="noreferrer"
                  className="mt-2 block text-center text-[12px] text-fg-subtle hover:text-fg"
                >
                  Health
                </a>
              </section>
            ) : null}
          </div>
        )}
      </div>
    </aside>
  );
}
