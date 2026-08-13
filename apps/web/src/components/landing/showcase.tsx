import { Apple, Download, Monitor, Terminal } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { ProductDemo, ShowcaseEnterButton } from "@/components/landing/product-demo";
import { PROOFSHIP_README, PROOFSHIP_RELEASES } from "@/lib/links";

export function ShowcaseSection() {
  const { t } = useI18n();
  return (
    <section id="showcase" className="scroll-mt-20 border-t border-border bg-bg py-16 sm:py-20">
      <div className="mx-auto max-w-[1280px] px-5 sm:px-8">
        <div className="max-w-2xl">
          <p className="mb-3 text-[12px] font-semibold tracking-[0.14em] text-accent uppercase">
            {t.showcaseKicker}
          </p>
          <h2 className="text-[1.65rem] font-semibold tracking-tight sm:text-[1.9rem]">{t.showcaseTitle}</h2>
          <p className="mt-3 text-[15px] leading-relaxed text-fg-muted sm:text-[16px]">{t.showcaseLead}</p>
        </div>

        <div className="relative mt-10">
          <div
            className="pointer-events-none absolute inset-x-0 top-[18%] h-[85%] bg-[radial-gradient(55%_60%_at_50%_35%,rgba(240,90,40,0.16),transparent_70%)]"
            aria-hidden
          />
          <div className="relative">
            <ProductDemo />
          </div>
          <p className="mt-3 text-center text-[13px] text-fg-subtle">{t.showcaseCaption}</p>
        </div>

        <div className="mt-8 flex flex-col items-start gap-3 sm:flex-row">
          <ShowcaseEnterButton />
          <Button asChild variant="outline">
            <a href={PROOFSHIP_RELEASES}>
              <Download className="size-4" />
              {t.ctaDesktop}
            </a>
          </Button>
        </div>
      </div>
    </section>
  );
}

export function DownloadSection() {
  const { t } = useI18n();
  const packs = [
    { icon: Apple, name: t.downloadMac, hint: t.downloadMacHint },
    { icon: Monitor, name: t.downloadWin, hint: t.downloadWinHint },
    { icon: Terminal, name: t.downloadLinux, hint: t.downloadLinuxHint },
  ];

  return (
    <section id="download" className="scroll-mt-20 border-y border-border bg-surface py-16 sm:py-20">
      <div className="mx-auto max-w-[1280px] px-5 sm:px-8">
        <div className="max-w-2xl">
          <h2 className="text-[1.65rem] font-semibold tracking-tight sm:text-[1.9rem]">{t.downloadTitle}</h2>
          <p className="mt-3 text-[15px] leading-relaxed text-fg-muted sm:text-[16px]">{t.downloadLead}</p>
        </div>

        <div className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {packs.map(({ icon: Icon, name, hint }) => (
            <a
              key={name}
              href={PROOFSHIP_RELEASES}
              className="group rounded-[var(--radius-xl)] border border-border bg-bg p-6 transition-colors hover:border-border-strong"
            >
              <div className="inline-flex size-10 items-center justify-center rounded-[var(--radius-md)] border border-border bg-surface text-accent">
                <Icon className="size-5" strokeWidth={1.75} />
              </div>
              <h3 className="mt-4 text-[1.05rem] font-semibold tracking-tight">{name}</h3>
              <p className="mt-1 text-[13px] text-fg-muted">{hint}</p>
              <p className="mt-4 text-[13px] font-medium text-accent group-hover:text-accent-hover">
                {t.downloadGet} →
              </p>
            </a>
          ))}
          <a
            href={PROOFSHIP_README}
            target="_blank"
            rel="noreferrer"
            className="group rounded-[var(--radius-xl)] border border-border bg-bg p-6 transition-colors hover:border-border-strong"
          >
            <div className="inline-flex size-10 items-center justify-center rounded-[var(--radius-md)] border border-border bg-surface text-fg">
              <Terminal className="size-5" strokeWidth={1.75} />
            </div>
            <h3 className="mt-4 text-[1.05rem] font-semibold tracking-tight">{t.downloadSource}</h3>
            <p className="mt-1 font-mono text-[13px] text-fg-muted">{t.downloadSourceHint}</p>
            <p className="mt-4 text-[13px] font-medium text-fg-muted group-hover:text-fg">{t.downloadSoon}</p>
          </a>
        </div>
      </div>
    </section>
  );
}
