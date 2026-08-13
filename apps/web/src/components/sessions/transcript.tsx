import { useEffect, useRef, useState } from "react";
import { Check, ChevronDown, Copy, Download, X } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { pick, useLocale } from "@/lib/i18n";
import { copyText, downloadText } from "@/lib/download";
import { extractModule, type GateResult } from "@/lib/gate";
import { LeanCode } from "@/lib/lean-highlight";
import type { MessageRow } from "@/lib/sessions";

function PixelLoader() {
  return (
    <span className="pixel-loader" aria-hidden>
      {Array.from({ length: 10 }).map((_, i) => (
        <i key={i} />
      ))}
    </span>
  );
}

function GateCard({ result }: { result: GateResult }) {
  const { locale } = useLocale();
  const [open, setOpen] = useState(true);
  return (
    <div className="overflow-hidden rounded-[var(--radius-xl)] border border-border bg-surface">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center justify-between px-4 py-3 text-left"
      >
        <span className="flex items-center gap-2 text-[12px] font-semibold tracking-[0.08em] text-fg-subtle uppercase">
          {pick(locale, "Thinking · Gate", "思考 · 门禁")}
        </span>
        <span className="flex items-center gap-2">
          <Badge variant={result.passed ? "pass" : "fail"}>
            {result.passed ? pick(locale, "passed", "通过") : pick(locale, "fail-closed", "失败关闭")}
          </Badge>
          <ChevronDown className={`size-3.5 text-fg-subtle transition-transform ${open ? "rotate-180" : ""}`} />
        </span>
      </button>
      {open && (
        <ol className="space-y-0 border-t border-border">
          {result.steps.map((step, i) => (
            <li key={step.id} className="flex items-start gap-3 border-b border-border/70 px-4 py-3 last:border-0">
              <span className="mt-0.5 font-mono text-[11px] tabular-nums text-fg-subtle">
                {String(i + 1).padStart(2, "0")}
              </span>
              <span className="mt-0.5">
                {step.status === "pass" ? (
                  <Check className="size-3.5 text-success" />
                ) : step.status === "fail" ? (
                  <X className="size-3.5 text-red-300" />
                ) : (
                  <span className="block size-3.5 rounded-full border border-fg-subtle" />
                )}
              </span>
              <div className="min-w-0 flex-1">
                <div className="font-mono text-[12px] text-fg">{step.label}</div>
                <div className="mt-0.5 text-[12.5px] text-fg-muted">{step.detail}</div>
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
      )}
    </div>
  );
}

function ToolChip({ label, detail, ok }: { label: string; detail?: string; ok?: boolean }) {
  return (
    <div className="inline-flex max-w-full items-start gap-2 rounded-[var(--radius-lg)] border border-border bg-surface px-3 py-2">
      <span className={`mt-1 size-1.5 shrink-0 rounded-full ${ok === false ? "bg-red-400" : "bg-success"}`} />
      <div className="min-w-0">
        <div className="font-mono text-[11.5px] text-fg">{label}</div>
        {detail ? <div className="mt-0.5 text-[12px] leading-relaxed text-fg-muted">{detail}</div> : null}
      </div>
    </div>
  );
}

function LeanFile({ source }: { source: string }) {
  const { locale } = useLocale();
  const [open, setOpen] = useState(false);
  const module = extractModule(source) ?? "program";
  const lines = source.replace(/\n$/, "").split("\n");

  return (
    <div className="overflow-hidden rounded-[var(--radius-xl)] border border-border bg-bg">
      <div className="flex items-center gap-2 border-b border-border px-3 py-2">
        <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-fg">{module}.lean</span>
        <span className="font-mono text-[11px] text-fg-subtle">
          {lines.length} {pick(locale, "lines", "行")}
        </span>
        <button
          type="button"
          className="grid size-8 place-items-center text-fg-subtle hover:text-fg"
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
          className="grid size-8 place-items-center text-fg-subtle hover:text-fg"
          aria-label={pick(locale, "Download", "下载")}
          onClick={() => downloadText(`${module}.lean`, source)}
        >
          <Download className="size-3.5" />
        </button>
      </div>
      <div className={`overflow-auto p-4 ${open ? "max-h-[360px]" : "max-h-[168px]"}`}>
        <LeanCode source={source} />
      </div>
      {lines.length > 8 && (
        <button
          type="button"
          onClick={() => setOpen((v) => !v)}
          className="flex h-9 w-full items-center justify-center gap-1 border-t border-border text-[12px] text-fg-subtle hover:text-fg"
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
    <div className="mx-auto flex w-full max-w-[760px] flex-col gap-4 px-4 py-8 sm:px-6">
      {messages.map((m) => {
        if (m.kind === "gate" && m.meta) {
          return <GateCard key={m.id} result={m.meta} />;
        }
        if (m.kind === "lean") {
          return <LeanFile key={m.id} source={m.content} />;
        }
        if (m.role === "user") {
          const steered = m.content.startsWith("steer · ");
          return (
            <div key={m.id} className="ml-auto max-w-[85%]">
              {steered ? (
                <p className="mb-1 text-right text-[10px] font-semibold tracking-[0.1em] text-accent uppercase">
                  Steer
                </p>
              ) : null}
              <div className="rounded-2xl bg-surface-elevated px-4 py-3 text-[14.5px] leading-relaxed text-fg">
                {steered ? m.content.slice(8) : m.content}
              </div>
            </div>
          );
        }
        if (m.role === "tool") {
          return <ToolChip key={m.id} label={m.content.slice(0, 80)} detail={m.content} ok />;
        }
        if (m.role === "system") {
          const fail = /error|errored|refused|fail/i.test(m.content);
          return (
            <div
              key={m.id}
              className={`max-w-[92%] rounded-[var(--radius-lg)] border px-3 py-2 font-mono text-[12px] leading-relaxed ${
                fail ? "border-red-500/25 bg-red-500/5 text-red-200" : "border-border text-fg-subtle"
              }`}
            >
              {m.content}
            </div>
          );
        }
        return (
          <div key={m.id} className="max-w-[88%] whitespace-pre-wrap text-[14.5px] leading-relaxed text-fg-muted">
            {m.content}
          </div>
        );
      })}
      {running && (
        <div className="flex items-center gap-3 text-[13px] text-accent">
          <PixelLoader />
          {pick(locale, "Waiting on your desktop…", "等待你的桌面…")}
        </div>
      )}
      <div ref={endRef} />
    </div>
  );
}
