/**
 * Org membership + share-role capabilities.
 *
 * Share roles: readonly (observe) / comment (transcript note) / command
 * (drive the executor). Org members of a claimed room get command.
 * SIWE never grants deploy keys — cmd.deploy still needs a UserExecutor.
 */

import type { AccountSession, AccountShare, AccountStore, OrgMemberRole, ShareRole } from "./accounts";
import { resolveSession, resolveShare } from "./accounts";

export type WriteCap = "none" | "comment" | "command";

export interface ViewerAccess {
  ok: boolean;
  writeCap: WriteCap;
  kind: "denied" | "open" | "session" | "org" | "share";
  role?: ShareRole;
  userId?: string;
  address?: string;
  orgId?: string;
}

export function parseShareRole(raw: unknown): ShareRole | null {
  if (raw === "readonly" || raw === "comment" || raw === "command") return raw;
  return null;
}

export function parseOrgMemberRole(raw: unknown): OrgMemberRole | null {
  if (raw === "owner" || raw === "admin" || raw === "member") return raw;
  return null;
}

export function writeCapFromShareRole(role: ShareRole): WriteCap {
  if (role === "command") return "command";
  if (role === "comment") return "comment";
  return "none";
}

export function canManageMembers(role: OrgMemberRole): boolean {
  return role === "owner" || role === "admin";
}

export function commandAllowedForCap(cap: WriteCap, commandType: string): boolean {
  if (cap === "command") return true;
  if (cap === "comment") return commandType === "cmd.comment";
  return false;
}

export async function resolveViewerAccess(
  store: AccountStore,
  token: string | null,
  sessionId: string,
  nowMs: number,
  fallbackOpen: boolean,
): Promise<ViewerAccess> {
  const share = await resolveShare(store, token, sessionId, nowMs);
  if (share) {
    return {
      ok: true,
      writeCap: writeCapFromShareRole(share.role),
      kind: "share",
      role: share.role,
    };
  }

  const session = await resolveSession(store, token, nowMs);
  if (session) {
    const grant = await store.getRoomGrant(sessionId);
    if (!grant) {
      return {
        ok: true,
        writeCap: "command",
        kind: "session",
        userId: session.userId,
        address: session.address,
        orgId: session.orgId,
      };
    }
    const member = await store.getMember(grant.orgId, session.userId);
    if (member) {
      return {
        ok: true,
        writeCap: "command",
        kind: "org",
        userId: session.userId,
        address: session.address,
        orgId: grant.orgId,
      };
    }
    return { ok: false, writeCap: "none", kind: "denied" };
  }

  if (fallbackOpen) {
    return { ok: true, writeCap: "command", kind: "open" };
  }
  return { ok: false, writeCap: "none", kind: "denied" };
}

export function accessFromSessionAndGrant(
  session: AccountSession,
  memberRole: OrgMemberRole | null,
  claimed: boolean,
): ViewerAccess {
  if (!claimed) {
    return {
      ok: true,
      writeCap: "command",
      kind: "session",
      userId: session.userId,
      address: session.address,
      orgId: session.orgId,
    };
  }
  if (!memberRole) {
    return { ok: false, writeCap: "none", kind: "denied" };
  }
  return {
    ok: true,
    writeCap: "command",
    kind: "org",
    userId: session.userId,
    address: session.address,
    orgId: session.orgId,
  };
}
