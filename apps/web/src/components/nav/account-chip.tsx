import { Link } from "@tanstack/react-router";
import { useCurrentUserState } from "@/lib/auth/use-current-user";
import { authEnabled, signOut } from "@/lib/auth/client";
import { pick, useLocale } from "@/lib/i18n";
import { cn } from "@/lib/cn";

export function AccountChip({ className }: { className?: string }) {
  const { user, isPending } = useCurrentUserState();
  const { locale } = useLocale();

  if (isPending) {
    return <div className={cn("h-8 w-20 animate-pulse rounded-lg bg-raise", className)} />;
  }
  if (!user) {
    return (
      <Link
        to="/login"
        search={{ redirect: "/sessions" }}
        className={cn(
          "inline-flex h-8 items-center rounded-lg bg-purple px-3.5 text-[11.5px] font-medium text-white hover:bg-purple-hi",
          className,
        )}
      >
        {pick(locale, "Sign in", "登录")}
      </Link>
    );
  }

  const label = user.displayName ?? user.primaryEmail ?? "Account";
  return (
    <div className={cn("flex items-center gap-2", className)}>
      {user.profileImageUrl ? (
        <img src={user.profileImageUrl} alt="" className="size-7 rounded-full object-cover" />
      ) : (
        <span className="grid size-7 place-items-center rounded-full bg-raise text-[11px] font-medium text-ink">
          {label.charAt(0).toUpperCase()}
        </span>
      )}
      <span className="hidden max-w-28 truncate text-[13px] text-dim sm:inline">{label}</span>
      {authEnabled && (
        <button
          type="button"
          onClick={() => void signOut()}
          className="text-[12px] text-faint hover:text-ink"
        >
          {pick(locale, "Sign out", "退出")}
        </button>
      )}
    </div>
  );
}
