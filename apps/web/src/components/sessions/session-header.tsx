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
    <span className="inline-flex items-center gap-2 rounded-lg border border-border bg-bg px-2.5 py-1.5">
      <span
        className={cn(
          "size-1.5 shrink-0 rounded-full",
          tone === "on" && "bg-success",
          tone === "off" && "bg-fg-subtle",
          tone === "run" && "lamp-run bg-accent",
          tone === "fail" && "bg-red-400",
        )}
      />
      <span className="grid leading-none">
        <span className="text-[9px] font-medium uppercase tracking-[0.08em] text-fg-subtle">{label}</span>
        <span className="mt-0.5 font-mono text-[11px] text-fg">{value}</span>
      </span>
    </span>
  );
}

export function SessionHeader({
  session,
  desktopOnline,
  platformOnline,
  relayLive,
  connecting,
  onRename,
  onMenu,
  onRail,
}: {
  session: SessionRow | null;
  desktopOnline?: boolean;
  platformOnline?: boolean;
  relayLive?: boolean;
  connecting?: boolean;
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
  const gateTone = !gate ? "off" : gate.passed ? "on" : "fail";
  const gateValue = !gate
    ? pick(locale, "on desktop", "在桌面")
    : gate.passed
      ? pick(locale, "pass", "通过")
      : pick(locale, "closed", "关闭");

  return (
    <header className="flex h-14 shrink-0 items-center gap-2 border-b border-border bg-surface/70 px-3 backdrop-blur-sm sm:px-4">
      {onMenu && (
        <button
          type="button"
          className="grid size-10 place-items-center text-fg lg:hidden"
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
            className="h-8 w-full max-w-md rounded-md border border-border bg-bg px-2 text-[14px] font-medium text-fg outline-none focus:border-accent/50"
          />
        ) : (
          <button
            type="button"
            disabled={!session || !onRename}
            onClick={() => setEditing(true)}
            className="block max-w-full truncate text-left text-[14px] font-medium text-fg disabled:cursor-default"
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
        <Lamp label={pick(locale, "Gate", "门禁")} value={gateValue} tone={gateTone} />
      </div>

      {onRail && (
        <button
          type="button"
          className="grid size-10 place-items-center text-fg xl:hidden"
          onClick={onRail}
          aria-label="Ops"
        >
          <PanelRight className="size-5" />
        </button>
      )}
    </header>
  );
}
