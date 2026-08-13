/**
 * HTTP auth + per-session share minting.
 *
 * SIWE identifies a wallet account. Minted share tokens are hashed at rest.
 * Deploy keys never pass through these routes.
 */

import {
  NONCE_TTL_MS,
  SESSION_TTL_MS,
  SHARE_TTL_MS,
  type AccountStore,
  bearerFromRequest,
  ensurePersonalOrg,
  hashToken,
  randomToken,
  resolveSession,
  resolveShare,
  tokenFromUrl,
} from "./accounts";
import {
  canManageMembers,
  parseOrgMemberRole,
  parseShareRole,
  resolveViewerAccess,
  type WriteCap,
} from "./policy";
import { D1AccountStore, type D1Like } from "./d1";
import { MemoryAccountStore } from "./accounts";
import {
  SIWE_STATEMENT,
  buildSiweMessage,
  checkSiweFields,
  normalizeAddress,
  parseSiweMessage,
} from "./siwe";
import { recoverSiweSigner } from "./verify";

export interface AuthEnv {
  DB?: D1Like;
  SIWE_DOMAIN?: string;
  SIWE_URI?: string;
  AUTH_REQUIRED?: string;
}

let memoryStore: MemoryAccountStore | undefined;

export function getAccountStore(env: AuthEnv): AccountStore {
  if (env.DB) return new D1AccountStore(env.DB);
  if (!memoryStore) memoryStore = new MemoryAccountStore();
  return memoryStore;
}

/** Test-only: drop the process-local memory backend. */
export function resetMemoryAccountStore(): void {
  memoryStore = undefined;
}

export function corsHeaders(request: Request): HeadersInit {
  const origin = request.headers.get("origin") ?? "*";
  return {
    "access-control-allow-origin": origin,
    "access-control-allow-headers": "authorization, content-type",
    "access-control-allow-methods": "GET, POST, OPTIONS",
    vary: "Origin",
  };
}

export function withCors(request: Request, response: Response): Response {
  const headers = new Headers(response.headers);
  for (const [key, value] of Object.entries(corsHeaders(request))) {
    headers.set(key, value);
  }
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
}

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}

function siweDomain(request: Request, env: AuthEnv): string {
  if (env.SIWE_DOMAIN?.trim()) return env.SIWE_DOMAIN.trim();
  const origin = request.headers.get("origin");
  if (origin) {
    try {
      return new URL(origin).host;
    } catch {
      /* fall through */
    }
  }
  return new URL(request.url).host;
}

function siweUri(request: Request, env: AuthEnv, domain: string): string {
  if (env.SIWE_URI?.trim()) return env.SIWE_URI.trim();
  const origin = request.headers.get("origin");
  if (origin) return origin.endsWith("/") ? origin : `${origin}/`;
  const proto = new URL(request.url).protocol;
  return `${proto}//${domain}/`;
}

export async function handleAuth(
  request: Request,
  env: AuthEnv,
  store: AccountStore,
  nowMs = Date.now(),
): Promise<Response | null> {
  const url = new URL(request.url);

  if (request.method === "OPTIONS" && url.pathname.startsWith("/api/")) {
    return new Response(null, { status: 204, headers: corsHeaders(request) });
  }

  if (request.method === "GET" && url.pathname === "/api/auth/siwe/nonce") {
    const address = normalizeAddress(url.searchParams.get("address") ?? "");
    if (!address) return json({ ok: false, error: "valid address required" }, 400);
    const chainId = Number(url.searchParams.get("chainId") ?? "1952");
    if (!Number.isInteger(chainId) || chainId <= 0) {
      return json({ ok: false, error: "invalid chain id" }, 400);
    }
    const nonce = randomToken(16);
    const issuedAt = new Date(nowMs).toISOString();
    const expirationTime = new Date(nowMs + NONCE_TTL_MS).toISOString();
    await store.putNonce(address, nonce, expirationTime);
    const domain = siweDomain(request, env);
    const uri = siweUri(request, env, domain);
    const message = buildSiweMessage({
      domain,
      address,
      uri,
      nonce,
      chainId,
      issuedAt,
      expirationTime,
    });
    return json({
      ok: true,
      address,
      nonce,
      domain,
      uri,
      chainId,
      statement: SIWE_STATEMENT,
      issuedAt,
      expirationTime,
      message,
    });
  }

  if (request.method === "POST" && url.pathname === "/api/auth/siwe/verify") {
    let body: unknown;
    try {
      body = await request.json();
    } catch {
      return json({ ok: false, error: "invalid json" }, 400);
    }
    if (
      typeof body !== "object" ||
      body === null ||
      typeof (body as { message?: unknown }).message !== "string" ||
      typeof (body as { signature?: unknown }).signature !== "string"
    ) {
      return json({ ok: false, error: "message and signature required" }, 400);
    }
    const message = (body as { message: string }).message;
    const signature = (body as { signature: string }).signature;
    const fields = parseSiweMessage(message);
    if (!fields) return json({ ok: false, error: "invalid SIWE message" }, 400);
    const pending = await store.takeNonce(fields.address);
    if (!pending) return json({ ok: false, error: "unknown or reused nonce" }, 401);
    if (Date.parse(pending.expiresAt) <= nowMs) {
      return json({ ok: false, error: "nonce expired" }, 401);
    }
    const fieldErr = checkSiweFields(fields, {
      nowMs,
      expectedDomain: siweDomain(request, env),
      expectedAddress: fields.address,
      expectedNonce: pending.nonce,
    });
    if (fieldErr) return json({ ok: false, error: fieldErr }, 401);
    const signer = await recoverSiweSigner(message, signature);
    if (!signer || signer !== fields.address) {
      return json({ ok: false, error: "signature does not match address" }, 401);
    }
    const now = new Date(nowMs).toISOString();
    const user = await store.upsertUser(fields.address, now);
    const org = await ensurePersonalOrg(store, user, now);
    const token = randomToken(32);
    const expiresAt = new Date(nowMs + SESSION_TTL_MS).toISOString();
    await store.putSession(await hashToken(token), {
      userId: user.id,
      address: user.address,
      expiresAt,
      orgId: org.id,
    });
    return json({
      ok: true,
      token,
      address: user.address,
      userId: user.id,
      orgId: org.id,
      expiresAt,
    });
  }

  if (request.method === "GET" && url.pathname === "/api/auth/me") {
    const session = await resolveSession(
      store,
      bearerFromRequest(request) ?? tokenFromUrl(url),
      nowMs,
    );
    if (!session) return json({ ok: false, error: "not signed in" }, 401);
    const orgs = await store.listOrgsForUser(session.userId);
    return json({
      ok: true,
      address: session.address,
      userId: session.userId,
      orgId: session.orgId ?? orgs[0]?.id,
      orgs,
      expiresAt: session.expiresAt,
    });
  }

  if (request.method === "POST" && url.pathname === "/api/auth/logout") {
    const raw = bearerFromRequest(request) ?? tokenFromUrl(url);
    if (raw) await store.deleteSession(await hashToken(raw));
    return json({ ok: true });
  }

  const shareMint = url.pathname.match(/^\/api\/sessions\/([^/]+)\/share$/u);
  if (request.method === "POST" && shareMint) {
    const sessionId = decodeURIComponent(shareMint[1] ?? "");
    if (!sessionId) return json({ ok: false, error: "missing session id" }, 400);
    const session = await resolveSession(
      store,
      bearerFromRequest(request) ?? tokenFromUrl(url),
      nowMs,
    );
    if (!session) return json({ ok: false, error: "sign in to mint a share link" }, 401);
    let role = parseShareRole("readonly") ?? "readonly";
    try {
      const body = await request.json().catch(() => ({}));
      if (isRecord(body) && body.role !== undefined) {
        const parsed = parseShareRole(body.role);
        if (!parsed) return json({ ok: false, error: "role must be readonly|comment|command" }, 400);
        role = parsed;
      }
    } catch {
      /* empty body is readonly */
    }
    const access = await resolveViewerAccess(store, bearerFromRequest(request) ?? tokenFromUrl(url), sessionId, nowMs, false);
    if (!access.ok || access.writeCap !== "command") {
      return json({ ok: false, error: "only org members can mint share links" }, 403);
    }
    const token = randomToken(24);
    const expiresAt = new Date(nowMs + SHARE_TTL_MS).toISOString();
    await store.putShare(await hashToken(token), {
      sessionId,
      ownerId: session.userId,
      role,
      expiresAt,
    });
    return json({
      ok: true,
      token,
      sessionId,
      role,
      expiresAt,
    });
  }

  const orgHandled = await handleOrgRoutes(request, url, store, nowMs);
  if (orgHandled) return orgHandled;

  return null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

async function requireSession(
  request: Request,
  url: URL,
  store: AccountStore,
  nowMs: number,
) {
  return resolveSession(store, bearerFromRequest(request) ?? tokenFromUrl(url), nowMs);
}

async function handleOrgRoutes(
  request: Request,
  url: URL,
  store: AccountStore,
  nowMs: number,
): Promise<Response | null> {
  if (request.method === "GET" && url.pathname === "/api/orgs") {
    const session = await requireSession(request, url, store, nowMs);
    if (!session) return json({ ok: false, error: "not signed in" }, 401);
    const orgs = await store.listOrgsForUser(session.userId);
    return json({ ok: true, orgId: session.orgId, orgs });
  }

  if (request.method === "POST" && url.pathname === "/api/orgs") {
    const session = await requireSession(request, url, store, nowMs);
    if (!session) return json({ ok: false, error: "not signed in" }, 401);
    let name = "Workspace";
    try {
      const body = await request.json();
      if (isRecord(body) && typeof body.name === "string" && body.name.trim()) {
        name = body.name.trim().slice(0, 80);
      }
    } catch {
      /* default name */
    }
    const now = new Date(nowMs).toISOString();
    const org = await store.createOrg({
      id: `org:${randomToken(8)}`,
      name,
      createdAt: now,
      createdBy: session.userId,
    });
    await store.putMember({
      orgId: org.id,
      userId: session.userId,
      address: session.address,
      role: "owner",
      createdAt: now,
    });
    return json({ ok: true, org });
  }

  const select = url.pathname.match(/^\/api\/orgs\/([^/]+)\/select$/u);
  if (request.method === "POST" && select) {
    const orgId = decodeURIComponent(select[1] ?? "");
    const session = await requireSession(request, url, store, nowMs);
    if (!session) return json({ ok: false, error: "not signed in" }, 401);
    if (!orgId || !(await store.getMember(orgId, session.userId))) {
      return json({ ok: false, error: "not a member of that org" }, 403);
    }
    const raw = bearerFromRequest(request) ?? tokenFromUrl(url);
    if (raw) {
      await store.putSession(await hashToken(raw), { ...session, orgId });
    }
    return json({ ok: true, orgId });
  }

  const membersPath = url.pathname.match(/^\/api\/orgs\/([^/]+)\/members$/u);
  if (membersPath) {
    const orgId = decodeURIComponent(membersPath[1] ?? "");
    const session = await requireSession(request, url, store, nowMs);
    if (!session) return json({ ok: false, error: "not signed in" }, 401);
    const self = await store.getMember(orgId, session.userId);
    if (!self) return json({ ok: false, error: "not a member of that org" }, 403);
    if (request.method === "GET") {
      return json({ ok: true, orgId, members: await store.listMembers(orgId) });
    }
    if (request.method === "POST") {
      if (!canManageMembers(self.role)) {
        return json({ ok: false, error: "only owner/admin can add members" }, 403);
      }
      let body: unknown;
      try {
        body = await request.json();
      } catch {
        return json({ ok: false, error: "invalid json" }, 400);
      }
      if (!isRecord(body) || typeof body.address !== "string") {
        return json({ ok: false, error: "address required" }, 400);
      }
      const invited = await store.getUserByAddress(body.address);
      if (!invited) {
        return json({ ok: false, error: "invitee must sign in with SIWE once first" }, 404);
      }
      const role = parseOrgMemberRole(body.role) ?? "member";
      if (role === "owner") {
        return json({ ok: false, error: "cannot grant owner via invite" }, 400);
      }
      await store.putMember({
        orgId,
        userId: invited.id,
        address: invited.address,
        role,
        createdAt: new Date(nowMs).toISOString(),
      });
      return json({ ok: true, member: await store.getMember(orgId, invited.id) });
    }
    if (request.method === "DELETE") {
      if (!canManageMembers(self.role)) {
        return json({ ok: false, error: "only owner/admin can remove members" }, 403);
      }
      const address = url.searchParams.get("address") ?? "";
      const target = await store.getUserByAddress(address);
      if (!target) return json({ ok: false, error: "unknown member" }, 404);
      const existing = await store.getMember(orgId, target.id);
      if (!existing) return json({ ok: false, error: "not a member" }, 404);
      if (existing.role === "owner") {
        return json({ ok: false, error: "cannot remove the owner" }, 400);
      }
      await store.deleteMember(orgId, target.id);
      return json({ ok: true });
    }
  }

  const claim = url.pathname.match(/^\/api\/sessions\/([^/]+)\/claim$/u);
  if (request.method === "POST" && claim) {
    const sessionId = decodeURIComponent(claim[1] ?? "");
    const session = await requireSession(request, url, store, nowMs);
    if (!session) return json({ ok: false, error: "not signed in" }, 401);
    if (!sessionId) return json({ ok: false, error: "missing session id" }, 400);
    const orgId = session.orgId;
    if (!orgId || !(await store.getMember(orgId, session.userId))) {
      return json({ ok: false, error: "select an org first" }, 400);
    }
    const existing = await store.getRoomGrant(sessionId);
    if (existing && existing.orgId !== orgId) {
      return json({ ok: false, error: "session already claimed by another org" }, 403);
    }
    const grant = existing ?? {
      sessionId,
      orgId,
      ownerId: session.userId,
      claimedAt: new Date(nowMs).toISOString(),
    };
    if (!existing) await store.putRoomGrant(grant);
    return json({ ok: true, grant });
  }

  return null;
}

export async function viewerAllowed(
  request: Request,
  url: URL,
  store: AccountStore,
  sessionId: string,
  nowMs: number,
  fallback: (url: URL) => boolean,
): Promise<boolean> {
  const access = await resolveViewerAccess(
    store,
    bearerFromRequest(request) ?? tokenFromUrl(url),
    sessionId,
    nowMs,
    fallback(url),
  );
  return access.ok;
}

export async function shareAllowed(
  request: Request,
  url: URL,
  store: AccountStore,
  sessionId: string,
  nowMs: number,
  fallback: (url: URL) => boolean,
): Promise<boolean> {
  const access = await resolveViewerAccess(
    store,
    tokenFromUrl(url) ?? bearerFromRequest(request),
    sessionId,
    nowMs,
    fallback(url),
  );
  return access.ok;
}

export async function accessFor(
  request: Request,
  url: URL,
  store: AccountStore,
  sessionId: string,
  nowMs: number,
  fallbackOpen: boolean,
): Promise<import("./policy").ViewerAccess> {
  return resolveViewerAccess(
    store,
    bearerFromRequest(request) ?? tokenFromUrl(url),
    sessionId,
    nowMs,
    fallbackOpen,
  );
}

export function writeCapQuery(cap: WriteCap): string {
  return cap;
}
