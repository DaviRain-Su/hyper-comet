import { createServerFn } from "@tanstack/react-start";
import { authMiddleware } from "@/lib/auth/middleware";
import { getSql, type Sql } from "@/lib/db";
import { draftProgram } from "@/lib/agent";
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

async function persistTurn(opts: {
  sql: Sql;
  userId: string;
  sessionId: string;
  title?: string;
  userText?: string;
  agentNote: string;
  source: string;
  gate: GateResult;
}) {
  const now = new Date().toISOString();
  if (opts.userText) {
    await opts.sql`
      insert into ship_messages (id, session_id, user_id, role, kind, content)
      values (${nid()}, ${opts.sessionId}, ${opts.userId}, ${"user"}, ${"text"}, ${opts.userText})
    `;
  }
  await opts.sql`
    insert into ship_messages (id, session_id, user_id, role, kind, content)
    values (${nid()}, ${opts.sessionId}, ${opts.userId}, ${"agent"}, ${"text"}, ${opts.agentNote})
  `;
  await opts.sql`
    insert into ship_messages (id, session_id, user_id, role, kind, content)
    values (${nid()}, ${opts.sessionId}, ${opts.userId}, ${"agent"}, ${"lean"}, ${opts.source})
  `;
  const summary = gateSummary(opts.gate);
  await opts.sql`
    insert into ship_messages (id, session_id, user_id, role, kind, content, meta_json)
    values (
      ${nid()}, ${opts.sessionId}, ${opts.userId}, ${"tool"}, ${"gate"}, ${summary},
      ${JSON.stringify(opts.gate)}
    )
  `;
  const status: SessionStatus = opts.gate.passed ? "ready" : "failed";
  if (opts.title) {
    await opts.sql`
      update ship_sessions
      set title = ${opts.title},
          status = ${status},
          source = ${opts.source},
          module_name = ${opts.gate.module},
          gate_json = ${JSON.stringify(opts.gate)},
          updated_at = ${now}
      where id = ${opts.sessionId} and user_id = ${opts.userId}
    `;
  } else {
    await opts.sql`
      update ship_sessions
      set status = ${status},
          source = ${opts.source},
          module_name = ${opts.gate.module},
          gate_json = ${JSON.stringify(opts.gate)},
          updated_at = ${now}
      where id = ${opts.sessionId} and user_id = ${opts.userId}
    `;
  }
}

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
      update ship_sessions set status = ${"running"}, updated_at = ${new Date().toISOString()}
      where id = ${session.id} and user_id = ${context.userId}
    `;

    const draft = await draftProgram(prompt, session.source || undefined);
    if (!draft.ok) {
      const ask = draft.ask ?? draft.error;
      await sql`
        insert into ship_messages (id, session_id, user_id, role, kind, content)
        values (${nid()}, ${session.id}, ${context.userId}, ${"user"}, ${"text"}, ${prompt})
      `;
      await sql`
        insert into ship_messages (id, session_id, user_id, role, kind, content)
        values (${nid()}, ${session.id}, ${context.userId}, ${"agent"}, ${"text"}, ${ask})
      `;
      const nextTitle = session.title === "New session" ? titleFrom(prompt) : session.title;
      await sql`
        update ship_sessions
        set status = ${"idle"},
            title = ${nextTitle},
            updated_at = ${new Date().toISOString()}
        where id = ${session.id} and user_id = ${context.userId}
      `;
      return loadBundle(sql, context.userId, session.id);
    }

    let gate = runGate(draft.source);
    let source = draft.source;
    let note = draft.note;
    if (!gate.passed) {
      const repair = await draftProgram(
        `Gate failed. Repair the ProgramV1. Diagnostics:\n${gate.steps[0]?.diagnostics
          .map((d) => `${d.code}: ${d.message}`)
          .join("\n")}\n\nOriginal request:\n${prompt}`,
        draft.source,
      );
      if (repair.ok) {
        source = repair.source;
        gate = runGate(repair.source);
        note = `${note}\n\nRepair pass applied.`;
      }
    }

    await persistTurn({
      sql,
      userId: context.userId,
      sessionId: session.id,
      title: session.title === "New session" ? titleFrom(prompt) : undefined,
      userText: prompt,
      agentNote: note,
      source,
      gate,
    });

    return loadBundle(sql, context.userId, session.id);
  });

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

    await persistTurn({
      sql,
      userId: context.userId,
      sessionId: session.id,
      title,
      userText: prompt,
      agentNote:
        data.locale === "zh"
          ? `已载入模板 ${tpl.module}。源码仍须过门禁后才能上链。`
          : `Loaded starter ${tpl.module}. The gate still decides if it ships.`,
      source,
      gate,
    });

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
