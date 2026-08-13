import { useEffect, useState } from "react";
import { Menu, PanelRight } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { pick, useLocale } from "@/lib/i18n";
import { cn } from "@/lib/cn";
import type { SessionRow } from "@/lib/sessions";

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
    <span className="inline-flex items-center gap-2 rounded-lg border border-line bg-bg px-2.5 py-1.5">
      <span
        className={cn(
          "size-1.5 shrink-0 rounded-full",
          tone === "on" && "bg-emerald-400",
          tone === "off" && "bg-faint",
          tone === "run" && "lamp-run bg-purple-hi",
          tone === "fail" && "bg-red-400",
        )}
      />
      <span className="grid leading-none">
        <span className="text-[9px] font-medium uppercase tracking-[0.08em] text-faint">{label}</span>
        <span className="mt-0.5 font-mono text-[11px] text-ink">{value}</span>
      </span>
    </span>
  );
}

export function SessionHeader({
  session,
  running,
  onRename,
  onMenu,
  onRail,
}: {
  session: SessionRow | null;
  running?: boolean;
  onRename?: (title: string) => void;
  onMenu?: () => void;
  onRail?: () => void;
}) {
  const { locale } = useLocale();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(session?.title ?? "");

  useEffect(() => {
    setDraft(session?.title ?? "");
    setEditing(false);
  }, [session?.id, session?.title]);

  const commit = () => {
    setEditing(false);
    const next = draft.replace(/\s+/g, " ").trim();
    if (!next || !session || next === session.title) {
      setDraft(session?.title ?? "");
      return;
    }
    onRename?.(next);
  };

  const gate = session?.gate ?? null;
  const agentTone = running ? "run" : session?.source ? "on" : "off";
  const agentValue = running
    ? pick(locale, "drafting", "起草中")
    : session?.source
      ? pick(locale, "local", "本地")
      : pick(locale, "idle", "待命");
  const gateTone = !gate ? "off" : gate.passed ? "on" : "fail";
  const gateValue = !gate
    ? pick(locale, "idle", "待命")
    : gate.passed
      ? pick(locale, "pass", "通过")
      : pick(locale, "closed", "关闭");

  return (
    <header className="flex h-14 shrink-0 items-center gap-2 border-b border-line px-3 sm:px-4">
      {onMenu && (
        <button
          type="button"
          className="grid size-10 place-items-center text-ink lg:hidden"
          onClick={onMenu}
          aria-label="Sessions"
        >
          <Menu className="size-5" />
        </button>
      )}

      <div className="min-w-0 flex-1">
        {editing && session ? (
          <input
            autoFocus
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              if (e.key === "Enter") commit();
              if (e.key === "Escape") {
                setDraft(session.title);
                setEditing(false);
              }
            }}
            className="h-8 w-full max-w-md rounded-md border border-line bg-bg px-2 text-[14px] font-medium text-ink outline-none focus:border-purple/50"
          />
        ) : (
          <button
            type="button"
            disabled={!session || !onRename}
            onClick={() => setEditing(true)}
            className="block max-w-full truncate text-left text-[14px] font-medium text-ink disabled:cursor-default"
            title={pick(locale, "Rename session", "重命名会话")}
          >
            {session?.title ?? pick(locale, "Sessions", "Sessions")}
          </button>
        )}
      </div>

      {session?.status && session.status !== "idle" && (
        <Badge
          variant={session.status === "ready" ? "pass" : session.status === "failed" ? "fail" : "run"}
          className="hidden sm:inline-flex"
        >
          {session.status}
        </Badge>
      )}

      <div className="hidden items-center gap-1.5 md:flex">
        <Lamp label="Agent" value={agentValue} tone={agentTone} />
        <Lamp label={pick(locale, "Gate", "门禁")} value={gateValue} tone={gateTone} />
        <Lamp label={pick(locale, "Keys", "密钥")} value={pick(locale, "never here", "不在此")} tone="off" />
      </div>

      {onRail && (
        <button
          type="button"
          className="grid size-10 place-items-center text-ink xl:hidden"
          onClick={onRail}
          aria-label="Gate"
        >
          <PanelRight className="size-5" />
        </button>
      )}
    </header>
  );
}
