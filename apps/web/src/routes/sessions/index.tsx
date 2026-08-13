import { createFileRoute } from "@tanstack/react-router";
import { useCurrentUserState } from "@/lib/auth/use-current-user";
import { RedirectToSignIn } from "@/lib/auth/gates";
import { Workspace } from "@/components/sessions/workspace";
import { sessionSearchSchema } from "@/lib/relay";

export const Route = createFileRoute("/sessions/")({
  validateSearch: sessionSearchSchema,
  component: SessionsIndex,
});

function SessionsIndex() {
  const { user, isPending } = useCurrentUserState();
  if (isPending) {
    return <div className="grid min-h-dvh place-items-center bg-bg text-fg-muted">…</div>;
  }
  if (!user) return <RedirectToSignIn />;
  return <Workspace />;
}
