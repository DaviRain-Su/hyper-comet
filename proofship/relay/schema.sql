-- ProofShip relay accounts (Phase 4.2). Optional D1 binding `DB`.
-- Apply: wrangler d1 execute proofship-accounts --local --file=schema.sql
-- Wallet address is the account. Session / share tokens are stored hashed.

CREATE TABLE IF NOT EXISTS users (
  id TEXT PRIMARY KEY,
  address TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS nonces (
  address TEXT PRIMARY KEY,
  nonce TEXT NOT NULL,
  expires_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
  token_hash TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  address TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS shares (
  token_hash TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  role TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  created_at TEXT NOT NULL
);
