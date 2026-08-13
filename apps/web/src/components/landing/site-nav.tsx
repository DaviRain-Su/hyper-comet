import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import { Menu, X } from "lucide-react";
import { useI18n } from "@/lib/i18n";
import { useCurrentUserState } from "@/lib/auth/use-current-user";
import { UserButton } from "@/lib/auth/gates";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const PROOFSHIP_REPO = "https://github.com/DaviRain-Su/proofship";

export function SiteNav() {
  const { t, locale, setLocale } = useI18n();
  const { user, isPending } = useCurrentUserState();
  const [scrolled, setScrolled] = useState(false);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 12);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  const links = [
    { href: "/#showcase", label: t.navShowcase },
    { href: "/#product", label: t.navProduct },
    { href: "/#difference", label: t.navDiff },
    { href: "/#workflow", label: t.navHow },
    { href: "/#download", label: t.navDownload },
    { href: "/#proofforge", label: t.navForge },
    { href: "/#pricing", label: t.navPricing },
    { href: "/sessions", label: t.navStudio },
  ];

  return (
    <header
      className={cn(
        "fixed inset-x-0 top-0 z-50 transition-colors duration-300",
        scrolled || open
          ? "border-b border-border bg-[rgba(5,10,22,0.92)] backdrop-blur-md"
          : "bg-transparent",
      )}
    >
      <div className="mx-auto flex h-14 max-w-[1280px] items-center justify-between gap-4 px-5 sm:h-16 sm:px-8">
        <Link
          to="/"
          search={{}}
          className="font-display text-[1.55rem] italic leading-none tracking-tight text-fg sm:text-[1.65rem]"
        >
          ProofShip
        </Link>

        <nav className="hidden items-center gap-5 xl:flex">
          {links.map((l) =>
            l.href.startsWith("/") && !l.href.includes("#") ? (
              <Link
                key={l.href}
                to={l.href}
                search={{}}
                className="text-[13.5px] font-medium text-white/80 transition-colors hover:text-white"
              >
                {l.label}
              </Link>
            ) : (
              <a
                key={l.href}
                href={l.href}
                className="text-[13.5px] font-medium text-white/80 transition-colors hover:text-white"
              >
                {l.label}
              </a>
            ),
          )}
          <LangToggle locale={locale} setLocale={setLocale} t={t} />
        </nav>

        <div className="hidden items-center gap-2 md:flex">
          <a
            href={PROOFSHIP_REPO}
            target="_blank"
            rel="noreferrer"
            className="hidden text-[13px] font-medium text-white/70 transition-colors hover:text-white 2xl:inline"
          >
            {t.ctaGithub}
          </a>
          <AuthSlot isPending={isPending} user={user} t={t} />
        </div>

        <div className="flex items-center gap-2 xl:hidden">
          <LangToggle locale={locale} setLocale={setLocale} t={t} />
          <button
            type="button"
            className="inline-flex size-10 items-center justify-center rounded-[var(--radius-md)] text-fg"
            aria-label={open ? t.closeMenu : t.openMenu}
            aria-expanded={open}
            onClick={() => setOpen((v) => !v)}
          >
            {open ? <X className="size-5" /> : <Menu className="size-5" />}
          </button>
        </div>
      </div>

      <div
        className={cn(
          "overflow-hidden border-border bg-[rgba(5,10,22,0.96)] backdrop-blur-md transition-[max-height,opacity] duration-300 xl:hidden",
          open ? "max-h-[75vh] border-t opacity-100" : "max-h-0 border-t-0 opacity-0",
        )}
      >
        <nav className="flex flex-col gap-1 px-5 py-4">
          {links.map((l) =>
            l.href.startsWith("/") && !l.href.includes("#") ? (
              <Link
                key={l.href}
                to={l.href}
                search={{}}
                onClick={() => setOpen(false)}
                className="rounded-[var(--radius-md)] px-3 py-3 text-[15px] font-medium text-white/90 hover:bg-white/5"
              >
                {l.label}
              </Link>
            ) : (
              <a
                key={l.href}
                href={l.href}
                onClick={() => setOpen(false)}
                className="rounded-[var(--radius-md)] px-3 py-3 text-[15px] font-medium text-white/90 hover:bg-white/5"
              >
                {l.label}
              </a>
            ),
          )}
          <div className="mt-2 flex flex-col gap-2 border-t border-border pt-3">
            <AuthSlot isPending={isPending} user={user} t={t} full />
            <a
              href={PROOFSHIP_REPO}
              target="_blank"
              rel="noreferrer"
              className="rounded-[var(--radius-md)] px-3 py-3 text-[15px] font-medium text-white/80 hover:bg-white/5"
            >
              {t.ctaGithub}
            </a>
          </div>
        </nav>
      </div>
    </header>
  );
}

function LangToggle({
  locale,
  setLocale,
  t,
}: {
  locale: "zh" | "en";
  setLocale: (l: "zh" | "en") => void;
  t: { langEn: string; langZh: string };
}) {
  return (
    <div
      className="inline-flex items-center gap-0.5 rounded-[var(--radius-md)] border border-white/15 bg-white/5 p-0.5"
      role="group"
      aria-label="Language"
    >
      <button
        type="button"
        aria-pressed={locale === "en"}
        onClick={() => setLocale("en")}
        className={cn(
          "rounded-md px-2 py-1 text-[12px] font-semibold tracking-wide transition-colors",
          locale === "en" ? "bg-white/15 text-white" : "text-white/60 hover:text-white",
        )}
      >
        {t.langEn}
      </button>
      <button
        type="button"
        aria-pressed={locale === "zh"}
        onClick={() => setLocale("zh")}
        className={cn(
          "rounded-md px-2 py-1 text-[12px] font-semibold tracking-wide transition-colors",
          locale === "zh" ? "bg-white/15 text-white" : "text-white/60 hover:text-white",
        )}
      >
        {t.langZh}
      </button>
    </div>
  );
}

function AuthSlot({
  isPending,
  user,
  t,
  full,
}: {
  isPending: boolean;
  user: { displayName?: string | null } | null;
  t: { login: string; openStudio: string };
  full?: boolean;
}) {
  if (isPending) {
    return (
      <div
        className={cn(
          "animate-pulse rounded-[var(--radius-md)] bg-white/10",
          full ? "h-11 w-full" : "h-9 w-24",
        )}
      />
    );
  }
  if (user) {
    return (
      <div className={cn("flex items-center gap-2", full && "flex-col items-stretch")}>
        <Button asChild size={full ? "default" : "sm"} className={full ? "w-full" : undefined}>
          <Link to="/sessions" search={{}}>
            {t.openStudio}
          </Link>
        </Button>
        <div className={cn("text-fg", full && "px-1 py-2")}>
          <UserButton />
        </div>
      </div>
    );
  }
  return (
    <Button asChild size={full ? "default" : "sm"} className={full ? "w-full" : undefined}>
      <Link to="/login" search={{ redirect: "/sessions" }}>
        {t.login}
      </Link>
    </Button>
  );
}
