import { Link } from "@tanstack/react-router";
import { GitHubIcon, Wordmark } from "@/components/brand/logo";
import { LocaleToggle } from "@/components/brand/locale-toggle";
import { AccountChip } from "@/components/nav/account-chip";
import { pick, useLocale } from "@/lib/i18n";
import { useCurrentUserState } from "@/lib/auth/use-current-user";

export function SiteNav() {
  const { locale } = useLocale();
  const { user, isPending } = useCurrentUserState();

  return (
    <nav className="relative z-50">
      <div className="mx-auto flex h-16 max-w-[1140px] items-center gap-6 px-5 sm:px-8">
        <Wordmark />
        <div className="ml-auto flex items-center gap-3 sm:gap-4">
          <LocaleToggle />
          <a
            href="https://github.com/DaviRain-Su/proofship"
            aria-label="GitHub"
            className="text-dim transition-colors hover:text-ink"
          >
            <GitHubIcon />
          </a>
          {!isPending && user ? (
            <Link
              to="/sessions"
              className="inline-flex h-8 items-center rounded-lg bg-purple px-3.5 text-[11.5px] font-medium text-white hover:bg-purple-hi"
            >
              {pick(locale, "Open Sessions", "进入 Sessions")}
            </Link>
          ) : (
            <Link
              to="/login"
              search={{ redirect: "/sessions" }}
              className="inline-flex h-8 items-center rounded-lg bg-purple px-3.5 text-[11.5px] font-medium text-white hover:bg-purple-hi"
            >
              {pick(locale, "Open Sessions", "进入 Sessions")}
            </Link>
          )}
          <AccountChip className="hidden md:flex" />
        </div>
      </div>
    </nav>
  );
}
