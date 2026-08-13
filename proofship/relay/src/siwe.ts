/**
 * EIP-4361 (SIWE) message build / parse / field checks.
 *
 * Signature recovery is injected so this file stays crypto-free and
 * unit-testable. A SIWE login identifies an account; it never grants a
 * deploy key.
 */

export const SIWE_VERSION = "1";
export const SIWE_STATEMENT =
  "Sign in to ProofShip. This identifies your account only — it does not grant deploy keys.";

export interface SiweFields {
  domain: string;
  address: string;
  statement: string;
  uri: string;
  version: string;
  chainId: number;
  nonce: string;
  issuedAt: string;
  expirationTime?: string;
}

export interface SiweBuildInput {
  domain: string;
  address: string;
  uri: string;
  nonce: string;
  chainId: number;
  issuedAt: string;
  expirationTime?: string;
  statement?: string;
}

const ADDRESS_RE = /^0x[0-9a-fA-F]{40}$/u;
const NONCE_RE = /^[a-zA-Z0-9]{8,64}$/u;

export function normalizeAddress(address: string): string | null {
  const trimmed = address.trim();
  if (!ADDRESS_RE.test(trimmed)) return null;
  return `0x${trimmed.slice(2).toLowerCase()}`;
}

export function buildSiweMessage(input: SiweBuildInput): string {
  const address = normalizeAddress(input.address);
  if (!address) throw new Error("invalid address");
  if (!input.domain.trim()) throw new Error("missing domain");
  if (!input.uri.trim()) throw new Error("missing uri");
  if (!NONCE_RE.test(input.nonce)) throw new Error("invalid nonce");
  if (!Number.isInteger(input.chainId) || input.chainId <= 0) {
    throw new Error("invalid chain id");
  }
  const statement = input.statement ?? SIWE_STATEMENT;
  const lines = [
    `${input.domain} wants you to sign in with your Ethereum account:`,
    address,
    "",
    statement,
    "",
    `URI: ${input.uri}`,
    `Version: ${SIWE_VERSION}`,
    `Chain ID: ${input.chainId}`,
    `Nonce: ${input.nonce}`,
    `Issued At: ${input.issuedAt}`,
  ];
  if (input.expirationTime) {
    lines.push(`Expiration Time: ${input.expirationTime}`);
  }
  return lines.join("\n");
}

export function parseSiweMessage(message: string): SiweFields | null {
  const lines = message.replace(/\r\n/gu, "\n").split("\n");
  if (lines.length < 10) return null;
  const header = /^(?<domain>[^\s]+) wants you to sign in with your Ethereum account:$/u.exec(
    lines[0] ?? "",
  );
  if (!header?.groups?.domain) return null;
  const address = normalizeAddress(lines[1] ?? "");
  if (!address) return null;
  if (lines[2] !== "") return null;
  const statement = lines[3] ?? "";
  if (lines[4] !== "") return null;
  const fields = new Map<string, string>();
  for (const line of lines.slice(5)) {
    const idx = line.indexOf(": ");
    if (idx <= 0) return null;
    fields.set(line.slice(0, idx), line.slice(idx + 2));
  }
  const uri = fields.get("URI");
  const version = fields.get("Version");
  const chainRaw = fields.get("Chain ID");
  const nonce = fields.get("Nonce");
  const issuedAt = fields.get("Issued At");
  if (!uri || !version || !chainRaw || !nonce || !issuedAt) return null;
  const chainId = Number(chainRaw);
  if (!Number.isInteger(chainId) || chainId <= 0) return null;
  if (!NONCE_RE.test(nonce)) return null;
  return {
    domain: header.groups.domain,
    address,
    statement,
    uri,
    version,
    chainId,
    nonce,
    issuedAt,
    expirationTime: fields.get("Expiration Time"),
  };
}

export interface SiweCheckInput {
  nowMs: number;
  expectedDomain: string;
  expectedAddress: string;
  expectedNonce: string;
  allowedChainIds?: number[];
}

export function checkSiweFields(
  fields: SiweFields,
  check: SiweCheckInput,
): string | null {
  if (fields.version !== SIWE_VERSION) return "unsupported SIWE version";
  if (fields.domain !== check.expectedDomain) return "SIWE domain mismatch";
  const address = normalizeAddress(check.expectedAddress);
  if (!address || fields.address !== address) return "SIWE address mismatch";
  if (fields.nonce !== check.expectedNonce) return "SIWE nonce mismatch";
  if (check.allowedChainIds && !check.allowedChainIds.includes(fields.chainId)) {
    return "SIWE chain id not allowed";
  }
  const issued = Date.parse(fields.issuedAt);
  if (Number.isNaN(issued)) return "invalid issued-at";
  if (issued - 2 * 60 * 1000 > check.nowMs) return "SIWE issued in the future";
  if (fields.expirationTime) {
    const exp = Date.parse(fields.expirationTime);
    if (Number.isNaN(exp)) return "invalid expiration";
    if (exp <= check.nowMs) return "SIWE message expired";
  }
  return null;
}
