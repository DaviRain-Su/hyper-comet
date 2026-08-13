import { createServerFn } from "@tanstack/react-start";
import { authMiddleware } from "@/lib/auth/middleware";
import { getSql, type Sql } from "@/lib/db";
import { extractModule, gateSummary, runGate, type GateResult } from "@/lib/gate";
import { templateById } from "@/lib/templates";

export type SessionStatus = "idle" | "running" | "failed" | "ready";

export type SessionRow = {
  id: string;
  title: string;
  status: SessionStatus;
  source: string;
  moduleName: string | null;
  gate: GateResult | null;
  createdAt: string;
  updatedAt: string;
};

export type MessageRow = {
  id: string;
  role: "user" | "agent" | "tool" | "system";
  kind: "text" | "lean" | "gate";
  content: string;
  meta: GateResult | null;
  createdAt: string;
};

type SessionDb = {
  id: string;
  user_id: string;
  title: string;
  status: string;
  source: string;
  module_name: string | null;
  gate_json: string | null;
  created_at: string;
  updated_at: string;
};

type MessageDb = {
  id: string;
  session_id: string;
  user_id: string;
  role: string;
  kind: string;
  content: string;
  meta_json: string | null;
  created_at: string;
};

function mapSession(row: SessionDb): SessionRow {
  let gate: GateResult | null = null;
  if (row.gate_json) {
    try {
      gate = JSON.parse(row.gate_json) as GateResult;
    } catch {
      gate = null;
    }
  }
  return {
    id: row.id,
    title: row.title,
    status: row.status as SessionStatus,
    source: row.source,
    moduleName: row.module_name,
    gate,
    createdAt: String(row.created_at),
    updatedAt: String(row.updated_at),
  };
}

function mapMessage(row: MessageDb): MessageRow {
  let meta: GateResult | null = null;
  if (row.meta_json) {
    try {
      meta = JSON.parse(row.meta_json) as GateResult;
    } catch {
      meta = null;
    }
  }
  return {
    id: row.id,
    role: row.role as MessageRow["role"],
    kind: row.kind as MessageRow["kind"],
    content: row.content,
    meta,
    createdAt: String(row.created_at),
  };
}

function nid() {
  return crypto.randomUUID();
}

function titleFrom(text: string) {
  const t = text.replace(/\s+/g, " ").trim();
  return t.length > 48 ? `${t.slice(0, 48)}…` : t || "Untitled session";
}

async function loadBundle(sql: Sql, userId: string, id: string) {
  const sessions = await sql<SessionDb>`
    select * from ship_sessions
    where id = ${id} and user_id = ${userId}
    limit 1
  `;
  const session = sessions[0];
  if (!session) return null;
  const messages = await sql<MessageDb>`
    select * from ship_messages
    where session_id = ${id} and user_id = ${userId}
    order by created_at asc
  `;
  return { session: mapSession(session), messages: messages.map(mapMessage) };
}

export const listSessions = createServerFn({ method: "GET" })
  .middleware([authMiddleware])
  .handler(async ({ context }) => {
    const sql = await getSql();
    const rows = await sql<SessionDb>`
      select * from ship_sessions
      where user_id = ${context.userId}
      order by updated_at desc
    `;
    return rows.map(mapSession);
  });

export const getSession = createServerFn({ method: "GET" })
  .middleware([authMiddleware])
  .validator((input: { id: string }) => input)
  .handler(async ({ context, data }) => {
    const sql = await getSql();
    return loadBundle(sql, context.userId, data.id);
  });

export const createSession = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .validator((input: { title?: string }) => input)
  .handler(async ({ context, data }) => {
    const sql = await getSql();
    const id = nid();
    const title = data.title?.trim() || "New session";
    await sql`
      insert into ship_sessions (id, user_id, title, status)
      values (${id}, ${context.userId}, ${title}, ${"idle"})
    `;
    const rows = await sql<SessionDb>`
      select * from ship_sessions where id = ${id} and user_id = ${context.userId}
    `;
    return mapSession(rows[0]!);
  });

export const deleteSession = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .validator((input: { id: string }) => input)
  .handler(async ({ context, data }) => {
    const sql = await getSql();
    await sql`delete from ship_messages where session_id = ${data.id} and user_id = ${context.userId}`;
    await sql`delete from ship_sessions where id = ${data.id} and user_id = ${context.userId}`;
    return { ok: true as const };
  });

export const appendMessage = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .validator(
    (input: {
      sessionId: string;
      role: MessageRow["role"];
      kind: MessageRow["kind"];
      content: string;
      title?: string;
    }) => input,
  )
  .handler(async ({ context, data }) => {
    const sql = await getSql();
    const sessions = await sql<SessionDb>`
      select * from ship_sessions
      where id = ${data.sessionId} and user_id = ${context.userId}
      limit 1
    `;
    if (!sessions[0]) throw new Error("Session not found");
    await sql`
      insert into ship_messages (id, session_id, user_id, role, kind, content)
      values (${nid()}, ${data.sessionId}, ${context.userId}, ${data.role}, ${data.kind}, ${data.content})
    `;
    const title =
      data.title ||
      (sessions[0].title === "New session" && data.role === "user"
        ? titleFrom(data.content)
        : sessions[0].title);
    await sql`
      update ship_sessions
      set title = ${title}, updated_at = ${new Date().toISOString()}
      where id = ${data.sessionId} and user_id = ${context.userId}
    `;
    return loadBundle(sql, context.userId, data.sessionId);
  });

/** Queue a prompt for the desktop. This process never drafts. */
export const sendPrompt = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .validator((input: { sessionId: string; prompt: string }) => input)
  .handler(async ({ context, data }) => {
    const prompt = data.prompt.trim();
    if (!prompt) throw new Error("Empty prompt");
    const sql = await getSql();
    const sessions = await sql<SessionDb>`
      select * from ship_sessions
      where id = ${data.sessionId} and user_id = ${context.userId}
      limit 1
    `;
    const session = sessions[0];
    if (!session) throw new Error("Session not found");

    await sql`
      insert into ship_messages (id, session_id, user_id, role, kind, content)
      values (${nid()}, ${session.id}, ${context.userId}, ${"user"}, ${"text"}, ${prompt})
    `;
    await sql`
      insert into ship_messages (id, session_id, user_id, role, kind, content)
      values (
        ${nid()}, ${session.id}, ${context.userId}, ${"system"}, ${"text"},
        ${"Queued for your desktop. This page does not run an agent."}
      )
    `;
    const nextTitle = session.title === "New session" ? titleFrom(prompt) : session.title;
    await sql`
      update ship_sessions
      set status = ${"running"},
          title = ${nextTitle},
          updated_at = ${new Date().toISOString()}
      where id = ${session.id} and user_id = ${context.userId}
    `;
    return loadBundle(sql, context.userId, session.id);
  });

/** Load a known-good starter as a local preview — not an agent draft. */
export const applyTemplate = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .validator((input: { sessionId: string; templateId: string; locale?: "en" | "zh" }) => input)
  .handler(async ({ context, data }) => {
    const tpl = templateById(data.templateId);
    if (!tpl) throw new Error("Unknown template");
    const sql = await getSql();
    const sessions = await sql<SessionDb>`
      select * from ship_sessions
      where id = ${data.sessionId} and user_id = ${context.userId}
      limit 1
    `;
    const session = sessions[0];
    if (!session) throw new Error("Session not found");

    const prompt = data.locale === "zh" ? tpl.promptZh : tpl.prompt;
    const source = tpl.source;
    const gate = runGate(source);
    const title = data.locale === "zh" ? tpl.titleZh : tpl.title;
    const note =
      data.locale === "zh"
        ? `只读模板 ${tpl.module}。真正的起草和门禁在你的桌面端跑。把这段说明发给本机 agent。`
        : `Read-only starter ${tpl.module}. Real drafting and the gate run on your desktop. Send this spec to the local agent.`;

    await sql`
      insert into ship_messages (id, session_id, user_id, role, kind, content)
      values (${nid()}, ${session.id}, ${context.userId}, ${"user"}, ${"text"}, ${prompt})
    `;
    await sql`
      insert into ship_messages (id, session_id, user_id, role, kind, content)
      values (${nid()}, ${session.id}, ${context.userId}, ${"system"}, ${"text"}, ${note})
    `;
    await sql`
      insert into ship_messages (id, session_id, user_id, role, kind, content)
      values (${nid()}, ${session.id}, ${context.userId}, ${"agent"}, ${"lean"}, ${source})
    `;
    await sql`
      insert into ship_messages (id, session_id, user_id, role, kind, content, meta_json)
      values (
        ${nid()}, ${session.id}, ${context.userId}, ${"tool"}, ${"gate"},
        ${gateSummary(gate)}, ${JSON.stringify(gate)}
      )
    `;
    await sql`
      update ship_sessions
      set title = ${title},
          status = ${gate.passed ? "ready" : "failed"},
          source = ${source},
          module_name = ${gate.module},
          gate_json = ${JSON.stringify(gate)},
          updated_at = ${new Date().toISOString()}
      where id = ${session.id} and user_id = ${context.userId}
    `;
    return loadBundle(sql, context.userId, session.id);
  });

export const renameSession = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .validator((input: { id: string; title: string }) => input)
  .handler(async ({ context, data }) => {
    const title = data.title.replace(/\s+/g, " ").trim().slice(0, 80) || "Untitled session";
    const sql = await getSql();
    await sql`
      update ship_sessions
      set title = ${title}, updated_at = ${new Date().toISOString()}
      where id = ${data.id} and user_id = ${context.userId}
    `;
    return loadBundle(sql, context.userId, data.id);
  });

export const runGateAgain = createServerFn({ method: "POST" })
  .middleware([authMiddleware])
  .validator((input: { sessionId: string; source?: string }) => input)
  .handler(async ({ context, data }) => {
    const sql = await getSql();
    const sessions = await sql<SessionDb>`
      select * from ship_sessions
      where id = ${data.sessionId} and user_id = ${context.userId}
      limit 1
    `;
    const session = sessions[0];
    if (!session) throw new Error("Session not found");
    const source = (data.source ?? session.source).trim();
    if (!source) throw new Error("No source");
    const gate = runGate(source);
    const module = extractModule(source);
    const now = new Date().toISOString();
    await sql`
      insert into ship_messages (id, session_id, user_id, role, kind, content, meta_json)
      values (
        ${nid()}, ${session.id}, ${context.userId}, ${"tool"}, ${"gate"},
        ${gateSummary(gate)}, ${JSON.stringify(gate)}
      )
    `;
    await sql`
      update ship_sessions
      set status = ${gate.passed ? "ready" : "failed"},
          source = ${source},
          module_name = ${module},
          gate_json = ${JSON.stringify(gate)},
          updated_at = ${now}
      where id = ${session.id} and user_id = ${context.userId}
    `;
    return loadBundle(sql, context.userId, session.id);
  });
