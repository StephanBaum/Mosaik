/** Helpers the mock sharing handlers share (contacts, members, the master password). */
import type { ContactStatus, ContactView, MemberRow, ShareRole } from "../contract/index.ts";
import { sameCode } from "../safetyCode.ts";
import { sampleAccount } from "./data.ts";
import { hostedBackend, type MockSharing } from "./sharingData.ts";
import { fail, requireUnlocked, type MockState } from "./state.ts";

/** The sharing state, for a command that needs the Mosaic server. */
export function hosted(state: MockState): MockSharing {
  requireUnlocked(state);
  if (!hostedBackend(state)) {
    fail("sharingNotHosted");
  }
  return state.sharing;
}

export function contactOf(sharing: MockSharing, accountId: string): ContactView | undefined {
  return sharing.contacts.find((c) => c.accountId === accountId);
}

/** A made-up but stable code for an address the directory does not know. */
export function fakeCode(email: string): string {
  const alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  let seed = 7;
  for (const char of email) {
    seed = (seed * 31 + char.charCodeAt(0)) % 2_147_483_647;
  }
  const chars: string[] = [];
  for (let i = 0; i < 26; i += 1) {
    seed = (seed * 48_271) % 2_147_483_647;
    chars.push(alphabet[seed % alphabet.length] ?? "0");
  }
  const flat = chars.join("");
  return [flat.slice(0, 5), flat.slice(5, 10), flat.slice(10, 15), flat.slice(15, 20), flat.slice(20)].join(" ");
}

export function fakeAccountId(email: string): string {
  let hash = 0;
  for (const char of email) {
    hash = (hash * 31 + char.charCodeAt(0)) % 0xffff_ffff;
  }
  return `019a2b3c-4d5e-7030-8000-${(0x1_0000 + hash).toString(16).padStart(12, "0")}`;
}

/** What the contact book says about the key the server serves now. */
export function statusOf(contact: ContactView | undefined, served: string): ContactStatus {
  if (contact === undefined) {
    return "new";
  }
  if (contact.changed) {
    return "conflicting";
  }
  if (!sameCode(contact.safetyCode, served)) {
    return "changed";
  }
  return contact.verified ? "verified" : "pinned";
}

/** The master password of a fresh login (`wrongCredentials`, `emptyPassword`). */
export function checkPassword(state: MockState, password: string | null): void {
  if (password === null || password === "") {
    fail("emptyPassword");
  }
  if (password !== (state.account?.password ?? sampleAccount.password)) {
    fail("wrongCredentials");
  }
}

/** The sync account's second-factor code, when it has one. */
export function checkCode(state: MockState, code: string | null): void {
  if (!state.sync.totp) {
    return;
  }
  if (code === null || code === "") {
    fail("syncTwoFactorRequired");
  }
  if (code !== "123456") {
    fail("syncTwoFactorInvalid");
  }
}

export function vaultKnown(state: MockState, vaultId: string): void {
  if (!state.vaults.some((v) => v.vaultId === vaultId)) {
    fail("vaultNotFound");
  }
}

export function ownRoleOf(sharing: MockSharing, vaultId: string): ShareRole {
  return sharing.roles.get(vaultId) ?? "owner";
}

/** Whether this account may invite, change roles and remove people. */
export function requireManager(sharing: MockSharing, vaultId: string): ShareRole {
  const own = ownRoleOf(sharing, vaultId);
  if (own !== "owner" && own !== "manage") {
    fail("sharingRoleForbidden");
  }
  return own;
}

/** The members of a vault; a vault that was not shared has its owner only. */
export function membersOf(sharing: MockSharing, vaultId: string): MemberRow[] {
  const existing = sharing.members.get(vaultId);
  if (existing !== undefined) {
    return existing;
  }
  const rows: MemberRow[] = [
    {
      accountId: sampleAccount.accountId,
      email: sampleAccount.email,
      role: "owner",
      verified: true,
      self: true,
      state: "member",
      origin: "owner",
    },
  ];
  sharing.members.set(vaultId, rows);
  sharing.roles.set(vaultId, "owner");
  return rows;
}
