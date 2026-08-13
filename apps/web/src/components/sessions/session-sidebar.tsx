import { Link, useNavigate } from "@tanstack/react-router";
import { Plus, Trash2 } from "lucide-react";
import { Wordmark } from "@/components/brand/logo";
import { LocaleToggle } from "@/components/brand/locale-toggle";
import { AccountChip } from "@/components/nav/account-chip";
import { pick, useLocale } from "@/lib/i18n";
import { cn } from "@/lib/cn";
import type { SessionRow } from "@/lib/sessions";

export function SessionSidebar({
  sessions,
  activeId,
  onNew,
  onDelete,
  creating,
}: {
  sessions: SessionRow[];
  activeId?: string;
  onNew: () => void;
  onDelete: (id: string) => void;
  creating?: boolean;
}) {
  const { locale } = useLocale();
  const navigate = useNavigate();

  return (
    <aside className="flex h-full min-h-0 w-full flex-col border-r border-line bg-raise">
      <div className="flex h-14 items-center justify-between px-4">
        <Wordmark />
        <LocaleToggle />
      </div>
      <div className="px-3 pb-3">
        <button
          type="button"
          onClick={onNew}
          disabled={creating}
          className="flex h-10 w-full items-center justify-center gap-2 rounded-lg border border-line bg-bg text-[13px] font-medium text-ink hover:border-faint disabled:opacity-50"
        >
          <Plus className="size-4" />
          {pick(locale, "New session", "新会话")}
        </button>
      </div>
      <nav className="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
        {sessions.length === 0 ? (
          <p className="px-3 py-6 text-[13px] text-faint">
            {pick(locale, "No sessions yet.", "还没有会话。")}
          </p>
        ) : (
          <ul className="space-y-0.5">
            {sessions.map((s) => (
              <li key={s.id} className="group relative">
                <Link
                  to="/sessions/$sessionId"
                  params={{ sessionId: s.id }}
                  className={cn(
                    "block rounded-lg px-3 py-2.5 pr-9 text-[13px] leading-snug",
                    activeId === s.id ? "bg-bg text-ink" : "text-dim hover:bg-bg/60 hover:text-ink",
                  )}
                >
                  <span className="flex items-start gap-2">
                    <span
                      className={cn(
                        "mt-1.5 size-1.5 shrink-0 rounded-full",
                        s.status === "ready" && "bg-emerald-400",
                        s.status === "failed" && "bg-red-400",
                        s.status === "running" && "bg-purple-hi",
                        s.status === "idle" && "bg-faint",
                      )}
                    />
                    <span className="min-w-0">
                      <span className="line-clamp-2">{s.title}</span>
                      <span className="mt-1 block font-mono text-[10px] uppercase tracking-wider text-faint">
                        {s.status}
                      </span>
                    </span>
                  </span>
                </Link>
                <button
                  type="button"
                  aria-label={pick(locale, "Delete", "删除")}
                  className="absolute right-2 top-2.5 hidden rounded p-1 text-faint hover:text-ink group-hover:block"
                  onClick={(e) => {
                    e.preventDefault();
                    onDelete(s.id);
                    if (activeId === s.id) void navigate({ to: "/sessions" });
                  }}
                >
                  <Trash2 className="size-3.5" />
                </button>
              </li>
            ))}
          </ul>
        )}
      </nav>
      <div className="border-t border-line px-4 py-3">
        <AccountChip />
      </div>
    </aside>
  );
}
