import type {
  AccountOrg,
  AccountShare,
  AccountStore,
  AccountSession,
  AccountUser,
  OrgInvite,
  OrgMember,
  RoomGrant,
} from "./accounts";
import { normalizeAddress } from "./siwe";

export interface D1Like {
  prepare(query: string): {
    bind(...values: unknown[]): {
      first<T = Record<string, unknown>>(): Promise<T | null>;
      all<T = Record<string, unknown>>(): Promise<{ results: T[] }>;
      run(): Promise<unknown>;
    };
  };
}

const SCHEMA = [
  `CREATE TABLE IF NOT EXISTS users (
     id TEXT PRIMARY KEY,
     address TEXT NOT NULL UNIQUE,
     created_at TEXT NOT NULL
   )`,
  `CREATE TABLE IF NOT EXISTS nonces (
     address TEXT PRIMARY KEY,
     nonce TEXT NOT NULL,
     expires_at TEXT NOT NULL
   )`,
  `CREATE TABLE IF NOT EXISTS sessions (
     token_hash TEXT PRIMARY KEY,
     user_id TEXT NOT NULL,
     address TEXT NOT NULL,
     expires_at TEXT NOT NULL,
     created_at TEXT NOT NULL,
     org_id TEXT
   )`,
  `CREATE TABLE IF NOT EXISTS shares (
     token_hash TEXT PRIMARY KEY,
     session_id TEXT NOT NULL,
     owner_id TEXT NOT NULL,
     role TEXT NOT NULL,
     expires_at TEXT NOT NULL,
     created_at TEXT NOT NULL
   )`,
  `CREATE TABLE IF NOT EXISTS orgs (
     id TEXT PRIMARY KEY,
     name TEXT NOT NULL,
     created_at TEXT NOT NULL,
     created_by TEXT NOT NULL
   )`,
  `CREATE TABLE IF NOT EXISTS org_members (
     org_id TEXT NOT NULL,
     user_id TEXT NOT NULL,
     address TEXT NOT NULL,
     role TEXT NOT NULL,
     created_at TEXT NOT NULL,
     PRIMARY KEY (org_id, user_id)
   )`,
  `CREATE TABLE IF NOT EXISTS room_grants (
     session_id TEXT PRIMARY KEY,
     org_id TEXT NOT NULL,
     owner_id TEXT NOT NULL,
     claimed_at TEXT NOT NULL
   )`,
  `CREATE TABLE IF NOT EXISTS org_invites (
     token_hash TEXT PRIMARY KEY,
     org_id TEXT NOT NULL,
     role TEXT NOT NULL,
     address TEXT,
     invited_by TEXT NOT NULL,
     expires_at TEXT NOT NULL,
     created_at TEXT NOT NULL
   )`,
];

export class D1AccountStore implements AccountStore {
  private ready: Promise<void> | null = null;

  constructor(private readonly db: D1Like) {}

  private async ensure(): Promise<void> {
    if (!this.ready) {
      this.ready = (async () => {
        for (const sql of SCHEMA) {
          await this.db.prepare(sql).bind().run();
        }
      })();
    }
    await this.ready;
  }

  async putNonce(address: string, nonce: string, expiresAt: string): Promise<void> {
    await this.ensure();
    const key = normalizeAddress(address);
    if (!key) throw new Error("invalid address");
    await this.db
      .prepare(
        "INSERT OR REPLACE INTO nonces (address, nonce, expires_at) VALUES (?, ?, ?)",
      )
      .bind(key, nonce, expiresAt)
      .run();
  }

  async takeNonce(address: string): Promise<{ nonce: string; expiresAt: string } | null> {
    await this.ensure();
    const key = normalizeAddress(address);
    if (!key) return null;
    const row = await this.db
      .prepare("SELECT nonce, expires_at AS expiresAt FROM nonces WHERE address = ?")
      .bind(key)
      .first<{ nonce: string; expiresAt: string }>();
    if (!row) return null;
    await this.db.prepare("DELETE FROM nonces WHERE address = ?").bind(key).run();
    return row;
  }

  async upsertUser(address: string, now: string): Promise<AccountUser> {
    await this.ensure();
    const normalized = normalizeAddress(address);
    if (!normalized) throw new Error("invalid address");
    const existing = await this.db
      .prepare("SELECT id, address, created_at AS createdAt FROM users WHERE address = ?")
      .bind(normalized)
      .first<AccountUser>();
    if (existing) return existing;
    const user: AccountUser = {
      id: `user:${normalized}`,
      address: normalized,
      createdAt: now,
    };
    await this.db
      .prepare("INSERT INTO users (id, address, created_at) VALUES (?, ?, ?)")
      .bind(user.id, user.address, user.createdAt)
      .run();
    return user;
  }

  async putSession(tokenHash: string, session: AccountSession): Promise<void> {
    await this.ensure();
    await this.db
      .prepare(
        "INSERT OR REPLACE INTO sessions (token_hash, user_id, address, expires_at, created_at, org_id) VALUES (?, ?, ?, ?, ?, ?)",
      )
      .bind(
        tokenHash,
        session.userId,
        session.address,
        session.expiresAt,
        new Date().toISOString(),
        session.orgId ?? "",
      )
      .run();
  }

  async getSession(tokenHash: string): Promise<AccountSession | null> {
    await this.ensure();
    const row = await this.db
      .prepare(
        "SELECT user_id AS userId, address, expires_at AS expiresAt, org_id AS orgId FROM sessions WHERE token_hash = ?",
      )
      .bind(tokenHash)
      .first<AccountSession & { orgId?: string }>();
    if (!row) return null;
    return { ...row, orgId: row.orgId || undefined };
  }

  async deleteSession(tokenHash: string): Promise<void> {
    await this.ensure();
    await this.db.prepare("DELETE FROM sessions WHERE token_hash = ?").bind(tokenHash).run();
  }

  async putShare(tokenHash: string, share: AccountShare): Promise<void> {
    await this.ensure();
    await this.db
      .prepare(
        "INSERT OR REPLACE INTO shares (token_hash, session_id, owner_id, role, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?)",
      )
      .bind(
        tokenHash,
        share.sessionId,
        share.ownerId,
        share.role,
        share.expiresAt,
        new Date().toISOString(),
      )
      .run();
  }

  async getShare(tokenHash: string): Promise<AccountShare | null> {
    await this.ensure();
    return this.db
      .prepare(
        "SELECT session_id AS sessionId, owner_id AS ownerId, role, expires_at AS expiresAt FROM shares WHERE token_hash = ?",
      )
      .bind(tokenHash)
      .first<AccountShare>();
  }

  async getUser(userId: string): Promise<AccountUser | null> {
    await this.ensure();
    return this.db
      .prepare("SELECT id, address, created_at AS createdAt FROM users WHERE id = ?")
      .bind(userId)
      .first<AccountUser>();
  }

  async getUserByAddress(address: string): Promise<AccountUser | null> {
    await this.ensure();
    const key = normalizeAddress(address);
    if (!key) return null;
    return this.db
      .prepare("SELECT id, address, created_at AS createdAt FROM users WHERE address = ?")
      .bind(key)
      .first<AccountUser>();
  }

  async createOrg(org: AccountOrg): Promise<AccountOrg> {
    await this.ensure();
    await this.db
      .prepare("INSERT OR IGNORE INTO orgs (id, name, created_at, created_by) VALUES (?, ?, ?, ?)")
      .bind(org.id, org.name, org.createdAt, org.createdBy)
      .run();
    return (await this.getOrg(org.id)) ?? org;
  }

  async getOrg(orgId: string): Promise<AccountOrg | null> {
    await this.ensure();
    return this.db
      .prepare(
        "SELECT id, name, created_at AS createdAt, created_by AS createdBy FROM orgs WHERE id = ?",
      )
      .bind(orgId)
      .first<AccountOrg>();
  }

  async listOrgsForUser(userId: string): Promise<AccountOrg[]> {
    await this.ensure();
    const { results } = await this.db
      .prepare(
        `SELECT o.id, o.name, o.created_at AS createdAt, o.created_by AS createdBy
         FROM orgs o JOIN org_members m ON m.org_id = o.id
         WHERE m.user_id = ?`,
      )
      .bind(userId)
      .all<AccountOrg>();
    return results ?? [];
  }

  async putMember(member: OrgMember): Promise<void> {
    await this.ensure();
    await this.db
      .prepare(
        "INSERT OR REPLACE INTO org_members (org_id, user_id, address, role, created_at) VALUES (?, ?, ?, ?, ?)",
      )
      .bind(member.orgId, member.userId, member.address, member.role, member.createdAt)
      .run();
  }

  async getMember(orgId: string, userId: string): Promise<OrgMember | null> {
    await this.ensure();
    return this.db
      .prepare(
        "SELECT org_id AS orgId, user_id AS userId, address, role, created_at AS createdAt FROM org_members WHERE org_id = ? AND user_id = ?",
      )
      .bind(orgId, userId)
      .first<OrgMember>();
  }

  async listMembers(orgId: string): Promise<OrgMember[]> {
    await this.ensure();
    const { results } = await this.db
      .prepare(
        "SELECT org_id AS orgId, user_id AS userId, address, role, created_at AS createdAt FROM org_members WHERE org_id = ?",
      )
      .bind(orgId)
      .all<OrgMember>();
    return results ?? [];
  }

  async deleteMember(orgId: string, userId: string): Promise<void> {
    await this.ensure();
    await this.db
      .prepare("DELETE FROM org_members WHERE org_id = ? AND user_id = ?")
      .bind(orgId, userId)
      .run();
  }

  async putRoomGrant(grant: RoomGrant): Promise<void> {
    await this.ensure();
    await this.db
      .prepare(
        "INSERT OR REPLACE INTO room_grants (session_id, org_id, owner_id, claimed_at) VALUES (?, ?, ?, ?)",
      )
      .bind(grant.sessionId, grant.orgId, grant.ownerId, grant.claimedAt)
      .run();
  }

  async getRoomGrant(sessionId: string): Promise<RoomGrant | null> {
    await this.ensure();
    return this.db
      .prepare(
        "SELECT session_id AS sessionId, org_id AS orgId, owner_id AS ownerId, claimed_at AS claimedAt FROM room_grants WHERE session_id = ?",
      )
      .bind(sessionId)
      .first<RoomGrant>();
  }

  async listRoomGrants(orgId: string): Promise<RoomGrant[]> {
    await this.ensure();
    const { results } = await this.db
      .prepare(
        "SELECT session_id AS sessionId, org_id AS orgId, owner_id AS ownerId, claimed_at AS claimedAt FROM room_grants WHERE org_id = ? ORDER BY claimed_at DESC",
      )
      .bind(orgId)
      .all<RoomGrant>();
    return results ?? [];
  }

  async putInvite(tokenHash: string, invite: OrgInvite): Promise<void> {
    await this.ensure();
    await this.db
      .prepare(
        "INSERT OR REPLACE INTO org_invites (token_hash, org_id, role, address, invited_by, expires_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
      )
      .bind(
        tokenHash,
        invite.orgId,
        invite.role,
        invite.address,
        invite.invitedBy,
        invite.expiresAt,
        invite.createdAt,
      )
      .run();
  }

  async getInvite(tokenHash: string): Promise<OrgInvite | null> {
    await this.ensure();
    const row = await this.db
      .prepare(
        "SELECT org_id AS orgId, role, address, invited_by AS invitedBy, expires_at AS expiresAt, created_at AS createdAt FROM org_invites WHERE token_hash = ?",
      )
      .bind(tokenHash)
      .first<OrgInvite>();
    return row ?? null;
  }

  async deleteInvite(tokenHash: string): Promise<void> {
    await this.ensure();
    await this.db.prepare("DELETE FROM org_invites WHERE token_hash = ?").bind(tokenHash).run();
  }

  async listInvitesForOrg(orgId: string): Promise<Array<OrgInvite & { tokenHash: string }>> {
    await this.ensure();
    const { results } = await this.db
      .prepare(
        "SELECT token_hash AS tokenHash, org_id AS orgId, role, address, invited_by AS invitedBy, expires_at AS expiresAt, created_at AS createdAt FROM org_invites WHERE org_id = ?",
      )
      .bind(orgId)
      .all<OrgInvite & { tokenHash: string }>();
    return results ?? [];
  }

  async listInvitesForAddress(
    address: string,
  ): Promise<Array<OrgInvite & { tokenHash: string }>> {
    await this.ensure();
    const key = normalizeAddress(address);
    if (!key) return [];
    const { results } = await this.db
      .prepare(
        "SELECT token_hash AS tokenHash, org_id AS orgId, role, address, invited_by AS invitedBy, expires_at AS expiresAt, created_at AS createdAt FROM org_invites WHERE address = ?",
      )
      .bind(key)
      .all<OrgInvite & { tokenHash: string }>();
    return results ?? [];
  }
}
