/**
 * Mock sharing commands (M6b): the same rules as `src-tauri/src/commands/sharing.rs` where the
 * UI can see them. Verification is required before an invite or a grant, a key change blocks
 * sharing until the new code was compared, and the owner of a vault joined through a manager
 * has to be verified, too. The real backend asks for removing and leaving in a native
 * dialog; the mock answers yes. Contacts live in `sharingContacts.ts`, emergency access in
 * `sharingEmergency.ts`.
 */
import type { ContactView, ShareRole } from "../contract/index.ts";
import { sameCode } from "../safetyCode.ts";
import { sampleAccount } from "./data.ts";
import { contactHandlers } from "./sharingContacts.ts";
import { publishSharing, type MockSharing } from "./sharingData.ts";
import { emergencyHandlers } from "./sharingEmergency.ts";
import { contactOf, hosted, membersOf, ownRoleOf, requireManager, vaultKnown } from "./sharingShared.ts";
import { fail, type MockHandlers, type MockState } from "./state.ts";

const DAY = 86_400_000;

function newInviteId(sharing: MockSharing): string {
  sharing.inviteCounter += 1;
  return `019a2b3c-4d5e-7060-8000-${sharing.inviteCounter.toString(16).padStart(12, "0")}`;
}

function removeIncoming(sharing: MockSharing, inviteId: string): void {
  sharing.incoming = sharing.incoming.filter((i) => i.inviteId !== inviteId);
}

/** The contact an invite goes to: verified, and still on the key that was verified. */
function shareableContact(sharing: MockSharing, accountId: string): ContactView {
  const contact = contactOf(sharing, accountId) ?? fail("sharingContactUnverified");
  const served = sharing.directory.get(contact.email)?.code ?? contact.safetyCode;
  if (contact.changed || !sameCode(contact.safetyCode, served)) {
    fail("sharingKeyChanged");
  }
  if (!contact.verified) {
    fail("sharingContactUnverified");
  }
  return contact;
}

/** The contact in the book for `accountId`, pinned (unverified) if it is new. */
function pinned(sharing: MockSharing, accountId: string, email: string, code: string): ContactView {
  let contact = contactOf(sharing, accountId);
  if (contact === undefined) {
    contact = { accountId, email, safetyCode: code, verified: false, changed: false };
    sharing.contacts.push(contact);
  }
  return contact;
}

function inviteMember(state: MockState, vaultId: string, accountId: string, role: ShareRole) {
  const sharing = hosted(state);
  if (role === "owner") {
    fail("invalidInput");
  }
  vaultKnown(state, vaultId);
  const contact = shareableContact(sharing, accountId);
  requireManager(sharing, vaultId);
  const rows = membersOf(sharing, vaultId);
  if (rows.some((m) => m.accountId === accountId && m.state !== "dropped")) {
    // The roster lists an invitee already, so a second invite is a member's, as on the server.
    fail("sharingAlreadyMember");
  }
  const inviteId = newInviteId(sharing);
  const expiresAt = state.now() + 7 * DAY;
  sharing.outgoing.push({
    inviteId,
    vaultId,
    vaultName: state.vaults.find((v) => v.vaultId === vaultId)?.name ?? null,
    recipientEmail: contact.email,
    role,
    expiresAt,
  });
  rows.push({
    accountId: contact.accountId,
    email: contact.email,
    role,
    verified: true,
    self: false,
    state: "invited",
    origin: null,
  });
  return { inviteId, expiresAt };
}

function removeMember(state: MockState, vaultId: string, accountId: string) {
  const sharing = hosted(state);
  const own = requireManager(sharing, vaultId);
  const rows = membersOf(sharing, vaultId);
  const target = rows.find((m) => m.accountId === accountId) ?? fail("invalidInput");
  if (target.role === "owner" || accountId === sampleAccount.accountId) {
    fail("sharingRoleForbidden");
  }
  sharing.members.set(
    vaultId,
    rows.filter((m) => m.accountId !== accountId),
  );
  if (own === "manage") {
    // The owner's device changes the key at its next sync.
    sharing.pendingRotation.add(vaultId);
    return { unreachableAttachments: 0 };
  }
  sharing.unprotected = sharing.removalUnreachable;
  return { unreachableAttachments: sharing.removalUnreachable };
}

/** Joins the vault of an invite: the sender and the owner have to be verified. */
function acceptInvite(state: MockState, inviteId: string, code: string | null): null {
  const sharing = hosted(state);
  const invite = sharing.incoming.find((i) => i.inviteId === inviteId) ?? fail("sharingInviteExpired");
  const preview = sharing.previews.get(inviteId) ?? fail("sharingInviteStale");
  const sender = pinned(sharing, preview.senderAccountId, preview.senderEmail, preview.safetyCode);
  if (code !== null) {
    if (!sameCode(sender.safetyCode, code)) {
      fail("sharingKeyChanged");
    }
    sender.verified = true;
  }
  const owner = pinned(sharing, preview.ownerAccountId, preview.ownerEmail, preview.ownerCode);
  if (!sender.verified || !owner.verified) {
    fail("sharingContactUnverified");
  }
  removeIncoming(sharing, invite.inviteId);
  state.vaults.push({
    vaultId: preview.vaultId,
    name: preview.vaultName,
    itemCount: 0,
    available: true,
    protection: "accountKey",
    openUntil: null,
  });
  sharing.roles.set(preview.vaultId, invite.role);
  const me = {
    accountId: sampleAccount.accountId,
    email: sampleAccount.email,
    verified: true,
    self: false,
    state: "member",
  } as const;
  sharing.members.set(preview.vaultId, [
    { ...me, accountId: preview.ownerAccountId, email: preview.ownerEmail, role: "owner", origin: "owner" },
    { ...me, self: true, role: invite.role, origin: "invite" },
  ]);
  state.emit({ type: "syncDataChanged", vaultIds: [preview.vaultId] });
  publishSharing(state);
  return null;
}

export const sharingHandlers: MockHandlers = {
  ...contactHandlers,
  ...emergencyHandlers,

  vault_share: (state, request) => inviteMember(state, request.vaultId, request.accountId, request.role),

  vault_members: (state, request) => {
    const sharing = hosted(state);
    vaultKnown(state, request.vaultId);
    if (sharing.left.has(request.vaultId)) {
      fail("sharingRoleForbidden");
    }
    return {
      ownRole: ownRoleOf(sharing, request.vaultId),
      members: membersOf(sharing, request.vaultId),
      pendingRotation: sharing.pendingRotation.has(request.vaultId),
      unprotectedAttachments: sharing.unprotected,
    };
  },

  vault_member_set_role: (state, request) => {
    const sharing = hosted(state);
    if (request.role === "owner") {
      fail("invalidInput");
    }
    requireManager(sharing, request.vaultId);
    const member = membersOf(sharing, request.vaultId).find((m) => m.accountId === request.accountId);
    if (member === undefined || member.role === "owner") {
      fail("sharingRoleForbidden");
    }
    member.role = request.role;
    return null;
  },

  vault_member_remove: (state, request) => removeMember(state, request.vaultId, request.accountId),

  vault_invite_cancel: (state, request) => {
    const sharing = hosted(state);
    const invite = sharing.outgoing.find((i) => i.inviteId === request.inviteId) ?? fail("sharingInviteExpired");
    sharing.outgoing = sharing.outgoing.filter((i) => i.inviteId !== request.inviteId);
    const rows = sharing.members.get(invite.vaultId);
    if (rows !== undefined) {
      sharing.members.set(
        invite.vaultId,
        rows.filter((m) => !(m.state === "invited" && m.email === invite.recipientEmail)),
      );
    }
    return null;
  },

  vault_leave: (state, request) => {
    const sharing = hosted(state);
    vaultKnown(state, request.vaultId);
    if (ownRoleOf(sharing, request.vaultId) === "owner") {
      fail("sharingOwnerCannotLeave");
    }
    sharing.left.add(request.vaultId);
    return null;
  },

  invites_list: (state) => {
    const sharing = hosted(state);
    return { incoming: sharing.incoming, outgoing: sharing.outgoing };
  },

  invite_preview: (state, request) => {
    const sharing = hosted(state);
    if (!sharing.incoming.some((i) => i.inviteId === request.inviteId)) {
      fail("sharingInviteExpired");
    }
    if (sharing.unreadable.has(request.inviteId)) {
      fail("sharingInviteUnreadable");
    }
    const preview = sharing.previews.get(request.inviteId) ?? fail("sharingInviteExpired");
    const sender = contactOf(sharing, preview.senderAccountId);
    const owner = contactOf(sharing, preview.ownerAccountId);
    return {
      vaultName: preview.vaultName,
      role: preview.role,
      senderAccountId: preview.senderAccountId,
      senderEmail: preview.senderEmail,
      safetyCode: preview.safetyCode,
      ownerAccountId: preview.ownerAccountId,
      ownerEmail: preview.ownerEmail,
      ownerCode: preview.ownerCode,
      senderStatus: sender === undefined ? preview.senderStatus : sender.verified ? "verified" : "pinned",
      ownerStatus: owner === undefined ? preview.ownerStatus : owner.verified ? "verified" : "pinned",
    };
  },

  invite_accept: (state, request) => acceptInvite(state, request.inviteId, request.safetyCode),

  invite_decline: (state, request) => {
    const sharing = hosted(state);
    if (!sharing.incoming.some((i) => i.inviteId === request.inviteId)) {
      fail("sharingInviteExpired");
    }
    removeIncoming(sharing, request.inviteId);
    publishSharing(state);
    return null;
  },
};
