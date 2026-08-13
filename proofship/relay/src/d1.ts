import type { AccountShare, AccountStore, AccountSession, AccountUser } from "./accounts";
import { normalizeAddress } from "./siwe";

export interface D1Like {
  prepare(query: string): {
    bind(...values: unknown[]): {
      first<T = Record<string, unknown>>(): Promise<T | null>;
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
     created_at TEXT NOT NULL
   )`,
  `CREATE TABLE IF NOT EXISTS shares (
     token_hash TEXT PRIMARY KEY,
     session_id TEXT NOT NULL,
     owner_id TEXT NOT NULL,
     role TEXT NOT NULL,
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
        "INSERT OR REPLACE INTO sessions (token_hash, user_id, address, expires_at, created_at) VALUES (?, ?, ?, ?, ?)",
      )
      .bind(tokenHash, session.userId, session.address, session.expiresAt, new Date().toISOString())
      .run();
  }

  async getSession(tokenHash: string): Promise<AccountSession | null> {
    await this.ensure();
    return this.db
      .prepare(
        "SELECT user_id AS userId, address, expires_at AS expiresAt FROM sessions WHERE token_hash = ?",
      )
      .bind(tokenHash)
      .first<AccountSession>();
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
}
