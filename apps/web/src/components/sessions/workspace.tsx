import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";
import { SessionSidebar } from "@/components/sessions/session-sidebar";
import { Transcript } from "@/components/sessions/transcript";
import { Composer } from "@/components/sessions/composer";
import { GateRail } from "@/components/sessions/gate-rail";
import { EmptySession } from "@/components/sessions/empty-session";
import { SessionHeader } from "@/components/sessions/session-header";
import { pick, useLocale } from "@/lib/i18n";
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

  const handleNew = async () => {
    setCreating(true);
    try {
      const created = await createSession({ data: { title: "New session" } });
      await refreshList();
      setNavOpen(false);
      await navigate({ to: "/sessions/$sessionId", params: { sessionId: created.id } });
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

  const handleSend = async (text: string) => {
    setBusy(true);
    setMessages((prev) => [
      ...prev,
      {
        id: `tmp-${Date.now()}`,
        role: "user",
        kind: "text",
        content: text,
        meta: null,
        createdAt: new Date().toISOString(),
      },
    ]);
    try {
      let id = sessionId;
      if (!id) {
        const created = await createSession({ data: { title: "New session" } });
        id = created.id;
        await navigate({ to: "/sessions/$sessionId", params: { sessionId: id } });
      }
      const bundle = await sendPrompt({ data: { sessionId: id, prompt: text } });
      if (bundle) {
        setSession(bundle.session);
        setMessages(bundle.messages);
      }
      await refreshList();
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
        await navigate({ to: "/sessions/$sessionId", params: { sessionId: id } });
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

  const empty = !sessionId || messages.length === 0;

  return (
    <div className="flex h-dvh overflow-hidden bg-bg text-ink">
      <div className="hidden w-[260px] shrink-0 lg:block">
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

      <div className="flex min-w-0 flex-1 flex-col">
        <SessionHeader
          session={session}
          running={busy}
          onRename={sessionId ? (title) => void handleRename(title) : undefined}
          onMenu={() => setNavOpen(true)}
          onRail={() => setRailOpen(true)}
        />

        <div className="min-h-0 flex-1 overflow-y-auto">
          {empty ? (
            <EmptySession onTemplate={(id) => void handleTemplate(id)} busy={busy} />
          ) : (
            <Transcript messages={messages} running={busy} />
          )}
        </div>
        <Composer disabled={busy} onSend={(t) => void handleSend(t)} />
      </div>

      <div className="hidden w-[320px] shrink-0 xl:block">
        <GateRail session={session} onRegate={() => void handleRegate()} busy={busy} />
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
            <GateRail session={session} onRegate={() => void handleRegate()} busy={busy} />
          </div>
        </div>
      )}
    </div>
  );
}
