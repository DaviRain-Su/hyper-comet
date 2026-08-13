import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { Download, Plus, Trash2 } from "lucide-react";
import { Wordmark } from "@/components/brand/logo";
import { LocaleToggle } from "@/components/brand/locale-toggle";
import { AccountChip } from "@/components/nav/account-chip";
import { pick, useLocale } from "@/lib/i18n";
import { cn } from "@/lib/cn";
import { PROOFSHIP_RELEASES } from "@/lib/links";
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
  const search = useSearch({ strict: false }) as { relay?: string; session?: string };
  const keep = { relay: search.relay, session: search.session };

  return (
    <aside className="flex h-full min-h-0 w-full flex-col border-r border-border bg-surface">
      <div className="flex h-14 items-center justify-between px-4">
        <Wordmark />
        <LocaleToggle />
      </div>
      <div className="px-3 pb-3">
        <button
          type="button"
          onClick={onNew}
          disabled={creating}
          className="flex h-10 w-full items-center justify-center gap-2 rounded-[var(--radius-md)] bg-accent text-[13px] font-semibold text-accent-fg shadow-[var(--shadow-accent)] hover:bg-accent-hover disabled:opacity-50"
        >
          <Plus className="size-4" />
          {pick(locale, "New session", "新会话")}
        </button>
      </div>
      <nav className="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
        {sessions.length === 0 ? (
          <p className="px-3 py-6 text-[13px] text-fg-subtle">
            {pick(locale, "No sessions yet.", "还没有会话。")}
          </p>
        ) : (
          <ul className="space-y-0.5">
            {sessions.map((s) => (
              <li key={s.id} className="group relative">
                <Link
                  to="/sessions/$sessionId"
                  params={{ sessionId: s.id }}
                  search={keep}
                  className={cn(
                    "block rounded-[var(--radius-md)] px-3 py-2.5 pr-9 text-[13px] leading-snug",
                    activeId === s.id
                      ? "bg-bg text-fg"
                      : "text-fg-muted hover:bg-bg/60 hover:text-fg",
                  )}
                >
                  <span className="flex items-start gap-2">
                    <span
                      className={cn(
                        "mt-1.5 size-1.5 shrink-0 rounded-full",
                        s.status === "ready" && "bg-success",
                        s.status === "failed" && "bg-red-400",
                        s.status === "running" && "bg-accent",
                        s.status === "idle" && "bg-fg-subtle",
                      )}
                    />
                    <span className="min-w-0">
                      <span className="line-clamp-2">{s.title}</span>
                      <span className="mt-1 block font-mono text-[10px] uppercase tracking-wider text-fg-subtle">
                        {s.deviceId
                          ? `${s.status} · ${s.deviceId.slice(0, 8)}`
                          : s.status}
                      </span>
                    </span>
                  </span>
                </Link>
                <button
                  type="button"
                  aria-label={pick(locale, "Delete", "删除")}
                  className="absolute right-2 top-2.5 hidden rounded p-1 text-fg-subtle hover:text-fg group-hover:block"
                  onClick={(e) => {
                    e.preventDefault();
                    onDelete(s.id);
                    if (activeId === s.id) void navigate({ to: "/sessions", search: keep });
                  }}
                >
                  <Trash2 className="size-3.5" />
                </button>
              </li>
            ))}
          </ul>
        )}
      </nav>
      <div className="space-y-3 border-t border-border px-3 py-3">
        <a
          href={PROOFSHIP_RELEASES}
          className="flex h-10 items-center justify-center gap-2 rounded-[var(--radius-md)] border border-border bg-bg text-[12.5px] font-medium text-fg hover:border-border-strong"
        >
          <Download className="size-3.5 text-accent" />
          {pick(locale, "Download desktop", "下载桌面版")}
        </a>
        <div className="px-1">
          <AccountChip />
        </div>
      </div>
    </aside>
  );
}
