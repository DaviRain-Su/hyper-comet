import { useState, type FormEvent } from "react";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";
import { GROK_PROVIDERS, authClient, authEnabled, signIn } from "@/lib/auth/client";
import { useCurrentUserState } from "@/lib/auth/use-current-user";
import { useI18n } from "@/lib/i18n";
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
      void navigate({ to: "/sessions/$sessionId", params: { sessionId }, search: {} });
      return;
    }
  }
  void navigate({ to: "/sessions", search: {} });
}

function Login() {
  const { t } = useI18n();
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
    <div className="relative min-h-dvh overflow-hidden bg-bg text-fg">
      <div className="absolute inset-0" aria-hidden="true">
        <div className="absolute inset-0 bg-sky-deep" />
        <img src="/heroes/dusk-ridges.jpg" alt="" className="h-full w-full object-cover opacity-40" />
        <div
          className="absolute inset-0"
          style={{
            background: "linear-gradient(180deg, rgba(5,8,15,0.55) 0%, rgba(5,8,15,0.85) 100%)",
          }}
        />
      </div>
      <div className="film-grain opacity-20" aria-hidden="true" />

      <div className="relative z-10 mx-auto flex min-h-dvh max-w-md flex-col justify-center px-5 py-16">
        <Link
          to="/"
          search={{}}
          className="mb-8 inline-flex w-fit items-center gap-2 text-[13.5px] font-medium text-white/70 transition-colors hover:text-white"
        >
          <ArrowLeft className="size-4" />
          {t.loginBack}
        </Link>

        <div className="rounded-[var(--radius-2xl)] border border-border bg-surface/90 p-7 shadow-[var(--shadow-soft)] backdrop-blur-md sm:p-8">
          <p className="font-display text-[1.75rem] italic leading-none text-fg">ProofShip</p>
          <h1 className="mt-4 text-[1.35rem] font-semibold tracking-tight">{t.loginTitle}</h1>
          <p className="mt-2 text-[14px] leading-relaxed text-fg-muted">{t.loginSub}</p>

          <div className="mt-7 space-y-3">
            {isPending ? (
              <div className="h-11 animate-pulse rounded-[var(--radius-md)] bg-white/10" />
            ) : authEnabled ? (
              GROK_PROVIDERS.map((p) => (
                <Button
                  key={p.providerId}
                  type="button"
                  variant="secondary"
                  className="w-full"
                  onClick={() => void signIn(p.providerId, { callbackURL: next })}
                >
                  {t.loginContinue} {p.label}
                </Button>
              ))
            ) : (
              <p className="text-sm text-fg-muted">{t.loginDisabled}</p>
            )}
          </div>

          <div className="my-6 flex items-center gap-3 text-[12px] text-fg-subtle">
            <span className="h-px flex-1 bg-border" />
            {t.loginEmail}
            <span className="h-px flex-1 bg-border" />
          </div>

          <form className="space-y-3" onSubmit={(e) => void onEmail(e)}>
            {mode === "up" && (
              <Input
                placeholder={t.loginName}
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
              placeholder={t.loginPassword}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete={mode === "up" ? "new-password" : "current-password"}
            />
            {error && <p className="text-[13px] text-red-300">{error}</p>}
            <Button type="submit" className="h-11 w-full" disabled={busy}>
              {mode === "up" ? t.loginCreate : t.loginSignIn}
            </Button>
          </form>

          <button
            type="button"
            className="mt-4 text-[13px] text-fg-muted hover:text-fg"
            onClick={() => setMode(mode === "up" ? "in" : "up")}
          >
            {mode === "up" ? t.loginHaveAccount : t.loginNewHere}
          </button>
        </div>
      </div>
    </div>
  );
}
