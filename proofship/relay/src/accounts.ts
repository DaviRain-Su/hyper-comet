/**
 * Account / session / minted-share store.
 *
 * Memory backend is the local spike and the unit-test double. D1 is the
 * production backend (same interface). Session tokens are stored hashed;
 * the raw token is returned once. Private keys never enter this module.
 */

import { normalizeAddress } from "./siwe";

export const SESSION_TTL_MS = 7 * 24 * 60 * 60 * 1000;
export const NONCE_TTL_MS = 10 * 60 * 1000;
export const SHARE_TTL_MS = 30 * 24 * 60 * 60 * 1000;

export type ShareRole = "readonly";

export interface AccountUser {
  id: string;
  address: string;
  createdAt: string;
}

export interface AccountSession {
  userId: string;
  address: string;
  expiresAt: string;
}

export interface AccountShare {
  sessionId: string;
  ownerId: string;
  role: ShareRole;
  expiresAt: string;
}

export interface AccountStore {
  putNonce(address: string, nonce: string, expiresAt: string): Promise<void>;
  takeNonce(address: string): Promise<{ nonce: string; expiresAt: string } | null>;
  upsertUser(address: string, now: string): Promise<AccountUser>;
  putSession(tokenHash: string, session: AccountSession): Promise<void>;
  getSession(tokenHash: string): Promise<AccountSession | null>;
  deleteSession(tokenHash: string): Promise<void>;
  putShare(tokenHash: string, share: AccountShare): Promise<void>;
  getShare(tokenHash: string): Promise<AccountShare | null>;
}

export class MemoryAccountStore implements AccountStore {
  private nonces = new Map<string, { nonce: string; expiresAt: string }>();
  private users = new Map<string, AccountUser>();
  private sessions = new Map<string, AccountSession>();
  private shares = new Map<string, AccountShare>();

  async putNonce(address: string, nonce: string, expiresAt: string): Promise<void> {
    const key = normalizeAddress(address);
    if (!key) throw new Error("invalid address");
    this.nonces.set(key, { nonce, expiresAt });
  }

  async takeNonce(address: string): Promise<{ nonce: string; expiresAt: string } | null> {
    const key = normalizeAddress(address);
    if (!key) return null;
    const row = this.nonces.get(key) ?? null;
    if (row) this.nonces.delete(key);
    return row;
  }

  async upsertUser(address: string, now: string): Promise<AccountUser> {
    const normalized = normalizeAddress(address);
    if (!normalized) throw new Error("invalid address");
    const existing = this.users.get(normalized);
    if (existing) return existing;
    const user: AccountUser = {
      id: `user:${normalized}`,
      address: normalized,
      createdAt: now,
    };
    this.users.set(normalized, user);
    return user;
  }

  async putSession(tokenHash: string, session: AccountSession): Promise<void> {
    this.sessions.set(tokenHash, session);
  }

  async getSession(tokenHash: string): Promise<AccountSession | null> {
    return this.sessions.get(tokenHash) ?? null;
  }

  async deleteSession(tokenHash: string): Promise<void> {
    this.sessions.delete(tokenHash);
  }

  async putShare(tokenHash: string, share: AccountShare): Promise<void> {
    this.shares.set(tokenHash, share);
  }

  async getShare(tokenHash: string): Promise<AccountShare | null> {
    return this.shares.get(tokenHash) ?? null;
  }
}

export function sessionStillValid(session: AccountSession, nowMs: number): boolean {
  const exp = Date.parse(session.expiresAt);
  return !Number.isNaN(exp) && exp > nowMs;
}

export function shareStillValid(share: AccountShare, nowMs: number): boolean {
  const exp = Date.parse(share.expiresAt);
  return !Number.isNaN(exp) && exp > nowMs;
}

export async function hashToken(token: string): Promise<string> {
  const bytes = new TextEncoder().encode(token);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return bufferToHex(digest);
}

export function randomToken(bytes = 32): string {
  const buf = new Uint8Array(bytes);
  crypto.getRandomValues(buf);
  return bufferToHex(buf.buffer);
}

function bufferToHex(buf: ArrayBuffer): string {
  return [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function bearerFromRequest(request: Request): string | null {
  const header = request.headers.get("authorization");
  if (header?.toLowerCase().startsWith("bearer ")) {
    const token = header.slice(7).trim();
    return token || null;
  }
  return null;
}

export function tokenFromUrl(url: URL): string | null {
  const token =
    url.searchParams.get("sessionToken") ??
    url.searchParams.get("viewerToken") ??
    url.searchParams.get("token") ??
    url.searchParams.get("shareToken");
  return token && token.trim() ? token.trim() : null;
}

export async function resolveSession(
  store: AccountStore,
  token: string | null,
  nowMs: number,
): Promise<AccountSession | null> {
  if (!token) return null;
  const session = await store.getSession(await hashToken(token));
  if (!session || !sessionStillValid(session, nowMs)) return null;
  return session;
}

export async function resolveShare(
  store: AccountStore,
  token: string | null,
  sessionId: string,
  nowMs: number,
): Promise<AccountShare | null> {
  if (!token) return null;
  const share = await store.getShare(await hashToken(token));
  if (!share || !shareStillValid(share, nowMs)) return null;
  if (share.sessionId !== sessionId) return null;
  if (share.role !== "readonly") return null;
  return share;
}
