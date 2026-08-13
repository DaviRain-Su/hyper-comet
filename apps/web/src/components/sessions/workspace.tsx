import { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { toast } from "sonner";
import { SessionSidebar } from "@/components/sessions/session-sidebar";
import { Transcript } from "@/components/sessions/transcript";
import { Composer } from "@/components/sessions/composer";
import { GateRail } from "@/components/sessions/gate-rail";
import { EmptySession } from "@/components/sessions/empty-session";
import { SessionHeader } from "@/components/sessions/session-header";
import { DesktopLinkBar } from "@/components/sessions/desktop-link";
import { pick, useLocale } from "@/lib/i18n";
import { useDesktopLink, type PromptMode } from "@/lib/use-desktop-link";
import {
  applyTemplate,
  createSession,
  deleteSession,
  getSession,
  listSessions,
  renameSession,
  runGateAgain,
  sendPrompt,
  type MessageRow,
  type SessionRow,
} from "@/lib/sessions";

export function Workspace({ sessionId }: { sessionId?: string }) {
  const { locale } = useLocale();
  const navigate = useNavigate();
  const search = useSearch({ strict: false }) as { relay?: string; session?: string };
  const link = useDesktopLink(undefined, {
    relay: search.relay,
    session: search.session,
  });
  const [sessions, setSessions] = useState<SessionRow[]>([]);
  const [session, setSession] = useState<SessionRow | null>(null);
  const [messages, setMessages] = useState<MessageRow[]>([]);
  const [busy, setBusy] = useState(false);
  const [creating, setCreating] = useState(false);
  const [navOpen, setNavOpen] = useState(false);
  const [railOpen, setRailOpen] = useState(false);

  const refreshList = useCallback(async () => {
    try {
      const rows = await listSessions();
      setSessions(rows);
    } catch {
      /* signed out handled by route */
    }
  }, []);

  const loadOne = useCallback(async (id: string) => {
    const bundle = await getSession({ data: { id } });
    if (!bundle) {
      setSession(null);
      setMessages([]);
      return;
    }
    setSession(bundle.session);
    setMessages(bundle.messages);
  }, []);

  useEffect(() => {
    void refreshList();
  }, [refreshList]);

  useEffect(() => {
    if (!sessionId) {
      setSession(null);
      setMessages([]);
      return;
    }
    void loadOne(sessionId);
  }, [sessionId, loadOne]);

  const merged = useMemo(() => {
    const seen = new Set(messages.map((m) => `${m.role}:${m.kind}:${m.content}`));
    const extra = link.messages.filter((m) => !seen.has(`${m.role}:${m.kind}:${m.content}`));
    return extra.length ? [...messages, ...extra] : messages;
  }, [messages, link.messages]);

  const handleNew = async () => {
    setCreating(true);
    try {
      const created = await createSession({ data: { title: "New session" } });
      await refreshList();
      setNavOpen(false);
      await navigate({
        to: "/sessions/$sessionId",
        params: { sessionId: created.id },
        search: { relay: search.relay, session: search.session },
      });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed");
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteSession({ data: { id } });
      await refreshList();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed");
    }
  };

  const handleRename = async (title: string) => {
    if (!sessionId) return;
    try {
      const bundle = await renameSession({ data: { id: sessionId, title } });
      if (bundle) {
        setSession(bundle.session);
        setMessages(bundle.messages);
      }
      await refreshList();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed");
    }
  };

  const handleSend = async (text: string, mode: PromptMode = "prompt") => {
    if (!link.desktopOnline && mode !== "comment") {
      toast.error(
        pick(locale, "Desktop offline — will not draft in the cloud.", "桌面离线 — 不会在云端起草。"),
      );
      return;
    }
    setBusy(true);
    try {
      let id = sessionId;
      if (!id) {
        const created = await createSession({ data: { title: "New session" } });
        id = created.id;
        await navigate({
          to: "/sessions/$sessionId",
          params: { sessionId: id },
          search: {
            relay: search.relay,
            session: search.session,
          },
        });
      }
      try {
        if (mode === "steer") link.sendSteer(text);
        else if (mode === "comment") link.sendComment(text);
        else link.sendPrompt(text);
      } catch (e) {
        toast.error(e instanceof Error ? e.message : pick(locale, "Relay not connected", "中继未连接"));
        setBusy(false);
        return;
      }
      if (mode === "prompt") {
        const bundle = await sendPrompt({ data: { sessionId: id, prompt: text } });
        if (bundle) {
          setSession(bundle.session);
          setMessages(bundle.messages);
        }
        await refreshList();
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : pick(locale, "Send failed", "发送失败"));
    } finally {
      setBusy(false);
    }
  };

  const handleTemplate = async (templateId: string) => {
    setBusy(true);
    try {
      let id = sessionId;
      if (!id) {
        const created = await createSession({ data: { title: "New session" } });
        id = created.id;
        await navigate({
          to: "/sessions/$sessionId",
          params: { sessionId: id },
          search: {
            relay: search.relay,
            session: search.session,
          },
        });
      }
      const bundle = await applyTemplate({ data: { sessionId: id, templateId, locale } });
      if (bundle) {
        setSession(bundle.session);
        setMessages(bundle.messages);
      }
      await refreshList();
      setNavOpen(false);
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed");
    } finally {
      setBusy(false);
    }
  };

  const handleRegate = async () => {
    if (!sessionId) return;
    setBusy(true);
    try {
      const bundle = await runGateAgain({ data: { sessionId } });
      if (bundle) {
        setSession(bundle.session);
        setMessages(bundle.messages);
      }
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed");
    } finally {
      setBusy(false);
    }
  };

  const handleDeploy = (opts: { networkId: string; module: string; digest?: string }) => {
    if (!link.desktopOnline) {
      toast.error(pick(locale, "Desktop offline. Deploy stays on that machine.", "桌面离线。部署只在那台机器上。"));
      return;
    }
    try {
      link.sendDeploy(opts);
      toast(pick(locale, "Deploy command sent to desktop.", "部署命令已发到桌面。"));
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Failed");
    }
  };

  const empty = !sessionId || merged.length === 0;

  return (
    <div className="relative flex h-dvh overflow-hidden bg-bg text-fg">
      <div
        className="pointer-events-none absolute inset-0 bg-[radial-gradient(80%_50%_at_50%_-10%,rgba(4,26,66,0.55),transparent_60%)]"
        aria-hidden
      />
      <div className="relative hidden w-[260px] shrink-0 lg:block">
        <SessionSidebar
          sessions={sessions}
          activeId={sessionId}
          onNew={() => void handleNew()}
          onDelete={(id) => void handleDelete(id)}
          creating={creating}
        />
      </div>

      {navOpen && (
        <div className="fixed inset-0 z-40 lg:hidden">
          <button
            type="button"
            className="absolute inset-0 bg-black/60"
            aria-label="Close"
            onClick={() => setNavOpen(false)}
          />
          <div className="relative h-full w-[280px]">
            <SessionSidebar
              sessions={sessions}
              activeId={sessionId}
              onNew={() => void handleNew()}
              onDelete={(id) => void handleDelete(id)}
              creating={creating}
            />
          </div>
        </div>
      )}

      <div className="relative flex min-w-0 flex-1 flex-col">
        <SessionHeader
          session={session}
          desktopOnline={link.desktopOnline}
          platformOnline={link.platformOnline}
          relayLive={link.status === "live"}
          connecting={link.status === "connecting"}
          onRename={sessionId ? (title) => void handleRename(title) : undefined}
          onMenu={() => setNavOpen(true)}
          onRail={() => setRailOpen(true)}
        />
        <DesktopLinkBar link={link} sessionId={sessionId} />

        <div className="min-h-0 flex-1 overflow-y-auto">
          {empty ? (
            <EmptySession
              onTemplate={(id) => void handleTemplate(id)}
              busy={busy}
              link={link}
              sessionId={sessionId}
            />
          ) : (
            <Transcript messages={merged} running={busy && link.desktopOnline} />
          )}
        </div>
        <Composer
          disabled={busy}
          desktopOnline={link.desktopOnline}
          running={busy && link.desktopOnline}
          onCancel={() => {
            try {
              link.sendCancel();
            } catch (e) {
              toast.error(e instanceof Error ? e.message : "Failed");
            }
          }}
          onSend={(t, mode) => void handleSend(t, mode)}
        />
      </div>

      <div className="relative hidden w-[320px] shrink-0 xl:block">
        <GateRail
          session={session}
          link={link}
          onRegate={() => void handleRegate()}
          onDeploy={handleDeploy}
          desktopOnline={link.desktopOnline}
          busy={busy}
        />
      </div>

      {railOpen && (
        <div className="fixed inset-0 z-40 xl:hidden">
          <button
            type="button"
            className="absolute inset-0 bg-black/60"
            aria-label="Close"
            onClick={() => setRailOpen(false)}
          />
          <div className="absolute inset-y-0 right-0 w-[min(100%,360px)]">
            <GateRail
              session={session}
              link={link}
              onRegate={() => void handleRegate()}
              onDeploy={handleDeploy}
              desktopOnline={link.desktopOnline}
              busy={busy}
            />
          </div>
        </div>
      )}
    </div>
  );
}
