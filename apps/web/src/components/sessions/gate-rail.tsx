import { useState } from "react";
import { Check, Copy, Download, X } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { pick, useLocale } from "@/lib/i18n";
import { copyText, downloadText } from "@/lib/download";
import type { SessionRow } from "@/lib/sessions";

export function GateRail({
  session,
  onRegate,
  busy,
}: {
  session: SessionRow | null;
  onRegate?: () => void;
  busy?: boolean;
}) {
  const { locale } = useLocale();
  const [tab, setTab] = useState<"gate" | "source" | "artifacts">("gate");
  const gate = session?.gate ?? null;
  const module = session?.moduleName ?? "program";

  const deployCmd = session?.moduleName
    ? `PF_XLAYER_CONFIRM=yes proofship/scripts/deploy-xlayer-testnet.sh <lean> ${session.moduleName}`
    : "";

  return (
    <aside className="flex h-full min-h-0 w-full flex-col border-l border-line bg-raise">
      <div className="flex h-14 items-center gap-1 border-b border-line px-3">
        {(["gate", "source", "artifacts"] as const).map((id) => (
          <button
            key={id}
            type="button"
            onClick={() => setTab(id)}
            className={`rounded-md px-2.5 py-1.5 text-[12px] ${
              tab === id ? "bg-bg text-ink" : "text-faint hover:text-ink"
            }`}
          >
            {id === "gate"
              ? pick(locale, "Gate", "门禁")
              : id === "source"
                ? pick(locale, "Source", "源码")
                : pick(locale, "Artifacts", "制品")}
          </button>
        ))}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {tab === "gate" && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-[13px] font-medium text-ink">check → build → inspect</h2>
              {gate && (
                <Badge variant={gate.passed ? "pass" : "fail"}>{gate.passed ? "pass" : "closed"}</Badge>
              )}
            </div>
            {!gate ? (
              <p className="text-[13px] leading-relaxed text-dim">
                {pick(
                  locale,
                  "No run yet. Describe a contract or load a template.",
                  "还没有跑过门禁。描述合约，或载入模板。",
                )}
              </p>
            ) : (
              <ol className="space-y-3">
                {gate.steps.map((step) => (
                  <li key={step.id} className="rounded-lg border border-line bg-bg p-3">
                    <div className="flex items-center gap-2">
                      {step.status === "pass" ? (
                        <Check className="size-3.5 text-emerald-300" />
                      ) : step.status === "fail" ? (
                        <X className="size-3.5 text-red-300" />
                      ) : (
                        <span className="size-3.5 rounded-full border border-faint" />
                      )}
                      <span className="font-mono text-[12px]">{step.label}</span>
                    </div>
                    <p className="mt-1.5 text-[12px] text-dim">{step.detail}</p>
                  </li>
                ))}
              </ol>
            )}
            {onRegate && session?.source && (
              <button
                type="button"
                disabled={busy}
                onClick={onRegate}
                className="h-9 w-full rounded-lg border border-line text-[12.5px] text-ink hover:border-faint disabled:opacity-40"
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
                  className="grid size-8 place-items-center text-faint hover:text-ink"
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
                  className="grid size-8 place-items-center text-faint hover:text-ink"
                  aria-label={pick(locale, "Download", "下载")}
                  onClick={() => downloadText(`${module}.lean`, session.source)}
                >
                  <Download className="size-3.5" />
                </button>
              </div>
              <pre className="whitespace-pre-wrap break-all font-mono text-[11.5px] leading-relaxed text-dim">
                {session.source}
              </pre>
            </div>
          ) : (
            <p className="text-[13px] text-dim">{pick(locale, "No source yet.", "还没有源码。")}</p>
          ))}

        {tab === "artifacts" && (
          <div className="space-y-3">
            {!gate?.passed ? (
              <p className="text-[13px] text-dim">
                {pick(
                  locale,
                  "Fail-closed: zero artifacts until the gate passes.",
                  "失败关闭：门禁通过前没有制品。",
                )}
              </p>
            ) : (
              gate.artifacts.map((a) => (
                <div key={a.name} className="rounded-lg border border-line bg-bg p-3">
                  <div className="mb-1.5 flex items-center justify-between">
                    <span className="font-mono text-[11.5px] text-ink">{a.name}</span>
                    <button
                      type="button"
                      className="text-faint hover:text-ink"
                      onClick={() => {
                        void copyText(a.body);
                        toast(pick(locale, "Copied", "已复制"));
                      }}
                    >
                      <Copy className="size-3.5" />
                    </button>
                  </div>
                  <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-all font-mono text-[11px] text-dim">
                    {a.body}
                  </pre>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      <div className="border-t border-line p-4">
        <p className="mb-2 text-[11px] uppercase tracking-[0.08em] text-faint">
          {pick(locale, "Deploy", "部署")}
        </p>
        {gate?.passed ? (
          <>
            <p className="mb-3 text-[12.5px] leading-relaxed text-dim">
              {pick(
                locale,
                "Web never holds keys. Run this on a machine with PF_XLAYER_KEY set.",
                "Web 从不持有密钥。在设好 PF_XLAYER_KEY 的机器上执行。",
              )}
            </p>
            <button
              type="button"
              className="flex h-9 w-full items-center justify-center gap-2 rounded-lg border border-line text-[12px] text-ink hover:border-faint"
              onClick={() => {
                void copyText(deployCmd);
                toast(pick(locale, "Deploy command copied", "部署命令已复制"));
              }}
            >
              <Copy className="size-3.5" />
              {pick(locale, "Copy desktop deploy", "复制桌面部署命令")}
            </button>
          </>
        ) : (
          <p className="text-[12.5px] text-dim">{pick(locale, "Pass the gate first.", "先过门禁。")}</p>
        )}
      </div>
    </aside>
  );
}
