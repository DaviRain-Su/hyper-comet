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
export const INVITE_TTL_MS = 14 * 24 * 60 * 60 * 1000;

export type ShareRole = "readonly" | "comment" | "command";
export type OrgMemberRole = "owner" | "admin" | "member";

export interface AccountUser {
  id: string;
  address: string;
  createdAt: string;
}

export interface AccountSession {
  userId: string;
  address: string;
  expiresAt: string;
  orgId?: string;
}

export interface AccountShare {
  sessionId: string;
  ownerId: string;
  role: ShareRole;
  expiresAt: string;
}

export interface AccountOrg {
  id: string;
  name: string;
  createdAt: string;
  createdBy: string;
}

export interface OrgMember {
  orgId: string;
  userId: string;
  address: string;
  role: OrgMemberRole;
  createdAt: string;
}

export interface RoomGrant {
  sessionId: string;
  orgId: string;
  ownerId: string;
  claimedAt: string;
}

export interface OrgInvite {
  orgId: string;
  role: Exclude<OrgMemberRole, "owner">;
  /** When set, only this wallet can accept. Null = open join link. */
  address: string | null;
  invitedBy: string;
  expiresAt: string;
  createdAt: string;
}

export interface AccountStore {
  putNonce(address: string, nonce: string, expiresAt: string): Promise<void>;
  takeNonce(address: string): Promise<{ nonce: string; expiresAt: string } | null>;
  upsertUser(address: string, now: string): Promise<AccountUser>;
  getUser(userId: string): Promise<AccountUser | null>;
  getUserByAddress(address: string): Promise<AccountUser | null>;
  putSession(tokenHash: string, session: AccountSession): Promise<void>;
  getSession(tokenHash: string): Promise<AccountSession | null>;
  deleteSession(tokenHash: string): Promise<void>;
  putShare(tokenHash: string, share: AccountShare): Promise<void>;
  getShare(tokenHash: string): Promise<AccountShare | null>;
  createOrg(org: AccountOrg): Promise<AccountOrg>;
  getOrg(orgId: string): Promise<AccountOrg | null>;
  listOrgsForUser(userId: string): Promise<AccountOrg[]>;
  putMember(member: OrgMember): Promise<void>;
  getMember(orgId: string, userId: string): Promise<OrgMember | null>;
  listMembers(orgId: string): Promise<OrgMember[]>;
  deleteMember(orgId: string, userId: string): Promise<void>;
  putRoomGrant(grant: RoomGrant): Promise<void>;
  getRoomGrant(sessionId: string): Promise<RoomGrant | null>;
  putInvite(tokenHash: string, invite: OrgInvite): Promise<void>;
  getInvite(tokenHash: string): Promise<OrgInvite | null>;
  deleteInvite(tokenHash: string): Promise<void>;
  listInvitesForOrg(orgId: string): Promise<OrgInvite[]>;
  listInvitesForAddress(address: string): Promise<Array<OrgInvite & { tokenHash: string }>>;
}

export class MemoryAccountStore implements AccountStore {
  private nonces = new Map<string, { nonce: string; expiresAt: string }>();
  private users = new Map<string, AccountUser>();
  private usersById = new Map<string, AccountUser>();
  private sessions = new Map<string, AccountSession>();
  private shares = new Map<string, AccountShare>();
  private orgs = new Map<string, AccountOrg>();
  private members = new Map<string, OrgMember>();
  private rooms = new Map<string, RoomGrant>();
  private invites = new Map<string, OrgInvite>();

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
    this.usersById.set(user.id, user);
    return user;
  }

  async getUser(userId: string): Promise<AccountUser | null> {
    return this.usersById.get(userId) ?? null;
  }

  async getUserByAddress(address: string): Promise<AccountUser | null> {
    const key = normalizeAddress(address);
    if (!key) return null;
    return this.users.get(key) ?? null;
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

  async createOrg(org: AccountOrg): Promise<AccountOrg> {
    this.orgs.set(org.id, org);
    return org;
  }

  async getOrg(orgId: string): Promise<AccountOrg | null> {
    return this.orgs.get(orgId) ?? null;
  }

  async listOrgsForUser(userId: string): Promise<AccountOrg[]> {
    const ids = [...this.members.values()]
      .filter((m) => m.userId === userId)
      .map((m) => m.orgId);
    return ids
      .map((id) => this.orgs.get(id))
      .filter((org): org is AccountOrg => Boolean(org));
  }

  async putMember(member: OrgMember): Promise<void> {
    this.members.set(`${member.orgId}:${member.userId}`, member);
  }

  async getMember(orgId: string, userId: string): Promise<OrgMember | null> {
    return this.members.get(`${orgId}:${userId}`) ?? null;
  }

  async listMembers(orgId: string): Promise<OrgMember[]> {
    return [...this.members.values()].filter((m) => m.orgId === orgId);
  }

  async deleteMember(orgId: string, userId: string): Promise<void> {
    this.members.delete(`${orgId}:${userId}`);
  }

  async putRoomGrant(grant: RoomGrant): Promise<void> {
    this.rooms.set(grant.sessionId, grant);
  }

  async getRoomGrant(sessionId: string): Promise<RoomGrant | null> {
    return this.rooms.get(sessionId) ?? null;
  }

  async putInvite(tokenHash: string, invite: OrgInvite): Promise<void> {
    this.invites.set(tokenHash, invite);
  }

  async getInvite(tokenHash: string): Promise<OrgInvite | null> {
    return this.invites.get(tokenHash) ?? null;
  }

  async deleteInvite(tokenHash: string): Promise<void> {
    this.invites.delete(tokenHash);
  }

  async listInvitesForOrg(orgId: string): Promise<OrgInvite[]> {
    return [...this.invites.values()].filter((i) => i.orgId === orgId);
  }

  async listInvitesForAddress(
    address: string,
  ): Promise<Array<OrgInvite & { tokenHash: string }>> {
    const key = normalizeAddress(address);
    if (!key) return [];
    return [...this.invites.entries()]
      .filter(([, invite]) => invite.address === key)
      .map(([tokenHash, invite]) => ({ ...invite, tokenHash }));
  }
}

export function personalOrgId(address: string): string {
  return `org:${address}`;
}

export async function ensurePersonalOrg(
  store: AccountStore,
  user: AccountUser,
  now: string,
): Promise<AccountOrg> {
  const id = personalOrgId(user.address);
  const existing = await store.getOrg(id);
  if (existing) {
    if (!(await store.getMember(id, user.id))) {
      await store.putMember({
        orgId: id,
        userId: user.id,
        address: user.address,
        role: "owner",
        createdAt: now,
      });
    }
    return existing;
  }
  const org: AccountOrg = {
    id,
    name: "Personal",
    createdAt: now,
    createdBy: user.id,
  };
  await store.createOrg(org);
  await store.putMember({
    orgId: id,
    userId: user.id,
    address: user.address,
    role: "owner",
    createdAt: now,
  });
  return org;
}

export function inviteStillValid(invite: OrgInvite, nowMs: number): boolean {
  const exp = Date.parse(invite.expiresAt);
  return !Number.isNaN(exp) && exp > nowMs;
}

/** Join every pending address-bound invite, plus an optional open/token invite. */
export async function acceptPendingInvites(
  store: AccountStore,
  user: AccountUser,
  now: string,
  nowMs: number,
  extraTokenHash?: string,
): Promise<AccountOrg[]> {
  const joined: AccountOrg[] = [];
  const pending = await store.listInvitesForAddress(user.address);
  for (const invite of pending) {
    const org = await acceptOneInvite(store, user, invite, invite.tokenHash, now, nowMs, true);
    if (org) joined.push(org);
  }
  if (extraTokenHash) {
    const invite = await store.getInvite(extraTokenHash);
    if (invite) {
      const org = await acceptOneInvite(store, user, invite, extraTokenHash, now, nowMs, false);
      if (org) joined.push(org);
    }
  }
  return joined;
}

async function acceptOneInvite(
  store: AccountStore,
  user: AccountUser,
  invite: OrgInvite,
  tokenHash: string,
  now: string,
  nowMs: number,
  addressBound: boolean,
): Promise<AccountOrg | null> {
  if (!inviteStillValid(invite, nowMs)) {
    await store.deleteInvite(tokenHash);
    return null;
  }
  if (invite.address && invite.address !== user.address) return null;
  if (await store.getMember(invite.orgId, user.id)) {
    if (addressBound || invite.address) await store.deleteInvite(tokenHash);
    return store.getOrg(invite.orgId);
  }
  await store.putMember({
    orgId: invite.orgId,
    userId: user.id,
    address: user.address,
    role: invite.role,
    createdAt: now,
  });
  if (invite.address) await store.deleteInvite(tokenHash);
  return store.getOrg(invite.orgId);
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
  return share;
}
