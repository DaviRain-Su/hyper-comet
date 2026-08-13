import { useState } from "react";
import { ArrowUp, MessageSquare, Navigation, Square } from "lucide-react";
import { Textarea } from "@/components/ui/textarea";
import { pick, useLocale } from "@/lib/i18n";
import { cn } from "@/lib/cn";
import type { PromptMode } from "@/lib/use-desktop-link";

export function Composer({
  disabled,
  onSend,
  onCancel,
  desktopOnline,
  running,
}: {
  disabled?: boolean;
  onSend: (text: string, mode: PromptMode) => void;
  onCancel?: () => void;
  desktopOnline?: boolean;
  running?: boolean;
}) {
  const { locale } = useLocale();
  const [value, setValue] = useState("");
  const [mode, setMode] = useState<PromptMode>("prompt");
  const locked = disabled || !desktopOnline;

  const submit = () => {
    const t = value.trim();
    if (!t || locked) return;
    onSend(t, mode);
    setValue("");
  };

  const modes: { id: PromptMode; label: string }[] = [
    { id: "prompt", label: pick(locale, "Send", "发送") },
    { id: "steer", label: pick(locale, "Steer", "纠偏") },
    { id: "comment", label: pick(locale, "Comment", "批注") },
  ];

  const placeholder =
    mode === "steer"
      ? pick(locale, "Steer the running agent…", "给正在跑的 agent 纠偏…")
      : mode === "comment"
        ? pick(locale, "Leave a note on the transcript…", "在记录上留一条批注…")
        : desktopOnline
          ? pick(locale, "Prompt your local agent…", "向本机 agent 发提示…")
          : pick(locale, "Attach desktop to send…", "先连接桌面再发送…");

  return (
    <div className="border-t border-border bg-bg/85 px-3 py-3 backdrop-blur-sm sm:px-5">
      <div className="mx-auto max-w-[760px]">
        <div className="mb-2 flex flex-wrap items-center gap-1.5">
          {modes.map((m) => (
            <button
              key={m.id}
              type="button"
              onClick={() => setMode(m.id)}
              className={cn(
                "inline-flex h-8 items-center gap-1.5 rounded-full border px-3 text-[12px] font-medium transition-colors",
                mode === m.id
                  ? "border-accent/40 bg-accent/15 text-fg"
                  : "border-border bg-surface text-fg-muted hover:text-fg",
              )}
            >
              {m.id === "steer" ? (
                <Navigation className="size-3" />
              ) : m.id === "comment" ? (
                <MessageSquare className="size-3" />
              ) : null}
              {m.label}
            </button>
          ))}
          {running && onCancel ? (
            <button
              type="button"
              onClick={onCancel}
              className="ml-auto inline-flex h-8 items-center gap-1.5 rounded-full border border-border px-3 text-[12px] text-fg-muted hover:text-fg"
            >
              <Square className="size-3" />
              {pick(locale, "Cancel run", "取消运行")}
            </button>
          ) : null}
        </div>
        <div className="flex items-end gap-2 rounded-2xl border border-border bg-surface p-2 pl-3">
          <Textarea
            value={value}
            disabled={locked}
            rows={2}
            placeholder={placeholder}
            className="min-h-[52px] flex-1 border-0 bg-transparent px-0 py-2 focus-visible:ring-0"
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                submit();
              }
            }}
          />
          <button
            type="button"
            disabled={locked || !value.trim()}
            onClick={submit}
            className="grid size-10 shrink-0 place-items-center rounded-xl bg-accent text-accent-fg shadow-[var(--shadow-accent)] hover:bg-accent-hover disabled:opacity-40"
            aria-label={pick(locale, "Send", "发送")}
          >
            <ArrowUp className="size-4" />
          </button>
        </div>
        <p className="mt-2 px-1 text-[11px] text-fg-subtle">
          {desktopOnline
            ? pick(
                locale,
                "Send / steer / comment ride the relay. This page never runs the agent or holds keys.",
                "发送 / 纠偏 / 批注经中继到桌面。这个页面不跑 agent，也不持有密钥。",
              )
            : pick(
                locale,
                "Desktop offline. Open ProofShip on your computer — we will not draft in the cloud.",
                "桌面离线。在你的电脑上打开 ProofShip — 我们不会在云端起草。",
              )}
        </p>
      </div>
    </div>
  );
}
