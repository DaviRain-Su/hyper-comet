import { TEMPLATES } from "@/lib/templates";
import { pick, useLocale } from "@/lib/i18n";
import { PairingCard } from "@/components/sessions/desktop-link";
import type { useDesktopLink } from "@/lib/use-desktop-link";

export function EmptySession({
  onTemplate,
  busy,
  link,
}: {
  onTemplate: (id: string) => void;
  busy?: boolean;
  link: ReturnType<typeof useDesktopLink>;
}) {
  const { locale } = useLocale();
  return (
    <div className="mx-auto flex w-full max-w-[720px] flex-col px-5 py-10">
      <p className="text-[11px] font-semibold tracking-[0.14em] text-accent">Sessions</p>
      <h1 className="mt-3 text-balance font-display text-[2rem] text-fg">
        {pick(locale, "Drive the machine in front of you.", "远程驱动你的电脑。")}
      </h1>
      <p className="mt-3 max-w-[48ch] text-[14.5px] leading-relaxed text-fg-muted">
        {pick(
          locale,
          "Web is the remote panel. Agent + gate stay on your desktop. Starters below are read-only previews — they do not call a cloud model.",
          "Web 是远程面板。Agent 和门禁留在桌面。下面的模板只是只读预览——不会调用云端模型。",
        )}
      </p>

      <div className="mt-8">
        <PairingCard link={link} />
      </div>

      <p className="mt-10 text-[11px] font-semibold uppercase tracking-[0.1em] text-fg-subtle">
        {pick(locale, "Read-only starters", "只读模板")}
      </p>
      <ul className="mt-3 grid gap-3 sm:grid-cols-3">
        {TEMPLATES.map((t) => (
          <li key={t.id}>
            <button
              type="button"
              disabled={busy}
              onClick={() => onTemplate(t.id)}
              className="h-full w-full rounded-[var(--radius-xl)] border border-border bg-surface p-4 text-left transition-colors hover:border-border-strong disabled:opacity-50"
            >
              <div className="font-mono text-[11px] text-accent">{t.module}</div>
              <div className="mt-2 text-[14px] font-medium text-fg">
                {locale === "zh" ? t.titleZh : t.title}
              </div>
              <p className="mt-1.5 text-[12.5px] leading-relaxed text-fg-muted">
                {locale === "zh" ? t.blurbZh : t.blurb}
              </p>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
