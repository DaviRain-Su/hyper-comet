import { describe, expect, it } from "vitest";
import { MemoryAccountStore, hashToken } from "./accounts";
import {
  canManageMembers,
  commandAllowedForCap,
  parseShareRole,
  resolveViewerAccess,
  writeCapFromShareRole,
} from "./policy";

describe("share roles", () => {
  it("parses and maps write caps", () => {
    expect(parseShareRole("comment")).toBe("comment");
    expect(parseShareRole("admin")).toBeNull();
    expect(writeCapFromShareRole("readonly")).toBe("none");
    expect(writeCapFromShareRole("comment")).toBe("comment");
    expect(writeCapFromShareRole("command")).toBe("command");
    expect(commandAllowedForCap("comment", "cmd.comment")).toBe(true);
    expect(commandAllowedForCap("comment", "cmd.prompt")).toBe(false);
    expect(commandAllowedForCap("none", "cmd.comment")).toBe(false);
    expect(canManageMembers("owner")).toBe(true);
    expect(canManageMembers("member")).toBe(false);
  });
});

describe("resolveViewerAccess", () => {
  it("gives command to org members and the share role to token holders", async () => {
    const store = new MemoryAccountStore();
    const user = await store.upsertUser("0x3333333333333333333333333333333333333333", "t0");
    await store.createOrg({
      id: "org-a",
      name: "A",
      createdAt: "t0",
      createdBy: user.id,
    });
    await store.putMember({
      orgId: "org-a",
      userId: user.id,
      address: user.address,
      role: "owner",
      createdAt: "t0",
    });
    const token = "sess";
    await store.putSession(await hashToken(token), {
      userId: user.id,
      address: user.address,
      expiresAt: "2026-09-01T00:00:00.000Z",
      orgId: "org-a",
    });
    await store.putRoomGrant({
      sessionId: "room-1",
      orgId: "org-a",
      ownerId: user.id,
      claimedAt: "t0",
    });
    const now = Date.parse("2026-08-13T00:00:00.000Z");
    const orgAccess = await resolveViewerAccess(store, token, "room-1", now, false);
    expect(orgAccess).toMatchObject({ ok: true, kind: "org", writeCap: "command" });

    const shareTok = "share-comment";
    await store.putShare(await hashToken(shareTok), {
      sessionId: "room-1",
      ownerId: user.id,
      role: "comment",
      expiresAt: "2026-09-01T00:00:00.000Z",
    });
    const shareAccess = await resolveViewerAccess(store, shareTok, "room-1", now, false);
    expect(shareAccess).toMatchObject({ ok: true, kind: "share", writeCap: "comment", role: "comment" });
  });
});
