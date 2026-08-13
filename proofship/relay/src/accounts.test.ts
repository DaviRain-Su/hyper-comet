import { describe, expect, it } from "vitest";
import {
  MemoryAccountStore,
  acceptPendingInvites,
  ensurePersonalOrg,
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

describe("orgs", () => {
  it("lists claimed rooms newest first", async () => {
    const store = new MemoryAccountStore();
    const user = await store.upsertUser(ADDRESS, "t0");
    const org = await ensurePersonalOrg(store, user, "t0");
    await store.putRoomGrant({
      sessionId: "desktop-old",
      orgId: org.id,
      ownerId: user.id,
      claimedAt: "2026-08-01T00:00:00.000Z",
    });
    await store.putRoomGrant({
      sessionId: "desktop-new",
      orgId: org.id,
      ownerId: user.id,
      claimedAt: "2026-08-13T00:00:00.000Z",
    });
    await store.putRoomGrant({
      sessionId: "other-org",
      orgId: "org:other",
      ownerId: user.id,
      claimedAt: "2026-08-14T00:00:00.000Z",
    });
    expect((await store.listRoomGrants(org.id)).map((r) => r.sessionId)).toEqual([
      "desktop-new",
      "desktop-old",
    ]);
  });

  it("creates a personal org and accepts comment/command shares", async () => {
    const store = new MemoryAccountStore();
    const user = await store.upsertUser(ADDRESS, "t0");
    const org = await ensurePersonalOrg(store, user, "t0");
    expect(org.name).toBe("Personal");
    expect(await store.getMember(org.id, user.id)).toMatchObject({ role: "owner" });
    const again = await ensurePersonalOrg(store, user, "t1");
    expect(again.id).toBe(org.id);

    const token = "cmd-share";
    await store.putShare(await hashToken(token), {
      sessionId: "s1",
      ownerId: user.id,
      role: "command",
      expiresAt: "2026-09-01T00:00:00.000Z",
    });
    const now = Date.parse("2026-08-13T00:00:00.000Z");
    expect((await resolveShare(store, token, "s1", now))?.role).toBe("command");
  });
});

describe("invites", () => {
  it("accepts an address-bound invite after the wallet exists", async () => {
    const store = new MemoryAccountStore();
    const owner = await store.upsertUser("0x4444444444444444444444444444444444444444", "t0");
    const org = await ensurePersonalOrg(store, owner, "t0");
    const hash = await hashToken("invite-raw");
    await store.putInvite(hash, {
      orgId: org.id,
      role: "member",
      address: ADDRESS,
      invitedBy: owner.id,
      expiresAt: "2026-09-01T00:00:00.000Z",
      createdAt: "t0",
    });
    const guest = await store.upsertUser(ADDRESS, "t1");
    const joined = await acceptPendingInvites(
      store,
      guest,
      "t1",
      Date.parse("2026-08-13T00:00:00.000Z"),
    );
    expect(joined.map((o) => o.id)).toContain(org.id);
    expect(await store.getMember(org.id, guest.id)).toMatchObject({ role: "member" });
    expect(await store.getInvite(hash)).toBeNull();
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
