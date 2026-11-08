/** Mock contact commands (M6b): the own safety code, lookup, verification and key changes. */
import { sameCode } from "../safetyCode.ts";
import { hosted, contactOf, fakeAccountId, fakeCode, statusOf } from "./sharingShared.ts";
import { fail, requireUnlocked, type MockHandlers } from "./state.ts";

export const contactHandlers: MockHandlers = {
  sharing_identity: (state) => {
    const sharing = hosted(state);
    return { safetyCode: sharing.ownCode, published: sharing.ownCode !== null };
  },

  sharing_lookup: (state, request) => {
    const sharing = hosted(state);
    const email = request.email.trim().toLowerCase();
    if (!email.includes("@") || email.length > 320) {
      fail("invalidInput");
    }
    const entry = sharing.directory.get(email) ?? { accountId: fakeAccountId(email), code: fakeCode(email) };
    let contact = contactOf(sharing, entry.accountId);
    if (contact === undefined) {
      contact = { accountId: entry.accountId, email, safetyCode: entry.code, verified: false, changed: false };
      sharing.contacts.push(contact);
    }
    return {
      accountId: entry.accountId,
      email,
      safetyCode: entry.code,
      status: statusOf(contact, entry.code),
    };
  },

  sharing_verify_contact: (state, request) => {
    requireUnlocked(state);
    const contact = contactOf(state.sharing, request.accountId) ?? fail("sharingContactUnverified");
    if (contact.changed || !sameCode(contact.safetyCode, request.safetyCode)) {
      fail("sharingKeyChanged");
    }
    contact.verified = true;
    state.sharing.findings = state.sharing.findings.filter(
      (f) => !(f.kind === "memberUnverified" && f.accountId === request.accountId),
    );
    for (const rows of state.sharing.members.values()) {
      for (const member of rows) {
        if (member.accountId === request.accountId) {
          member.verified = true;
        }
      }
    }
    return null;
  },

  sharing_accept_key_change: (state, request) => {
    const sharing = hosted(state);
    const contact = contactOf(sharing, request.accountId) ?? fail("invalidInput");
    const served = sharing.directory.get(contact.email)?.code ?? contact.safetyCode;
    if (!sameCode(served, request.safetyCode)) {
      fail("sharingKeyChanged");
    }
    contact.safetyCode = served;
    contact.verified = false;
    contact.changed = false;
    return null;
  },

  sharing_contacts: (state) => {
    requireUnlocked(state);
    return { contacts: state.sharing.contacts };
  },
};
