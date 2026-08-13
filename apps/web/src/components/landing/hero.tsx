import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import { ArrowRight, Download } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { PROOFSHIP_RELEASES } from "@/lib/links";

const HEROES = [
  "/heroes/alpine-peaks.jpg",
  "/heroes/sunset-path.jpg",
  "/heroes/green-hills.jpg",
  "/heroes/dusk-ridges.jpg",
  "/heroes/red-mountains.jpg",
];

export function Hero() {
  const { t } = useI18n();
  const [index, setIndex] = useState(0);

  useEffect(() => {
    const id = window.setInterval(() => {
      setIndex((i) => (i + 1) % HEROES.length);
    }, 6500);
    return () => window.clearInterval(id);
  }, []);

  return (
    <section className="relative min-h-dvh w-full overflow-hidden">
      <div className="absolute inset-0 overflow-hidden" aria-hidden="true">
        <div className="absolute inset-0 bg-sky-deep" />
        {HEROES.map((src, i) => (
          <div
            key={src}
            className="absolute inset-0 transition-opacity duration-[1400ms] ease-out"
            style={{ opacity: i === index ? 1 : 0 }}
          >
            <img
              src={src}
              alt=""
              className={`hero-photo h-full w-full object-cover object-center ${i === index ? "hero-kenburns" : ""}`}
              draggable={false}
              loading={i === 0 ? "eager" : "lazy"}
              fetchPriority={i === 0 ? "high" : "low"}
            />
          </div>
        ))}
        <div
          className="absolute inset-0"
          style={{
            background:
              "linear-gradient(180deg, rgba(4,18,48,0.22) 0%, rgba(4,18,48,0.08) 38%, rgba(4,12,28,0.58) 100%), linear-gradient(90deg, rgba(4,18,48,0.52) 0%, rgba(4,18,48,0.18) 48%, transparent 78%)",
          }}
        />
        <div className="absolute bottom-6 left-1/2 z-30 flex -translate-x-1/2 gap-2">
          {HEROES.map((_, i) => (
            <button
              key={i}
              type="button"
              aria-label={`Show landscape ${i + 1}`}
              onClick={() => setIndex(i)}
              className="h-1.5 rounded-full transition-all duration-300"
              style={{
                width: i === index ? 22 : 8,
                background: i === index ? "rgba(255,255,255,0.95)" : "rgba(255,255,255,0.35)",
              }}
            />
          ))}
        </div>
      </div>

      <div className="film-grain" aria-hidden="true" />

      <div className="relative z-30 mx-auto flex min-h-dvh max-w-[1280px] flex-col justify-center px-5 pb-20 pt-28 sm:px-8 sm:pb-24 sm:pt-32">
        <div className="grid items-center gap-10 lg:grid-cols-[minmax(0,1fr)_minmax(0,540px)]">
          <div className="max-w-[40rem]">
            <p className="rise-in mb-4 inline-flex flex-wrap items-center gap-2 rounded-full border border-white/15 bg-white/5 px-3 py-1 text-[12px] font-medium tracking-wide text-white/85 backdrop-blur-sm">
              <span className="size-1.5 rounded-full bg-accent" />
              {t.heroBadge}
            </p>
            <h1 className="rise-in rise-in-delay-1 text-[1.85rem] font-medium leading-[1.16] tracking-[-0.02em] text-white sm:text-[2.35rem] md:text-[2.65rem] lg:text-[2.85rem]">
              {t.heroTitle}
            </h1>
            <p className="rise-in rise-in-delay-2 mt-5 max-w-xl text-[15px] leading-relaxed text-white/80 sm:text-[16px]">
              {t.heroSub}
            </p>
            <div className="rise-in rise-in-delay-3 mt-8 flex flex-col gap-3 sm:mt-10 sm:flex-row sm:items-center">
              <Button asChild size="lg">
                <Link to="/login" search={{ redirect: "/sessions" }}>
                  {t.ctaLogin}
                  <ArrowRight className="size-4" strokeWidth={2.25} />
                </Link>
              </Button>
              <Button asChild variant="secondary" size="lg">
                <a href={PROOFSHIP_RELEASES}>
                  <Download className="size-4" />
                  {t.ctaDesktop}
                </a>
              </Button>
            </div>

            <dl className="rise-in rise-in-delay-4 mt-12 grid max-w-xl grid-cols-1 gap-3 sm:grid-cols-3 sm:gap-4">
              {[t.statsAgents, t.statsGate, t.statsChain].map((label) => (
                <div
                  key={label}
                  className="rounded-[var(--radius-lg)] border border-white/15 bg-white/5 px-3 py-3 backdrop-blur-sm"
                >
                  <dt className="text-[13px] font-medium text-white/90">{label}</dt>
                </div>
              ))}
            </dl>
          </div>

          <div className="rise-in rise-in-delay-3 hidden lg:block">
            <a href="#showcase" className="group block">
              <div className="overflow-hidden rounded-[var(--radius-2xl)] border border-white/15 bg-black/30 shadow-[var(--shadow-soft)] backdrop-blur-sm">
                <div className="flex h-9 items-center gap-2 border-b border-white/10 px-3">
                  <span className="size-2 rounded-full bg-white/20" />
                  <span className="size-2 rounded-full bg-white/20" />
                  <span className="size-2 rounded-full bg-white/20" />
                  <span className="ml-2 font-mono text-[10px] tracking-wide text-white/55">
                    Desktop · ProofShip
                  </span>
                </div>
                <img
                  src="/assets/app-screenshot.jpg"
                  alt={t.showcaseShotAlt}
                  className="aspect-[16/10] w-full object-cover object-top outline outline-1 -outline-offset-1 outline-white/10 transition duration-300 group-hover:opacity-95"
                />
              </div>
              <p className="mt-2 text-center text-[12px] text-white/60">{t.showcaseShotHint}</p>
            </a>
          </div>
        </div>
      </div>
    </section>
  );
}
