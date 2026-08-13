import { useEffect, useRef, useState } from "react";
import { Check, ChevronDown, Copy, Download, X } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { pick, useLocale } from "@/lib/i18n";
import { copyText, downloadText } from "@/lib/download";
import { extractModule, type GateResult } from "@/lib/gate";
import type { MessageRow } from "@/lib/sessions";

function GateCard({ result }: { result: GateResult }) {
  const { locale } = useLocale();
  return (
    <div className="rounded-xl border border-line bg-raise p-4">
      <div className="mb-3 flex items-center justify-between">
        <span className="text-[12px] font-medium uppercase tracking-[0.08em] text-faint">
          {pick(locale, "Gate", "门禁")}
        </span>
        <Badge variant={result.passed ? "pass" : "fail"}>
          {result.passed ? pick(locale, "passed", "通过") : pick(locale, "fail-closed", "失败关闭")}
        </Badge>
      </div>
      <ol className="space-y-2">
        {result.steps.map((step) => (
          <li key={step.id} className="flex items-start gap-2.5 text-[13px]">
            <span className="mt-0.5">
              {step.status === "pass" ? (
                <Check className="size-3.5 text-emerald-300" />
              ) : step.status === "fail" ? (
                <X className="size-3.5 text-red-300" />
              ) : (
                <span className="block size-3.5 rounded-full border border-faint" />
              )}
            </span>
            <div>
              <div className="font-mono text-[12px] text-ink">{step.label}</div>
              <div className="text-[12.5px] text-dim">{step.detail}</div>
              {step.diagnostics.map((d) => (
                <div key={`${d.code}-${d.message}`} className="mt-1 font-mono text-[11.5px] text-red-300">
                  {d.code}
                  {d.line ? `:${d.line}` : ""} — {d.message}
                </div>
              ))}
            </div>
          </li>
        ))}
      </ol>
    </div>
  );
}

function LeanFile({ source }: { source: string }) {
  const { locale } = useLocale();
  const [open, setOpen] = useState(false);
  const module = extractModule(source) ?? "program";
  const lines = source.replace(/\n$/, "").split("\n");

  return (
    <div className="overflow-hidden rounded-xl border border-line bg-bg">
      <div className="flex items-center gap-2 border-b border-line px-3 py-2">
        <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-ink">{module}.lean</span>
        <span className="font-mono text-[11px] text-faint">
          {lines.length} {pick(locale, "lines", "行")}
        </span>
        <button
          type="button"
          className="grid size-8 place-items-center text-faint hover:text-ink"
          aria-label={pick(locale, "Copy", "复制")}
          onClick={() => {
            void copyText(source);
            toast(pick(locale, "Copied source", "已复制源码"));
          }}
        >
          <Copy className="size-3.5" />
        </button>
        <button
          type="button"
          className="grid size-8 place-items-center text-faint hover:text-ink"
          aria-label={pick(locale, "Download", "下载")}
          onClick={() => downloadText(`${module}.lean`, source)}
        >
          <Download className="size-3.5" />
        </button>
      </div>
      <pre
        className={`overflow-auto p-4 font-mono text-[12px] leading-relaxed text-dim ${
          open ? "max-h-[360px]" : "max-h-[168px]"
        }`}
      >
        {source}
      </pre>
      {lines.length > 8 && (
        <button
          type="button"
          onClick={() => setOpen((v) => !v)}
          className="flex h-9 w-full items-center justify-center gap-1 border-t border-line text-[12px] text-faint hover:text-ink"
        >
          <ChevronDown className={`size-3.5 transition-transform ${open ? "rotate-180" : ""}`} />
          {open
            ? pick(locale, "Collapse", "收起")
            : pick(locale, `Show all ${lines.length} lines`, `展开全部 ${lines.length} 行`)}
        </button>
      )}
    </div>
  );
}

export function Transcript({
  messages,
  running,
}: {
  messages: MessageRow[];
  running?: boolean;
}) {
  const { locale } = useLocale();
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages.length, running]);

  return (
    <div className="mx-auto flex w-full max-w-[720px] flex-col gap-5 px-4 py-8 sm:px-6">
      {messages.map((m) => {
        if (m.kind === "gate" && m.meta) {
          return <GateCard key={m.id} result={m.meta} />;
        }
        if (m.kind === "lean") {
          return <LeanFile key={m.id} source={m.content} />;
        }
        if (m.role === "user") {
          return (
            <div
              key={m.id}
              className="ml-auto max-w-[85%] rounded-2xl bg-raise px-4 py-3 text-[14.5px] leading-relaxed text-ink"
            >
              {m.content}
            </div>
          );
        }
        return (
          <div key={m.id} className="max-w-[85%] whitespace-pre-wrap text-[14.5px] leading-relaxed text-dim">
            {m.content}
          </div>
        );
      })}
      {running && (
        <div className="text-[13px] text-purple-hi">
          {pick(locale, "Agent drafting · gate standing by…", "Agent 起草中 · 门禁待命…")}
        </div>
      )}
      <div ref={endRef} />
    </div>
  );
}
