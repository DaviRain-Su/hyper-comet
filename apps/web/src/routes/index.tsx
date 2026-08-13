import { createFileRoute, Navigate } from "@tanstack/react-router";
import { LandingPage } from "@/components/landing/landing-page";
import { sessionSearchSchema } from "@/lib/relay";

export const Route = createFileRoute("/")({
  validateSearch: sessionSearchSchema,
  component: Home,
});

function Home() {
  const { relay, session } = Route.useSearch();
  if (relay || session) {
    return <Navigate to="/sessions" search={{ relay, session }} />;
  }
  return <LandingPage />;
}
