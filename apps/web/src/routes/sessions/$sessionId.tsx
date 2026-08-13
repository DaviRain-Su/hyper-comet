import { createFileRoute } from "@tanstack/react-router";
import { useCurrentUserState } from "@/lib/auth/use-current-user";
import { RedirectToSignIn } from "@/lib/auth/gates";
import { Workspace } from "@/components/sessions/workspace";

export const Route = createFileRoute("/sessions/$sessionId")({
  component: SessionPage,
});

function SessionPage() {
  const { sessionId } = Route.useParams();
  const { user, isPending } = useCurrentUserState();
  if (isPending) {
    return <div className="grid min-h-dvh place-items-center bg-bg text-dim">…</div>;
  }
  if (!user) return <RedirectToSignIn />;
  return <Workspace sessionId={sessionId} />;
}
