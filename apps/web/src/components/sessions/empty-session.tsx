import { TEMPLATES } from "@/lib/templates";
import { pick, useLocale } from "@/lib/i18n";

export function EmptySession({
  onTemplate,
  busy,
}: {
  onTemplate: (id: string) => void;
  busy?: boolean;
}) {
  const { locale } = useLocale();
  return (
    <div className="mx-auto flex w-full max-w-[720px] flex-col px-5 py-16">
      <p className="text-[11px] font-semibold tracking-[0.14em] text-purple-hi">Sessions</p>
      <h1 className="mt-3 text-balance text-[28px] font-semibold tracking-[-0.03em] text-ink">
        {pick(locale, "What should the gate see?", "门禁要看什么合约？")}
      </h1>
      <p className="mt-3 max-w-[46ch] text-[14.5px] leading-relaxed text-dim">
        {pick(
          locale,
          "Describe a contract in the composer, or start from a starter. Your agent drafts ProgramV1. The gate decides if it ships.",
          "在下方描述合约，或从模板开始。Agent 起草 ProgramV1，门禁决定能不能上链。",
        )}
      </p>
      <ul className="mt-10 grid gap-3 sm:grid-cols-3">
        {TEMPLATES.map((t) => (
          <li key={t.id}>
            <button
              type="button"
              disabled={busy}
              onClick={() => onTemplate(t.id)}
              className="h-full w-full rounded-xl border border-line bg-raise p-4 text-left transition-colors hover:border-faint disabled:opacity-50"
            >
              <div className="font-mono text-[11px] text-purple-hi">{t.module}</div>
              <div className="mt-2 text-[14px] font-medium text-ink">
                {locale === "zh" ? t.titleZh : t.title}
              </div>
              <p className="mt-1.5 text-[12.5px] leading-relaxed text-dim">
                {locale === "zh" ? t.blurbZh : t.blurb}
              </p>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
