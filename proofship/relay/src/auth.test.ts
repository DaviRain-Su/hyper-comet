import { describe, expect, it, beforeEach } from "vitest";
import { privateKeyToAccount } from "viem/accounts";
import { MemoryAccountStore } from "./accounts";
import {
  handleAuth,
  resetMemoryAccountStore,
  shareAllowed,
  viewerAllowed,
} from "./auth";

const ACCOUNT = privateKeyToAccount(
  "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
);

function originHeaders(): HeadersInit {
  return { origin: "http://localhost:4173" };
}

describe("SIWE auth routes", () => {
  beforeEach(() => {
    resetMemoryAccountStore();
  });

  it("issues a nonce message and verifies a real wallet signature", async () => {
    const store = new MemoryAccountStore();
    const nonceRes = await handleAuth(
      new Request(
        `http://relay.test/api/auth/siwe/nonce?address=${ACCOUNT.address}&chainId=1952`,
        { headers: originHeaders() },
      ),
      {},
      store,
    );
    expect(nonceRes?.status).toBe(200);
    const nonceBody = (await nonceRes!.json()) as {
      ok: boolean;
      message: string;
      address: string;
    };
    expect(nonceBody.ok).toBe(true);
    expect(nonceBody.message).toContain(nonceBody.address);

    const signature = await ACCOUNT.signMessage({ message: nonceBody.message });
    const verifyRes = await handleAuth(
      new Request("http://relay.test/api/auth/siwe/verify", {
        method: "POST",
        headers: { ...originHeaders(), "content-type": "application/json" },
        body: JSON.stringify({ message: nonceBody.message, signature }),
      }),
      {},
      store,
    );
    expect(verifyRes?.status).toBe(200);
    const session = (await verifyRes!.json()) as {
      ok: boolean;
      token: string;
      address: string;
    };
    expect(session.ok).toBe(true);
    expect(session.address).toBe(ACCOUNT.address.toLowerCase());
    expect(session.token.length).toBeGreaterThan(16);

    const me = await handleAuth(
      new Request("http://relay.test/api/auth/me", {
        headers: { authorization: `Bearer ${session.token}` },
      }),
      {},
      store,
    );
    expect(me?.status).toBe(200);
    const meBody = (await me!.json()) as { address: string };
    expect(meBody.address).toBe(session.address);
  });

  it("rejects a reused nonce", async () => {
    const store = new MemoryAccountStore();
    const nonceRes = await handleAuth(
      new Request(
        `http://relay.test/api/auth/siwe/nonce?address=${ACCOUNT.address}`,
        { headers: originHeaders() },
      ),
      {},
      store,
    );
    const { message } = (await nonceRes!.json()) as { message: string };
    const signature = await ACCOUNT.signMessage({ message });
    const body = JSON.stringify({ message, signature });
    const first = await handleAuth(
      new Request("http://relay.test/api/auth/siwe/verify", {
        method: "POST",
        headers: { ...originHeaders(), "content-type": "application/json" },
        body,
      }),
      {},
      store,
    );
    expect(first?.status).toBe(200);
    const second = await handleAuth(
      new Request("http://relay.test/api/auth/siwe/verify", {
        method: "POST",
        headers: { ...originHeaders(), "content-type": "application/json" },
        body,
      }),
      {},
      store,
    );
    expect(second?.status).toBe(401);
  });

  it("mints a readonly share token for a signed-in owner", async () => {
    const store = new MemoryAccountStore();
    const nonceRes = await handleAuth(
      new Request(
        `http://relay.test/api/auth/siwe/nonce?address=${ACCOUNT.address}`,
        { headers: originHeaders() },
      ),
      {},
      store,
    );
    const { message } = (await nonceRes!.json()) as { message: string };
    const signature = await ACCOUNT.signMessage({ message });
    const verifyRes = await handleAuth(
      new Request("http://relay.test/api/auth/siwe/verify", {
        method: "POST",
        headers: { ...originHeaders(), "content-type": "application/json" },
        body: JSON.stringify({ message, signature }),
      }),
      {},
      store,
    );
    const { token } = (await verifyRes!.json()) as { token: string };
    const mint = await handleAuth(
      new Request("http://relay.test/api/sessions/demo-1/share", {
        method: "POST",
        headers: { authorization: `Bearer ${token}` },
      }),
      {},
      store,
    );
    expect(mint?.status).toBe(200);
    const share = (await mint!.json()) as { token: string; role: string };
    expect(share.role).toBe("readonly");

    const commandMint = await handleAuth(
      new Request("http://relay.test/api/sessions/demo-1/share", {
        method: "POST",
        headers: {
          authorization: `Bearer ${token}`,
          "content-type": "application/json",
        },
        body: JSON.stringify({ role: "command" }),
      }),
      {},
      store,
    );
    const commandShare = (await commandMint!.json()) as { role: string };
    expect(commandShare.role).toBe("command");

    const orgs = await handleAuth(
      new Request("http://relay.test/api/orgs", {
        headers: { authorization: `Bearer ${token}` },
      }),
      {},
      store,
    );
    const orgBody = (await orgs!.json()) as { orgs: { name: string }[] };
    expect(orgBody.orgs.some((o) => o.name === "Personal")).toBe(true);

    const url = new URL("http://relay.test/api/share/demo-1?token=" + share.token);
    expect(
      await shareAllowed(new Request(url), url, store, "demo-1", Date.now(), () => false),
    ).toBe(true);
    expect(
      await shareAllowed(new Request(url), url, store, "other", Date.now(), () => false),
    ).toBe(false);

    const viewerUrl = new URL("http://relay.test/ws/web/demo-1");
    expect(
      await viewerAllowed(
        new Request("http://relay.test/ws/web/demo-1", {
          headers: { authorization: `Bearer ${token}` },
        }),
        viewerUrl,
        store,
        "demo-1",
        Date.now(),
        () => false,
      ),
    ).toBe(true);
  });

  it("mints an invite for a wallet that has not signed in", async () => {
    const store = new MemoryAccountStore();
    const nonceRes = await handleAuth(
      new Request(
        `http://relay.test/api/auth/siwe/nonce?address=${ACCOUNT.address}`,
        { headers: originHeaders() },
      ),
      {},
      store,
    );
    const { message } = (await nonceRes!.json()) as { message: string };
    const signature = await ACCOUNT.signMessage({ message });
    const verifyRes = await handleAuth(
      new Request("http://relay.test/api/auth/siwe/verify", {
        method: "POST",
        headers: { ...originHeaders(), "content-type": "application/json" },
        body: JSON.stringify({ message, signature }),
      }),
      {},
      store,
    );
    const { token } = (await verifyRes!.json()) as { token: string };
    const orgs = await handleAuth(
      new Request("http://relay.test/api/orgs", {
        headers: { authorization: `Bearer ${token}` },
      }),
      {},
      store,
    );
    const orgId = ((await orgs!.json()) as { orgs: { id: string }[] }).orgs[0]?.id;
    const inviteRes = await handleAuth(
      new Request(`http://relay.test/api/orgs/${orgId}/invites`, {
        method: "POST",
        headers: {
          authorization: `Bearer ${token}`,
          "content-type": "application/json",
        },
        body: JSON.stringify({
          address: "0x5555555555555555555555555555555555555555",
          role: "member",
        }),
      }),
      {},
      store,
    );
    expect(inviteRes?.status).toBe(200);
    const inviteBody = (await inviteRes!.json()) as { token: string; invite: { address: string } };
    expect(inviteBody.token.length).toBeGreaterThan(8);
    expect(inviteBody.invite.address).toBe("0x5555555555555555555555555555555555555555");

    const peek = await handleAuth(
      new Request(`http://relay.test/api/invites/${inviteBody.token}`),
      {},
      store,
    );
    expect(peek?.status).toBe(200);
    const peekBody = (await peek!.json()) as { orgName: string };
    expect(peekBody.orgName).toBe("Personal");

    const listed = await handleAuth(
      new Request(`http://relay.test/api/orgs/${orgId}/invites`, {
        headers: { authorization: `Bearer ${token}` },
      }),
      {},
      store,
    );
    const listedBody = (await listed!.json()) as {
      invites: { tokenHash: string; address: string }[];
    };
    expect(listedBody.invites).toHaveLength(1);
    const hash = listedBody.invites[0]?.tokenHash;
    expect(hash).toBeTruthy();

    const revoked = await handleAuth(
      new Request(`http://relay.test/api/orgs/${orgId}/invites/${hash}`, {
        method: "DELETE",
        headers: { authorization: `Bearer ${token}` },
      }),
      {},
      store,
    );
    expect(revoked?.status).toBe(200);
    const peekGone = await handleAuth(
      new Request(`http://relay.test/api/invites/${inviteBody.token}`),
      {},
      store,
    );
    expect(peekGone?.status).toBe(404);
  });

  it("lists claimed rooms for an org member", async () => {
    const store = new MemoryAccountStore();
    const nonceRes = await handleAuth(
      new Request(
        `http://relay.test/api/auth/siwe/nonce?address=${ACCOUNT.address}`,
        { headers: originHeaders() },
      ),
      {},
      store,
    );
    const { message } = (await nonceRes!.json()) as { message: string };
    const signature = await ACCOUNT.signMessage({ message });
    const verifyRes = await handleAuth(
      new Request("http://relay.test/api/auth/siwe/verify", {
        method: "POST",
        headers: { ...originHeaders(), "content-type": "application/json" },
        body: JSON.stringify({ message, signature }),
      }),
      {},
      store,
    );
    const { token } = (await verifyRes!.json()) as { token: string };
    const orgs = await handleAuth(
      new Request("http://relay.test/api/orgs", {
        headers: { authorization: `Bearer ${token}` },
      }),
      {},
      store,
    );
    const orgId = ((await orgs!.json()) as { orgs: { id: string }[] }).orgs[0]?.id;
    const claim = await handleAuth(
      new Request("http://relay.test/api/sessions/desktop-abc/claim", {
        method: "POST",
        headers: { authorization: `Bearer ${token}` },
      }),
      {},
      store,
    );
    expect(claim?.status).toBe(200);
    const rooms = await handleAuth(
      new Request(`http://relay.test/api/orgs/${orgId}/rooms`, {
        headers: { authorization: `Bearer ${token}` },
      }),
      {},
      store,
    );
    expect(rooms?.status).toBe(200);
    const body = (await rooms!.json()) as { rooms: { sessionId: string }[] };
    expect(body.rooms.map((r) => r.sessionId)).toEqual(["desktop-abc"]);
  });
});
