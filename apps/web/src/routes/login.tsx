import { useState, type FormEvent } from "react";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { GROK_PROVIDERS, authClient, authEnabled, signIn } from "@/lib/auth/client";
import { useCurrentUserState } from "@/lib/auth/use-current-user";
import { pick, useLocale } from "@/lib/i18n";
import { Wordmark } from "@/components/brand/logo";
import { LocaleToggle } from "@/components/brand/locale-toggle";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

type LoginSearch = { redirect?: string };

export const Route = createFileRoute("/login")({
  validateSearch: (s: Record<string, unknown>): LoginSearch => ({
    redirect: typeof s.redirect === "string" ? s.redirect : "/sessions",
  }),
  component: Login,
});

function goNext(navigate: ReturnType<typeof useNavigate>, next: string) {
  if (next.startsWith("/sessions/") && next !== "/sessions/") {
    const sessionId = next.slice("/sessions/".length).split("?")[0] ?? "";
    if (sessionId) {
      void navigate({ to: "/sessions/$sessionId", params: { sessionId } });
      return;
    }
  }
  void navigate({ to: "/sessions" });
}

function Login() {
  const { locale } = useLocale();
  const { redirect } = Route.useSearch();
  const navigate = useNavigate();
  const { user, isPending } = useCurrentUserState();
  const [mode, setMode] = useState<"in" | "up">("in");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const next = redirect && redirect.startsWith("/") ? redirect : "/sessions";

  if (!isPending && user) {
    goNext(navigate, next);
  }

  const onEmail = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      if (mode === "up") {
        const res = await authClient.signUp.email({
          email,
          password,
          name: name || email.split("@")[0]!,
        });
        if (res.error) throw new Error(res.error.message || "Sign up failed");
      } else {
        const res = await authClient.signIn.email({ email, password });
        if (res.error) throw new Error(res.error.message || "Sign in failed");
      }
      goNext(navigate, next);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <main className="relative grid min-h-dvh place-items-center px-5 py-16">
      <div className="absolute inset-x-0 top-0 flex h-16 items-center justify-between px-5 sm:px-8">
        <Wordmark />
        <LocaleToggle />
      </div>
      <div className="w-full max-w-[400px]">
        <p className="text-[11px] font-semibold tracking-[0.14em] text-purple-hi">ProofShip</p>
        <h1 className="mt-3 text-[28px] font-semibold tracking-[-0.03em]">
          {pick(locale, "Sign in to Sessions", "登录进入 Sessions")}
        </h1>
        <p className="mt-2 text-[14px] leading-relaxed text-dim">
          {pick(
            locale,
            "This identifies you. It never sends a deploy key.",
            "只用来识别你。不会交出部署密钥。",
          )}
        </p>

        {authEnabled ? (
          <div className="mt-8 space-y-2.5">
            {GROK_PROVIDERS.map((p) => (
              <button
                key={p.providerId}
                type="button"
                onClick={() => signIn(p.providerId, { callbackURL: next })}
                className="flex h-11 w-full items-center justify-center rounded-lg border border-line bg-raise text-[14px] font-medium text-ink hover:border-faint"
              >
                {pick(locale, `Continue with ${p.label}`, `使用 ${p.label} 继续`)}
              </button>
            ))}
          </div>
        ) : (
          <p className="mt-8 text-sm text-dim">{pick(locale, "Sign-in is disabled.", "登录已关闭。")}</p>
        )}

        <div className="my-7 flex items-center gap-3 text-[12px] text-faint">
          <span className="h-px flex-1 bg-line" />
          {pick(locale, "or email", "或使用邮箱")}
          <span className="h-px flex-1 bg-line" />
        </div>

        <form className="space-y-3" onSubmit={(e) => void onEmail(e)}>
          {mode === "up" && (
            <Input
              placeholder={pick(locale, "Name", "名字")}
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoComplete="name"
            />
          )}
          <Input
            type="email"
            required
            placeholder="you@example.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            autoComplete="email"
          />
          <Input
            type="password"
            required
            minLength={8}
            placeholder={pick(locale, "Password", "密码")}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete={mode === "up" ? "new-password" : "current-password"}
          />
          {error && <p className="text-[13px] text-red-300">{error}</p>}
          <Button type="submit" className="h-11 w-full" disabled={busy}>
            {mode === "up"
              ? pick(locale, "Create account", "创建账户")
              : pick(locale, "Sign in with email", "邮箱登录")}
          </Button>
        </form>

        <button
          type="button"
          className="mt-4 text-[13px] text-dim hover:text-ink"
          onClick={() => setMode(mode === "up" ? "in" : "up")}
        >
          {mode === "up"
            ? pick(locale, "Already have an account? Sign in", "已有账户？去登录")
            : pick(locale, "New here? Create an account", "新用户？创建账户")}
        </button>

        <p className="mt-10 text-[12px] text-faint">
          <Link to="/" className="hover:text-dim">
            ← {pick(locale, "Back to ProofShip", "返回 ProofShip")}
          </Link>
        </p>
      </div>
    </main>
  );
}
