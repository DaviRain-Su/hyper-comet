import { Link } from "@tanstack/react-router";
import { EdgeDither, HandAscii, HeroAscii } from "@/components/brand/ascii-art";
import { GitHubIcon } from "@/components/brand/logo";
import { SiteNav } from "@/components/nav/site-nav";
import { pick, useLocale } from "@/lib/i18n";

const AGENTS = [
  { name: "Claude Code", src: "/assets/icons/claude-mark.svg" },
  { name: "Codex", src: "/assets/icons/openai-mark.svg" },
  { name: "Cursor", src: "/assets/icons/cursor-mark.svg" },
  { name: "Grok", src: "/assets/icons/grok-mark.svg" },
  { name: "Hermes", src: "/assets/icons/hermes-mark.svg" },
  { name: "Pi", src: "/assets/icons/pi-mark.svg" },
];

export function LandingPage() {
  const { locale } = useLocale();
  const t = (en: string, zh: string) => pick(locale, en, zh);

  return (
    <div className="min-h-dvh bg-bg text-ink">
      <SiteNav />

      <header className="relative overflow-hidden">
        <div className="mx-auto grid max-w-[1140px] items-center gap-10 px-5 pb-10 pt-12 sm:px-8 sm:pt-[72px] lg:grid-cols-[1.05fr_0.95fr] lg:gap-10">
          <div>
            <h1 className="max-w-[18ch] text-balance text-[clamp(2rem,4.8vw,3.625rem)] font-semibold leading-[1.05] tracking-[-0.035em]">
              {t("AI drafts the contract.", "AI 起草合约。")}
              <br />
              {t("The gate decides if it ships.", "门禁决定它能不能上链。")}
            </h1>
            <p className="mt-[22px] max-w-[460px] text-pretty text-base leading-[1.65] text-dim">
              {t(
                "ProofShip combines local Sessions with 7 ACP lanes and a machine gate. Your agent drafts ProgramV1 contracts; ProofForge checks, builds, and inspects before deployment to X Layer.",
                "本地 Sessions + 7 条 ACP + 机器门禁。你的 Agent 起草 ProgramV1；ProofForge 做 check → build → inspect。不过门禁，没有制品，没有部署。首发 X Layer。",
              )}
            </p>
            <div className="mt-9 flex flex-wrap items-center gap-3.5">
              <Link
                to="/login"
                search={{ redirect: "/sessions" }}
                className="inline-flex h-10 items-center rounded-lg bg-purple px-[18px] text-[13.5px] font-medium text-white hover:bg-purple-hi"
              >
                {t("Open web Sessions", "打开 web Sessions")}
              </Link>
              <span className="font-mono text-[12.5px] text-faint">v0.1.48</span>
            </div>
            <div className="mt-10 flex flex-wrap items-center gap-[26px]" aria-label="Supported agents">
              {AGENTS.map((a) => (
                <span
                  key={a.name}
                  title={a.name}
                  className="size-[22px] bg-faint transition-colors hover:bg-purple-hi [mask:var(--icon)_center/contain_no-repeat]"
                  style={{ ["--icon" as string]: `url('${a.src}')` }}
                />
              ))}
            </div>
          </div>
          <HeroAscii />
        </div>
      </header>

      <section className="relative pt-12">
        <div className="pointer-events-none absolute inset-x-0 top-[30%] h-[70%] bg-[radial-gradient(55%_55%_at_50%_60%,rgba(91,52,184,0.18),transparent_72%)]" />
        <EdgeDither side="left" />
        <EdgeDither side="right" />
        <div className="relative z-[1] mx-auto max-w-[1240px] px-5 sm:px-8">
          <img
            src="/assets/app-screenshot.jpg"
            alt={t(
              "ProofShip Sessions driving an ACP contract drafting run with ProofForge gate checks",
              "ProofShip Sessions：ACP 起草合约，ProofForge 门禁核验",
            )}
            className="block w-full rounded-md border border-line shadow-[0_30px_100px_-30px_rgba(0,0,0,0.8),0_0_90px_-30px_rgba(139,92,246,0.35)]"
          />
        </div>
      </section>

      <section className="mx-auto grid max-w-[1140px] gap-9 px-5 pt-16 sm:grid-cols-2 sm:px-8 sm:pt-24 lg:grid-cols-4 lg:gap-10">
        {[
          {
            idx: "Sessions & ACP",
            h: t("Sessions + 7 ACP lanes", "Sessions + 7 条 ACP"),
            p: t(
              "Draft ProgramV1 with Claude Code, Codex, Grok, Hermes, Pi, Cursor, or OpenCode over one ACP layer.",
              "用 Claude Code、Codex、Grok、Hermes、Pi、Cursor 或 OpenCode 起草 ProgramV1。一条 ACP，不绑平台模型。",
            ),
          },
          {
            idx: "ProofForge Gate",
            h: t("Check, build & inspect", "check → build → inspect"),
            p: t(
              "Engineering-grade machine gate. Fail-closed: no artifacts if check, build, or inspect fails.",
              "工程级机器门禁。失败即关：check / build / inspect 任一失败，零制品。",
            ),
          },
          {
            idx: "X Layer First",
            h: t("Multi-EVM deployment", "多 EVM 部署"),
            p: t(
              "X Layer testnet first, then multi-EVM. Sealed artifacts and ABIs stay with you.",
              "首发 X Layer 测试网，再扩多 EVM。密封制品与 ABI 留在你这边。",
            ),
          },
          {
            idx: "Local First",
            h: t("Keys stay user-side", "密钥留在用户侧"),
            p: t(
              "Deploy keys stay in your environment or wallet — never in the app, repo, or relay.",
              "部署密钥只在你的环境或钱包里——不进应用、不进仓库、不进中继。",
            ),
          },
        ].map((c) => (
          <div key={c.idx}>
            <div className="mb-4 text-[11px] font-semibold tracking-[0.14em] text-purple-hi">{c.idx}</div>
            <h3 className="mb-2.5 text-base font-semibold tracking-[-0.015em]">{c.h}</h3>
            <p className="text-[13.5px] leading-[1.65] text-dim">{c.p}</p>
          </div>
        ))}
      </section>

      <section className="relative mx-auto max-w-[1140px] px-5 py-[100px] sm:px-8 sm:py-[140px]">
        <HandAscii variant="down" />
        <div className="relative z-[1]">
          <h2 className="text-balance text-[clamp(1.75rem,3.8vw,2.5rem)] font-semibold tracking-[-0.03em]">
            {t("Draft safely. Ship with proof.", "安全起草。带证明上链。")}
          </h2>
          <p className="mt-4 max-w-[440px] text-[15px] leading-[1.65] text-dim">
            {t(
              "Local-first agent session workflows, ProofForge machine gating, and X Layer smart contract deployment.",
              "本地优先的 Sessions、ProofForge 机器门禁、X Layer 智能合约部署。",
            )}
          </p>
          <div className="mt-[34px] flex flex-wrap gap-3.5">
            <Link
              to="/login"
              search={{ redirect: "/sessions" }}
              className="inline-flex h-10 items-center rounded-lg bg-purple px-[18px] text-[13.5px] font-medium text-white hover:bg-purple-hi"
            >
              {t("Open web Sessions", "打开 web Sessions")}
            </Link>
            <a
              href="https://github.com/DaviRain-Su/proofship"
              className="inline-flex h-10 items-center rounded-lg border border-line px-[18px] text-[13.5px] font-medium text-dim hover:border-faint hover:text-ink"
            >
              {t("View on GitHub", "查看 GitHub")} ↗
            </a>
          </div>
        </div>
      </section>

      <footer className="relative px-5 pb-10 pt-7 sm:px-8">
        <HandAscii variant="up" />
        <div className="relative z-[1] mx-auto flex max-w-[1140px] flex-wrap items-center gap-5 text-[12.5px] text-faint">
          <span>ProofShip</span>
          <span className="hidden sm:inline">
            {t("Web companion · keys never on this page", "Web 伴侣 · 密钥从不在此页")}
          </span>
          <div className="ml-auto flex items-center gap-5">
            <a href="https://github.com/DaviRain-Su/proofship" aria-label="GitHub" className="hover:text-dim">
              <GitHubIcon />
            </a>
            <span>ProofShip © 2026</span>
          </div>
        </div>
      </footer>
    </div>
  );
}
