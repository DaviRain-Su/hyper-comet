import { Link } from "@tanstack/react-router";
import {
  ArrowRight,
  Bot,
  ExternalLink,
  GitBranch,
  Lock,
  ShieldCheck,
  Terminal,
  Cpu,
  Layers,
  Ban,
  Unplug,
  Globe2,
  Server,
  Check,
  Cloud,
  FileCode2,
  ScrollText,
  Route,
  Workflow,
} from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { Button } from "@/components/ui/button";

import { PROOF_FORGE_REPO, PROOF_FORGE_SITE, PROOFSHIP_RELEASES, PROOFSHIP_REPO } from "@/lib/links";

const TARGETS = [
  "EVM",
  "Solana",
  "NEAR",
  "Noir",
  "Aleo",
  "Psy",
  "Quint",
  "CosmWasm",
  "TON",
];

export function ProductSection() {
  const { t } = useI18n();
  const features = [
    { icon: Unplug, title: t.feature1Title, body: t.feature1Body },
    { icon: ShieldCheck, title: t.feature2Title, body: t.feature2Body },
    { icon: Globe2, title: t.feature3Title, body: t.feature3Body },
    { icon: Lock, title: t.feature4Title, body: t.feature4Body },
  ];

  return (
    <section id="product" className="scroll-mt-20 border-y border-border bg-surface py-16 sm:py-20">
      <div className="mx-auto max-w-[1280px] px-5 sm:px-8">
        <div className="max-w-2xl">
          <h2 className="text-[1.65rem] font-semibold tracking-tight text-fg sm:text-[1.9rem]">
            {t.productTitle}
          </h2>
          <p className="mt-3 text-[15px] leading-relaxed text-fg-muted sm:text-[16px]">
            {t.productLead}
          </p>
        </div>
        <div className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {features.map(({ icon: Icon, title, body }) => (
            <article
              key={title}
              className="group rounded-[var(--radius-xl)] border border-border bg-bg p-6 transition-colors hover:border-border-strong"
            >
              <div className="inline-flex size-10 items-center justify-center rounded-[var(--radius-md)] border border-border bg-surface text-fg">
                <Icon className="size-5" strokeWidth={1.75} />
              </div>
              <h3 className="mt-4 text-[1.05rem] font-semibold tracking-tight">{title}</h3>
              <p className="mt-2 text-[14px] leading-relaxed text-fg-muted">{body}</p>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}

export function DiffSection() {
  const { t } = useI18n();
  const items = [
    { icon: Globe2, title: t.diff1Title, body: t.diff1Body },
    { icon: Bot, title: t.diff2Title, body: t.diff2Body },
    { icon: ShieldCheck, title: t.diff3Title, body: t.diff3Body },
    { icon: Server, title: t.diff4Title, body: t.diff4Body },
  ];
  const rows = [
    { k: t.tableRowScope, a: t.tableRowScopeA, b: t.tableRowScopeB },
    { k: t.tableRowChain, a: t.tableRowChainA, b: t.tableRowChainB },
    { k: t.tableRowAgent, a: t.tableRowAgentA, b: t.tableRowAgentB },
    { k: t.tableRowFormal, a: t.tableRowFormalA, b: t.tableRowFormalB },
    { k: t.tableRowShip, a: t.tableRowShipA, b: t.tableRowShipB },
    { k: t.tableRowPay, a: t.tableRowPayA, b: t.tableRowPayB },
  ];

  return (
    <section id="difference" className="scroll-mt-20 py-16 sm:py-20">
      <div className="mx-auto max-w-[1280px] px-5 sm:px-8">
        <div className="max-w-2xl">
          <h2 className="text-[1.65rem] font-semibold tracking-tight sm:text-[1.9rem]">
            {t.diffTitle}
          </h2>
          <p className="mt-3 text-[15px] leading-relaxed text-fg-muted sm:text-[16px]">
            {t.diffLead}
          </p>
        </div>

        <div className="mt-10 grid gap-4 md:grid-cols-2">
          {items.map(({ icon: Icon, title, body }) => (
            <article
              key={title}
              className="flex gap-4 rounded-[var(--radius-xl)] border border-border bg-surface p-5 sm:p-6"
            >
              <div className="inline-flex size-10 shrink-0 items-center justify-center rounded-[var(--radius-md)] border border-border bg-bg text-accent">
                <Icon className="size-5" strokeWidth={1.75} />
              </div>
              <div>
                <h3 className="text-[1.05rem] font-semibold tracking-tight">{title}</h3>
                <p className="mt-2 text-[14px] leading-relaxed text-fg-muted">{body}</p>
              </div>
            </article>
          ))}
        </div>

        <div className="mt-8 overflow-x-auto rounded-[var(--radius-xl)] border border-border bg-surface-elevated">
          <table className="w-full min-w-[640px] text-left text-[13.5px]">
            <thead>
              <tr className="border-b border-border text-fg-muted">
                <th className="px-5 py-3.5 font-medium sm:px-6"> </th>
                <th className="px-5 py-3.5 font-semibold text-fg sm:px-6">{t.tableColUs}</th>
                <th className="px-5 py-3.5 font-medium sm:px-6">{t.tableColThem}</th>
              </tr>
            </thead>
            <tbody className="text-fg-muted">
              {rows.map((row) => (
                <tr key={row.k} className="border-b border-border/70 last:border-0">
                  <td className="px-5 py-3.5 font-medium text-fg sm:px-6">{row.k}</td>
                  <td className="px-5 py-3.5 text-fg sm:px-6">{row.a}</td>
                  <td className="px-5 py-3.5 sm:px-6">{row.b}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}

export function WorkflowSection() {
  const { t } = useI18n();
  const steps = [
    { n: "01", title: t.step1, body: t.step1Body },
    { n: "02", title: t.step2, body: t.step2Body },
    { n: "03", title: t.step3, body: t.step3Body },
    { n: "04", title: t.step4, body: t.step4Body },
  ];

  return (
    <section id="workflow" className="scroll-mt-20 border-y border-border bg-surface py-16 sm:py-20">
      <div className="mx-auto max-w-[1280px] px-5 sm:px-8">
        <div className="max-w-2xl">
          <h2 className="text-[1.65rem] font-semibold tracking-tight sm:text-[1.9rem]">
            {t.howTitle}
          </h2>
          <p className="mt-3 text-[15px] leading-relaxed text-fg-muted sm:text-[16px]">
            {t.howLead}
          </p>
        </div>

        <div className="mt-10 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {steps.map((s, i) => (
            <div
              key={s.n}
              className="relative rounded-[var(--radius-xl)] border border-border bg-bg p-5 sm:p-6"
            >
              <span className="font-mono text-[12px] font-medium tabular-nums text-accent">
                {s.n}
              </span>
              <h3 className="mt-2 text-[1.05rem] font-semibold tracking-tight">{s.title}</h3>
              <p className="mt-2 text-[14px] leading-relaxed text-fg-muted">{s.body}</p>
              {i < steps.length - 1 && (
                <span
                  className="absolute -right-2 top-1/2 z-10 hidden -translate-y-1/2 text-fg-subtle lg:block"
                  aria-hidden
                >
                  <ArrowRight className="size-4" />
                </span>
              )}
            </div>
          ))}
        </div>

        <div className="mt-8 overflow-x-auto rounded-[var(--radius-xl)] border border-border bg-surface-elevated">
          <pre className="min-w-[560px] p-5 font-mono text-[12.5px] leading-relaxed text-fg-muted sm:p-6 sm:text-[13px]">
            <code>
              <span className="text-fg-subtle">
                # gate path · BYO agent · theorem-aware
              </span>
              {"\n"}
              <span className="text-accent">NL</span>
              {"  →  your agent (Lean ProgramV1 + theorems)  →  "}
              <span className="text-success">check</span>
              {"  →  "}
              <span className="text-success">build</span>
              {"  →  "}
              <span className="text-success">inspect</span>
              {"\n"}
              {"     check: PF-* diagnostics · same-file theorem certification"}
              {"\n"}
              {"                                        └─ fail closed → "}
              <span className="text-warn">zero artifacts</span>
              {"\n"}
              {"                                        └─ pass → multi-target deploy"}
            </code>
          </pre>
        </div>
      </div>
    </section>
  );
}

export function ProofForgeSection() {
  const { t } = useI18n();
  const formalCards = [
    { icon: FileCode2, title: t.formal1Title, body: t.formal1Body },
    { icon: ScrollText, title: t.formal2Title, body: t.formal2Body },
    { icon: Route, title: t.formal3Title, body: t.formal3Body },
    { icon: Workflow, title: t.formal4Title, body: t.formal4Body },
  ];

  return (
    <section id="proofforge" className="scroll-mt-20 py-16 sm:py-20">
      <div className="mx-auto max-w-[1280px] px-5 sm:px-8">
        <div className="grid gap-10 lg:grid-cols-[1.15fr_0.85fr] lg:items-center">
          <div>
            <p className="mb-3 inline-flex items-center gap-2 text-[12px] font-semibold tracking-wide text-accent uppercase">
              <Cpu className="size-3.5" />
              Kernel
            </p>
            <h2 className="text-[1.65rem] font-semibold tracking-tight sm:text-[1.9rem]">
              {t.forgeTitle}
            </h2>
            <p className="mt-3 max-w-xl text-[15px] leading-relaxed text-fg-muted sm:text-[16px]">
              {t.forgeLead}
            </p>
            <div className="mt-6 flex flex-col gap-3 sm:flex-row sm:flex-wrap">
              <Button asChild>
                <a href={PROOF_FORGE_SITE} target="_blank" rel="noreferrer">
                  {t.forgeCtaSite}
                  <ExternalLink className="size-4" />
                </a>
              </Button>
              <Button asChild variant="secondary">
                <a href={PROOF_FORGE_REPO} target="_blank" rel="noreferrer">
                  {t.forgeCtaRepo}
                  <GitBranch className="size-4" />
                </a>
              </Button>
            </div>
          </div>

          <div className="grid gap-3">
            <div className="rounded-[var(--radius-xl)] border border-border bg-surface p-5">
              <div className="flex items-center gap-2 text-[13px] font-semibold text-fg">
                <Layers className="size-4 text-accent" />
                {t.forgeTargets}
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {TARGETS.map((name) => (
                  <span
                    key={name}
                    className="rounded-full border border-border bg-bg px-2.5 py-1 font-mono text-[12px] text-fg-muted"
                  >
                    {name}
                  </span>
                ))}
              </div>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="rounded-[var(--radius-xl)] border border-border bg-surface p-5">
                <div className="flex items-center gap-2 text-[13px] font-semibold">
                  <Ban className="size-4 text-accent" />
                  {t.forgeFail}
                </div>
                <p className="mt-2 text-[13px] leading-relaxed text-fg-muted">
                  Fail closed · no best-effort fallback
                </p>
              </div>
              <div className="rounded-[var(--radius-xl)] border border-border bg-surface p-5">
                <div className="flex items-center gap-2 text-[13px] font-semibold">
                  <ShieldCheck className="size-4 text-accent" />
                  {t.forgeLean}
                </div>
                <p className="mt-2 text-[13px] leading-relaxed text-fg-muted">
                  Lean 4 multi-target compiler
                </p>
              </div>
            </div>
          </div>
        </div>

        {/* Formal verification capability */}
        <div
          id="formal"
          className="mt-12 scroll-mt-20 rounded-[var(--radius-2xl)] border border-border-strong bg-surface p-6 sm:p-8 lg:p-10"
        >
          <div className="flex flex-wrap items-center gap-2">
            <span className="inline-flex size-9 items-center justify-center rounded-[var(--radius-md)] border border-border bg-bg text-accent">
              <ScrollText className="size-4" strokeWidth={1.75} />
            </span>
            <p className="text-[12px] font-semibold tracking-wide text-accent uppercase">
              Formal methods
            </p>
          </div>
          <h3 className="mt-3 text-[1.35rem] font-semibold tracking-tight sm:text-[1.55rem]">
            {t.formalTitle}
          </h3>
          <p className="mt-3 max-w-3xl text-[15px] leading-relaxed text-fg-muted sm:text-[16px]">
            {t.formalLead}
          </p>

          <div className="mt-8 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {formalCards.map(({ icon: Icon, title, body }) => (
              <article
                key={title}
                className="rounded-[var(--radius-xl)] border border-border bg-bg p-5"
              >
                <div className="inline-flex size-9 items-center justify-center rounded-[var(--radius-md)] border border-border bg-surface text-fg">
                  <Icon className="size-4" strokeWidth={1.75} />
                </div>
                <h4 className="mt-3 text-[0.98rem] font-semibold tracking-tight">{title}</h4>
                <p className="mt-2 text-[13.5px] leading-relaxed text-fg-muted">{body}</p>
              </article>
            ))}
          </div>

          <div className="mt-6 overflow-x-auto rounded-[var(--radius-xl)] border border-border bg-bg">
            <pre className="min-w-[520px] p-4 font-mono text-[12px] leading-relaxed text-fg-muted sm:p-5 sm:text-[12.5px]">
              <code>
                <span className="text-fg-subtle">
                  {"// ProgramV1 — theorems ride with the contract source"}
                </span>
                {"\n"}
                <span className="text-accent">program</span>
                {" StateCell "}
                <span className="text-accent">where</span>
                {"\n"}
                {"  state count : UInt64"}
                {"\n"}
                {"  entry increment(delta : UInt64) : UInt64 "}
                <span className="text-accent">do</span>
                {"\n"}
                {"    count := count + delta"}
                {"\n"}
                {"    "}
                <span className="text-accent">return</span>
                {" count"}
                {"\n"}
                <span className="text-fg-subtle">
                  {"  -- same-file theorems / certification checked at gate"}
                </span>
              </code>
            </pre>
          </div>

          <p className="mt-5 max-w-3xl text-[13px] leading-relaxed text-fg-subtle">
            {t.formalNote}
          </p>
        </div>
      </div>
    </section>
  );
}

export function PricingSection() {
  const { t } = useI18n();

  return (
    <section id="pricing" className="scroll-mt-20 border-y border-border bg-surface py-16 sm:py-20">
      <div className="mx-auto max-w-[1280px] px-5 sm:px-8">
        <div className="max-w-2xl">
          <h2 className="text-[1.65rem] font-semibold tracking-tight sm:text-[1.9rem]">
            {t.pricingTitle}
          </h2>
          <p className="mt-3 text-[15px] leading-relaxed text-fg-muted sm:text-[16px]">
            {t.pricingLead}
          </p>
        </div>

        <div className="mt-10 grid gap-4 lg:grid-cols-2">
          <article className="rounded-[var(--radius-2xl)] border border-border bg-bg p-6 sm:p-8">
            <div className="flex items-center gap-2 text-accent">
              <Terminal className="size-4" />
              <span className="text-[12px] font-semibold tracking-wide uppercase">
                Local
              </span>
            </div>
            <h3 className="mt-3 text-[1.25rem] font-semibold">{t.pricingLocalTitle}</h3>
            <p className="mt-1 font-display text-[2.25rem] italic text-fg">
              {t.pricingLocalPrice}
            </p>
            <p className="mt-3 text-[14px] leading-relaxed text-fg-muted">
              {t.pricingLocalBody}
            </p>
            <ul className="mt-6 space-y-2.5">
              {[t.pricingLocalF1, t.pricingLocalF2, t.pricingLocalF3].map((f) => (
                <li key={f} className="flex items-start gap-2 text-[14px] text-fg">
                  <Check className="mt-0.5 size-4 shrink-0 text-success" />
                  {f}
                </li>
              ))}
            </ul>
            <Button asChild className="mt-8 w-full sm:w-auto">
              <Link to="/login" search={{ redirect: "/sessions" }}>
                {t.ctaLogin}
                <ArrowRight className="size-4" />
              </Link>
            </Button>
          </article>

          <article className="rounded-[var(--radius-2xl)] border border-border-strong bg-sky-deep/40 p-6 sm:p-8">
            <div className="flex items-center gap-2 text-accent">
              <Cloud className="size-4" />
              <span className="text-[12px] font-semibold tracking-wide uppercase">
                Cloud
              </span>
            </div>
            <h3 className="mt-3 text-[1.25rem] font-semibold">{t.pricingCloudTitle}</h3>
            <p className="mt-1 font-display text-[2.25rem] italic text-fg">
              {t.pricingCloudPrice}
            </p>
            <p className="mt-3 text-[14px] leading-relaxed text-fg-muted">
              {t.pricingCloudBody}
            </p>
            <ul className="mt-6 space-y-2.5">
              {[t.pricingCloudF1, t.pricingCloudF2, t.pricingCloudF3].map((f) => (
                <li key={f} className="flex items-start gap-2 text-[14px] text-fg">
                  <Check className="mt-0.5 size-4 shrink-0 text-success" />
                  {f}
                </li>
              ))}
            </ul>
            <p className="mt-8 text-[13px] leading-relaxed text-fg-muted">{t.pricingNote}</p>
          </article>
        </div>
      </div>
    </section>
  );
}

export function HonestySection() {
  const { t } = useI18n();
  return (
    <section className="py-14 sm:py-16">
      <div className="mx-auto max-w-[1280px] px-5 sm:px-8">
        <div className="rounded-[var(--radius-2xl)] border border-border bg-surface px-6 py-8 sm:px-10 sm:py-10">
          <h2 className="text-[1.2rem] font-semibold tracking-tight sm:text-[1.35rem]">
            {t.honestyTitle}
          </h2>
          <p className="mt-3 max-w-3xl text-[14.5px] leading-relaxed text-fg-muted sm:text-[15px]">
            {t.honestyBody}
          </p>
        </div>
      </div>
    </section>
  );
}

export function FinalCta() {
  const { t } = useI18n();
  return (
    <section className="border-t border-border bg-sky-deep py-16 sm:py-20">
      <div className="mx-auto max-w-[1280px] px-5 text-center sm:px-8">
        <h2 className="font-display text-[2rem] italic tracking-tight text-white sm:text-[2.4rem]">
          {t.ctaTitle}
        </h2>
        <p className="mx-auto mt-3 max-w-lg text-[15px] text-white/75">{t.ctaSub}</p>
        <div className="mt-8 flex flex-col items-center justify-center gap-3 sm:flex-row">
          <Button asChild size="lg">
            <Link to="/login" search={{ redirect: "/sessions" }}>
              {t.ctaLogin}
              <ArrowRight className="size-4" />
            </Link>
          </Button>
          <Button asChild variant="secondary" size="lg">
            <a href={PROOFSHIP_REPO} target="_blank" rel="noreferrer">
              {t.ctaGithub}
            </a>
          </Button>
        </div>
      </div>
    </section>
  );
}

export function SiteFooter() {
  const { t } = useI18n();
  return (
    <footer className="border-t border-border bg-bg py-10">
      <div className="mx-auto flex max-w-[1280px] flex-col gap-6 px-5 sm:flex-row sm:items-center sm:justify-between sm:px-8">
        <div>
          <p className="font-display text-[1.35rem] italic text-fg">ProofShip</p>
          <p className="mt-1 text-[13px] text-fg-muted">{t.footerTag}</p>
        </div>
        <div className="flex flex-wrap items-center gap-x-5 gap-y-2 text-[13px] text-fg-muted">
          <a
            href={PROOFSHIP_REPO}
            target="_blank"
            rel="noreferrer"
            className="transition-colors hover:text-fg"
          >
            ProofShip GitHub
          </a>
          <a
            href={PROOF_FORGE_REPO}
            target="_blank"
            rel="noreferrer"
            className="transition-colors hover:text-fg"
          >
            ProofForge
          </a>
          <a
            href={PROOF_FORGE_SITE}
            target="_blank"
            rel="noreferrer"
            className="transition-colors hover:text-fg"
          >
            pf.grok.me
          </a>
          <a href="/#download" className="transition-colors hover:text-fg">
            {t.ctaDesktop}
          </a>
          <a href={PROOFSHIP_RELEASES} className="transition-colors hover:text-fg">
            Releases
          </a>
          <span>{t.footerMit}</span>
        </div>
      </div>
    </footer>
  );
}
