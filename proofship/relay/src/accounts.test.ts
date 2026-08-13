import { describe, expect, it } from "vitest";
import {
  MemoryAccountStore,
  hashToken,
  resolveSession,
  resolveShare,
  sessionStillValid,
  shareStillValid,
} from "./accounts";

const ADDRESS = "0x2222222222222222222222222222222222222222";

describe("MemoryAccountStore", () => {
  it("consumes a nonce once", async () => {
    const store = new MemoryAccountStore();
    await store.putNonce(ADDRESS, "abc12345", "2026-08-13T00:10:00.000Z");
    const first = await store.takeNonce(ADDRESS);
    expect(first?.nonce).toBe("abc12345");
    expect(await store.takeNonce(ADDRESS)).toBeNull();
  });

  it("upserts a user by checksum-insensitive address", async () => {
    const store = new MemoryAccountStore();
    const a = await store.upsertUser(ADDRESS.toUpperCase().replace("0X", "0x"), "t0");
    const b = await store.upsertUser(ADDRESS, "t1");
    expect(a.id).toBe(b.id);
    expect(a.address).toBe(ADDRESS);
    expect(b.createdAt).toBe("t0");
  });

  it("resolves hashed sessions and shares", async () => {
    const store = new MemoryAccountStore();
    const user = await store.upsertUser(ADDRESS, "t0");
    const sessionToken = "session-raw-token";
    await store.putSession(await hashToken(sessionToken), {
      userId: user.id,
      address: user.address,
      expiresAt: "2026-08-20T00:00:00.000Z",
    });
    const now = Date.parse("2026-08-13T00:00:00.000Z");
    const session = await resolveSession(store, sessionToken, now);
    expect(session?.address).toBe(ADDRESS);

    const shareToken = "share-raw-token";
    await store.putShare(await hashToken(shareToken), {
      sessionId: "sess-1",
      ownerId: user.id,
      role: "readonly",
      expiresAt: "2026-09-01T00:00:00.000Z",
    });
    expect(await resolveShare(store, shareToken, "sess-1", now)).not.toBeNull();
    expect(await resolveShare(store, shareToken, "other", now)).toBeNull();
  });
});

describe("expiry helpers", () => {
  it("treats past timestamps as invalid", () => {
    expect(
      sessionStillValid(
        { userId: "u", address: ADDRESS, expiresAt: "2026-08-01T00:00:00.000Z" },
        Date.parse("2026-08-13T00:00:00.000Z"),
      ),
    ).toBe(false);
    expect(
      shareStillValid(
        {
          sessionId: "s",
          ownerId: "u",
          role: "readonly",
          expiresAt: "2026-09-01T00:00:00.000Z",
        },
        Date.parse("2026-08-13T00:00:00.000Z"),
      ),
    ).toBe(true);
  });
});
