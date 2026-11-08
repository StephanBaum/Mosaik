import { describe, expect, it } from "vitest";
import type { MosaicIpc } from "../client.ts";
import type { CommandName, ErrorCode, RequestOf, ResponseOf } from "../contract/index.ts";
import { createMockIpc, type MockOptions } from "../mock.ts";
import { MOCK_PASSWORD } from "./data.ts";
import { KIM, LEE, SAM } from "./sharingData.ts";

const WORK = "019a2b3c-4d5e-7010-8000-000000000002";

async function value<K extends CommandName>(
  ipc: MosaicIpc,
  command: K,
  request: RequestOf<K>,
): Promise<ResponseOf<K>> {
  const result = await ipc.call(command, request);
  if (!result.ok) {
    throw new Error(`${command} failed with ${result.error.code}`);
  }
  return result.value;
}

async function code<K extends CommandName>(
  ipc: MosaicIpc,
  command: K,
  request: RequestOf<K>,
): Promise<ErrorCode | "ok"> {
  const result = await ipc.call(command, request);
  return result.ok ? "ok" : result.error.code;
}

function mock(sharing: MockOptions["sharing"] = "basic", sync: MockOptions["sync"] = { variant: "hosted" }) {
  return createMockIpc({ start: "unlocked", sharing, sync });
}

describe("mock sharing: contacts and verification", () => {
  it("pins a new contact unverified, then verifies it with the code as shown", async () => {
    const ipc = mock("empty");
    const found = await value(ipc, "sharing_lookup", { email: "Sam@Example.com" });
    expect(found).toMatchObject({ accountId: SAM.accountId, email: "sam@example.com", status: "pinned" });
    expect(await code(ipc, "sharing_verify_contact", { accountId: SAM.accountId, safetyCode: KIM.code })).toBe(
      "sharingKeyChanged",
    );
    // Spaces, case and the Crockford aliases do not matter.
    const loose = found.safetyCode.toLowerCase().replaceAll(" ", "-");
    expect(await code(ipc, "sharing_verify_contact", { accountId: SAM.accountId, safetyCode: loose })).toBe("ok");
    const again = await value(ipc, "sharing_lookup", { email: SAM.email });
    expect(again.status).toBe("verified");
  });

  it("gives an address nobody knows a key, too, and never says whether it has an account", async () => {
    const ipc = mock("empty");
    const a = await value(ipc, "sharing_lookup", { email: "nobody@example.com" });
    const b = await value(ipc, "sharing_lookup", { email: "nobody@example.com" });
    expect(a.safetyCode).toBe(b.safetyCode);
    expect(a.status).toBe("pinned");
    expect(b.status).toBe("pinned");
    expect(await code(ipc, "sharing_lookup", { email: "not an address" })).toBe("invalidInput");
  });

  it("needs a verified contact before an invite, and refuses the owner role", async () => {
    const ipc = mock("empty");
    const kim = await value(ipc, "sharing_lookup", { email: KIM.email });
    const request = { vaultId: WORK, accountId: kim.accountId, role: "read" } as const;
    expect(await code(ipc, "vault_share", request)).toBe("sharingContactUnverified");
    await value(ipc, "sharing_verify_contact", { accountId: kim.accountId, safetyCode: kim.safetyCode });
    expect(await code(ipc, "vault_share", { ...request, role: "owner" })).toBe("invalidInput");
    const sent = await value(ipc, "vault_share", request);
    expect(sent.expiresAt).toBeGreaterThan(Date.now());
    expect(await code(ipc, "vault_share", request)).toBe("sharingAlreadyMember");
    const members = await value(ipc, "vault_members", { vaultId: WORK });
    expect(members.members.find((m) => m.accountId === kim.accountId)?.state).toBe("invited");
  });

  it("blocks sharing after a key change until the new code was compared", async () => {
    const ipc = mock("keyChanged");
    const found = await value(ipc, "sharing_lookup", { email: SAM.email });
    expect(found.status).toBe("changed");
    expect(found.safetyCode).not.toBe(SAM.code);
    const request = { vaultId: WORK, accountId: SAM.accountId, role: "write" } as const;
    expect(await code(ipc, "vault_share", request)).toBe("sharingKeyChanged");
    expect(
      await code(ipc, "sharing_accept_key_change", { accountId: SAM.accountId, safetyCode: SAM.code }),
    ).toBe("sharingKeyChanged");
    await value(ipc, "sharing_accept_key_change", { accountId: SAM.accountId, safetyCode: found.safetyCode });
    // Accepting leaves the new key unverified.
    expect(await code(ipc, "vault_share", request)).toBe("sharingContactUnverified");
    await value(ipc, "sharing_verify_contact", { accountId: SAM.accountId, safetyCode: found.safetyCode });
    expect(await code(ipc, "vault_share", request)).toBe("sharingAlreadyMember");
  });

  it("says sharing needs the server with a folder or no sync", async () => {
    for (const sync of [{ variant: "folder" }, { variant: "none" }] as const) {
      const ipc = mock("basic", sync);
      expect(await code(ipc, "sharing_identity", null)).toBe("sharingNotHosted");
      expect(await code(ipc, "invites_list", null)).toBe("sharingNotHosted");
      expect(await code(ipc, "emergency_overview", null)).toBe("sharingNotHosted");
      expect(await code(ipc, "vault_members", { vaultId: WORK })).toBe("sharingNotHosted");
    }
  });
});

describe("mock sharing: members and roles", () => {
  it("lets the owner change a role and remove a member, and reports what a key change could not protect", async () => {
    const ipc = mock("rotation");
    const before = await value(ipc, "vault_members", { vaultId: WORK });
    expect(before.ownRole).toBe("owner");
    expect(before.pendingRotation).toBe(true);
    expect(before.unprotectedAttachments).toBe(2);
    expect(before.members.map((m) => m.state)).toContain("removing");
    await value(ipc, "vault_member_set_role", { vaultId: WORK, accountId: SAM.accountId, role: "read" });
    expect(await code(ipc, "vault_member_set_role", { vaultId: WORK, accountId: SAM.accountId, role: "owner" })).toBe(
      "invalidInput",
    );
    const removed = await value(ipc, "vault_member_remove", { vaultId: WORK, accountId: SAM.accountId });
    expect(removed.unreachableAttachments).toBe(2);
    const after = await value(ipc, "vault_members", { vaultId: WORK });
    expect(after.members.some((m) => m.accountId === SAM.accountId)).toBe(false);
  });

  it("lets a manager remove, but only the owner's device changes the key", async () => {
    const ipc = mock("manager");
    const view = await value(ipc, "vault_members", { vaultId: WORK });
    expect(view.ownRole).toBe("manage");
    expect(view.pendingRotation).toBe(false);
    const removed = await value(ipc, "vault_member_remove", { vaultId: WORK, accountId: KIM.accountId });
    expect(removed.unreachableAttachments).toBe(0);
    expect((await value(ipc, "vault_members", { vaultId: WORK })).pendingRotation).toBe(true);
    // The owner cannot be removed, and nobody removes themselves.
    expect(await code(ipc, "vault_member_remove", { vaultId: WORK, accountId: SAM.accountId })).toBe(
      "sharingRoleForbidden",
    );
  });

  it("keeps a viewer from inviting or removing, and the owner from leaving", async () => {
    const viewer = mock("viewer");
    expect(await code(viewer, "vault_member_remove", { vaultId: WORK, accountId: KIM.accountId })).toBe(
      "sharingRoleForbidden",
    );
    expect(await code(viewer, "vault_invite_cancel", { inviteId: "019a2b3c-4d5e-7060-8000-0000000000ff" })).toBe(
      "sharingInviteExpired",
    );
    await value(viewer, "vault_leave", { vaultId: WORK });
    expect(await code(viewer, "vault_members", { vaultId: WORK })).toBe("sharingRoleForbidden");
    const owner = mock("basic");
    expect(await code(owner, "vault_leave", { vaultId: WORK })).toBe("sharingOwnerCannotLeave");
  });

  it("withdraws a pending invite", async () => {
    const ipc = mock("basic");
    const list = await value(ipc, "invites_list", null);
    expect(list.outgoing).toHaveLength(1);
    await value(ipc, "vault_invite_cancel", { inviteId: list.outgoing[0]?.inviteId ?? "" });
    expect((await value(ipc, "invites_list", null)).outgoing).toHaveLength(0);
  });
});

describe("mock sharing: the invites inbox", () => {
  const SAMS = "019a2b3c-4d5e-7060-8000-000000000011";
  const KIMS = "019a2b3c-4d5e-7060-8000-000000000012";
  const LEES = "019a2b3c-4d5e-7060-8000-000000000013";

  it("opens an invite, requires the sender's code, and the owner's when a manager invites", async () => {
    const ipc = mock("invites");
    const list = await value(ipc, "invites_list", null);
    expect(list.incoming.map((i) => i.inviteId)).toEqual([SAMS, KIMS, LEES]);

    const preview = await value(ipc, "invite_preview", { inviteId: SAMS });
    expect(preview).toMatchObject({ vaultName: "Family", senderStatus: "pinned", ownerStatus: "pinned" });
    expect(await code(ipc, "invite_accept", { inviteId: SAMS, safetyCode: null })).toBe("sharingContactUnverified");
    expect(await code(ipc, "invite_accept", { inviteId: SAMS, safetyCode: KIM.code })).toBe("sharingKeyChanged");
    await value(ipc, "invite_accept", { inviteId: SAMS, safetyCode: SAM.code });
    const vaults = await value(ipc, "list_vaults", null);
    expect(vaults.some((v) => v.name === "Family")).toBe(true);

    // Kim is a manager inviting to Sam's vault, and Sam was verified just now.
    const second = await value(ipc, "invite_preview", { inviteId: KIMS });
    expect(second.senderStatus).toBe("verified");
    expect(second.ownerStatus).toBe("verified");
    await value(ipc, "invite_accept", { inviteId: KIMS, safetyCode: null });
  });

  it("makes the owner verified first when the sender is not the owner", async () => {
    const ipc = mock("invites");
    // Kim (verified in this variant) invites to a vault Sam owns, and Sam is not verified yet.
    const preview = await value(ipc, "invite_preview", { inviteId: KIMS });
    expect(preview.senderStatus).toBe("verified");
    expect(preview.ownerStatus).toBe("pinned");
    expect(await code(ipc, "invite_accept", { inviteId: KIMS, safetyCode: null })).toBe("sharingContactUnverified");
    await value(ipc, "sharing_verify_contact", { accountId: preview.ownerAccountId, safetyCode: preview.ownerCode });
    await value(ipc, "invite_accept", { inviteId: KIMS, safetyCode: null });
  });

  it("says an unreadable invite cannot be opened, and declines with or without a block", async () => {
    const ipc = mock("invites");
    expect(await code(ipc, "invite_preview", { inviteId: LEES })).toBe("sharingInviteUnreadable");
    await value(ipc, "invite_decline", { inviteId: LEES, block: true });
    await value(ipc, "invite_decline", { inviteId: SAMS, block: false });
    expect((await value(ipc, "invites_list", null)).incoming.map((i) => i.inviteId)).toEqual([KIMS]);
    expect(await code(ipc, "invite_preview", { inviteId: LEES })).toBe("sharingInviteExpired");
  });

  it("announces the counts that wait", async () => {
    const ipc = mock("invites");
    const events: unknown[] = [];
    await ipc.subscribe((event) => {
      events.push(event);
    });
    await new Promise((resolve) => setTimeout(resolve, 5));
    expect(events).toContainEqual({ type: "sharingChanged", invites: 3, emergencyRequests: 0 });
  });
});

describe("mock sharing: emergency access", () => {
  const grant = (accountId: string, extra: Partial<RequestOf<"emergency_grant">> = {}): RequestOf<"emergency_grant"> => ({
    accountId,
    scope: { kind: "all" },
    waitDays: 7,
    password: MOCK_PASSWORD,
    code: null,
    ...extra,
  });

  it("grants to a verified contact with the password and a listed wait", async () => {
    const ipc = mock("empty", { variant: "hostedNoTotp" });
    const lee = await value(ipc, "sharing_lookup", { email: LEE.email });
    expect(await code(ipc, "emergency_grant", grant(lee.accountId))).toBe("sharingContactUnverified");
    await value(ipc, "sharing_verify_contact", { accountId: lee.accountId, safetyCode: lee.safetyCode });
    expect(await code(ipc, "emergency_grant", grant(lee.accountId, { waitDays: 3 }))).toBe("invalidInput");
    expect(await code(ipc, "emergency_grant", grant(lee.accountId, { password: "wrong" }))).toBe(
      "wrongCredentials",
    );
    expect(
      await code(ipc, "emergency_grant", grant(lee.accountId, { scope: { kind: "vaults", vaultIds: [] } })),
    ).toBe("invalidInput");
    await value(ipc, "emergency_grant", grant(lee.accountId));
    const overview = await value(ipc, "emergency_overview", null);
    expect(overview.granted).toHaveLength(1);
    expect(overview.granted[0]).toMatchObject({ email: LEE.email, waitDays: 7, state: "idle" });
    await value(ipc, "emergency_revoke", { accountId: lee.accountId });
    expect((await value(ipc, "emergency_overview", null)).granted).toHaveLength(0);
  });

  it("asks the account's two-step code when the sync account has one", async () => {
    const ipc = mock("empty", { variant: "hosted" });
    const lee = await value(ipc, "sharing_lookup", { email: LEE.email });
    await value(ipc, "sharing_verify_contact", { accountId: lee.accountId, safetyCode: lee.safetyCode });
    expect(await code(ipc, "emergency_grant", grant(lee.accountId))).toBe("syncTwoFactorRequired");
    expect(await code(ipc, "emergency_grant", grant(lee.accountId, { code: "000000" }))).toBe(
      "syncTwoFactorInvalid",
    );
    expect(await code(ipc, "emergency_grant", grant(lee.accountId, { code: "123456" }))).toBe("ok");
  });

  it("lets the grantor refuse or approve a waiting request, approving with the password", async () => {
    const ipc = mock("request", { variant: "hostedNoTotp" });
    const waiting = (await value(ipc, "emergency_overview", null)).granted[0];
    expect(waiting?.state).toBe("waiting");
    expect(
      await code(ipc, "emergency_respond", {
        granteeAccountId: SAM.accountId,
        decision: "approve",
        password: null,
        code: null,
      }),
    ).toBe("emptyPassword");
    await value(ipc, "emergency_respond", {
      granteeAccountId: SAM.accountId,
      decision: "decline",
      password: null,
      code: null,
    });
    expect((await value(ipc, "emergency_overview", null)).granted[0]?.state).toBe("idle");
  });

  it("makes the grantee ask, wait and claim, with the grantor verified", async () => {
    const ipc = mock("emergency");
    const received = (await value(ipc, "emergency_overview", null)).received;
    // Lee has not been asked yet; the wait is two days.
    const requested = await value(ipc, "emergency_request", { grantorAccountId: LEE.accountId });
    expect(requested.releaseAt).toBeGreaterThan(Date.now());
    expect(received.find((r) => r.grantorAccountId === LEE.accountId)?.waitDays).toBe(2);
    expect(await code(ipc, "emergency_claim", { grantorAccountId: LEE.accountId })).toBe("sharingContactUnverified");
    const lee = await value(ipc, "sharing_lookup", { email: LEE.email });
    await value(ipc, "sharing_verify_contact", { accountId: lee.accountId, safetyCode: lee.safetyCode });
    expect(await code(ipc, "emergency_claim", { grantorAccountId: LEE.accountId })).toBe("emergencyWaiting");
    // Kim's grant was released: claiming adds a read-only vault.
    const kim = await value(ipc, "sharing_lookup", { email: KIM.email });
    await value(ipc, "sharing_verify_contact", { accountId: kim.accountId, safetyCode: kim.safetyCode });
    expect(await value(ipc, "emergency_claim", { grantorAccountId: KIM.accountId })).toEqual({ vaults: 1 });
    expect((await value(ipc, "list_vaults", null)).some((v) => v.name === "kim's vault")).toBe(true);
    expect(await code(ipc, "emergency_request", { grantorAccountId: LEE.accountId })).toBe("emergencyNotGranted");
  });

  it("includes the findings in the sync overview and clears them on acknowledge", async () => {
    const ipc = mock("findings");
    const status = await value(ipc, "sync_status", null);
    expect(status.sharingFindings.map((f) => f.kind)).toEqual([
      "identityMismatch",
      "rosterInvalid",
      "shareUntrusted",
      "memberUnverified",
      "memberDropped",
      "accessLost",
    ]);
    const cleared = await value(ipc, "sync_acknowledge", {});
    expect(cleared.sharingFindings).toEqual([]);
  });
});
