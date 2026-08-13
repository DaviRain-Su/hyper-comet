import { describe, expect, it } from "vitest";
import {
  SIWE_STATEMENT,
  buildSiweMessage,
  checkSiweFields,
  normalizeAddress,
  parseSiweMessage,
} from "./siwe";

const ADDRESS = "0x1111111111111111111111111111111111111111";

describe("normalizeAddress", () => {
  it("lowercases a valid address", () => {
    expect(normalizeAddress("0xAbCDEF0000000000000000000000000000000000")).toBe(
      "0xabcdef0000000000000000000000000000000000",
    );
  });

  it("rejects junk", () => {
    expect(normalizeAddress("not-an-address")).toBeNull();
    expect(normalizeAddress("0x123")).toBeNull();
  });
});

describe("SIWE build/parse", () => {
  it("round-trips a ProofShip login message", () => {
    const message = buildSiweMessage({
      domain: "proofship.example",
      address: ADDRESS,
      uri: "https://proofship.example/",
      nonce: "n0nceVal1",
      chainId: 1952,
      issuedAt: "2026-08-13T00:00:00.000Z",
      expirationTime: "2026-08-13T00:10:00.000Z",
    });
    expect(message).toContain(SIWE_STATEMENT);
    expect(message).toContain("Chain ID: 1952");
    const parsed = parseSiweMessage(message);
    expect(parsed).toEqual({
      domain: "proofship.example",
      address: ADDRESS,
      statement: SIWE_STATEMENT,
      uri: "https://proofship.example/",
      version: "1",
      chainId: 1952,
      nonce: "n0nceVal1",
      issuedAt: "2026-08-13T00:00:00.000Z",
      expirationTime: "2026-08-13T00:10:00.000Z",
    });
  });

  it("rejects a truncated message", () => {
    expect(parseSiweMessage("hello")).toBeNull();
  });
});

describe("checkSiweFields", () => {
  const fields = parseSiweMessage(
    buildSiweMessage({
      domain: "localhost:4173",
      address: ADDRESS,
      uri: "http://localhost:4173/",
      nonce: "abc12345",
      chainId: 1952,
      issuedAt: "2026-08-13T00:00:00.000Z",
      expirationTime: "2026-08-13T00:10:00.000Z",
    }),
  );
  if (!fields) throw new Error("expected parse");

  const base = {
    nowMs: Date.parse("2026-08-13T00:05:00.000Z"),
    expectedDomain: "localhost:4173",
    expectedAddress: ADDRESS,
    expectedNonce: "abc12345",
  };

  it("accepts a matching unexpired message", () => {
    expect(checkSiweFields(fields, base)).toBeNull();
  });

  it("rejects domain / nonce / expiry mismatches", () => {
    expect(checkSiweFields(fields, { ...base, expectedDomain: "evil.test" })).toBe(
      "SIWE domain mismatch",
    );
    expect(checkSiweFields(fields, { ...base, expectedNonce: "othernonce" })).toBe(
      "SIWE nonce mismatch",
    );
    expect(
      checkSiweFields(fields, { ...base, nowMs: Date.parse("2026-08-13T00:11:00.000Z") }),
    ).toBe("SIWE message expired");
  });
});
