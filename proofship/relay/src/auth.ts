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
  hashToken,
  randomToken,
  resolveSession,
  resolveShare,
  tokenFromUrl,
} from "./accounts";
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
    const token = randomToken(32);
    const expiresAt = new Date(nowMs + SESSION_TTL_MS).toISOString();
    await store.putSession(await hashToken(token), {
      userId: user.id,
      address: user.address,
      expiresAt,
    });
    return json({
      ok: true,
      token,
      address: user.address,
      userId: user.id,
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
    return json({
      ok: true,
      address: session.address,
      userId: session.userId,
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
    const token = randomToken(24);
    const expiresAt = new Date(nowMs + SHARE_TTL_MS).toISOString();
    await store.putShare(await hashToken(token), {
      sessionId,
      ownerId: session.userId,
      role: "readonly",
      expiresAt,
    });
    return json({
      ok: true,
      token,
      sessionId,
      role: "readonly",
      expiresAt,
    });
  }

  return null;
}

export async function viewerAllowed(
  request: Request,
  url: URL,
  store: AccountStore,
  nowMs: number,
  fallback: (url: URL) => boolean,
): Promise<boolean> {
  const session = await resolveSession(
    store,
    bearerFromRequest(request) ?? tokenFromUrl(url),
    nowMs,
  );
  if (session) return true;
  return fallback(url);
}

export async function shareAllowed(
  request: Request,
  url: URL,
  store: AccountStore,
  sessionId: string,
  nowMs: number,
  fallback: (url: URL) => boolean,
): Promise<boolean> {
  const token = tokenFromUrl(url) ?? bearerFromRequest(request);
  if (await resolveShare(store, token, sessionId, nowMs)) return true;
  const session = await resolveSession(store, token, nowMs);
  if (session) return true;
  return fallback(url);
}
