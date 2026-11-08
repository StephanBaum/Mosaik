/** Mock emergency-access commands (M6b): wait choices, the cooldown, a fresh login to grant and approve. */
import { publishSharing } from "./sharingData.ts";
import { checkCode, checkPassword, contactOf, hosted } from "./sharingShared.ts";
import { fail, type MockHandlers, type MockState } from "./state.ts";

const DAY = 86_400_000;
const WAIT_DAYS = [1, 2, 7, 14, 30];

/** The vault a claimed grant adds (a stable ID per grant). */
function claimedVault(index: number): string {
  return `019a2b3c-4d5e-7010-8000-${(0x30 + index).toString(16).padStart(12, "0")}`;
}

export const emergencyHandlers: MockHandlers = {
  emergency_overview: (state) => {
    const sharing = hosted(state);
    return { granted: sharing.granted, received: sharing.received };
  },

  emergency_grant: (state, request) => {
    const sharing = hosted(state);
    if (!WAIT_DAYS.includes(request.waitDays)) {
      fail("invalidInput");
    }
    if (request.scope.kind === "vaults" && request.scope.vaultIds.length === 0) {
      fail("invalidInput");
    }
    const contact = contactOf(sharing, request.accountId) ?? fail("sharingContactUnverified");
    if (contact.changed) {
      fail("sharingKeyChanged");
    }
    if (!contact.verified) {
      fail("sharingContactUnverified");
    }
    checkPassword(state, request.password);
    checkCode(state, request.code);
    const existing = sharing.granted.find((g) => g.granteeAccountId === request.accountId);
    if (existing?.state === "waiting") {
      fail("emergencyWaiting");
    }
    sharing.granted = sharing.granted.filter((g) => g.granteeAccountId !== request.accountId);
    sharing.granted.push({
      granteeAccountId: request.accountId,
      email: contact.email,
      waitDays: request.waitDays,
      scope: request.scope,
      state: "idle",
      requestedAt: null,
      releaseAt: null,
    });
    return null;
  },

  emergency_revoke: (state, request) => {
    const sharing = hosted(state);
    sharing.granted = sharing.granted.filter((g) => g.granteeAccountId !== request.accountId);
    publishSharing(state);
    return null;
  },

  emergency_request: (state, request) => {
    const sharing = hosted(state);
    const grant = sharing.received.find((r) => r.grantorAccountId === request.grantorAccountId) ?? fail("emergencyNotGranted");
    if (grant.cooldownUntil !== null && grant.cooldownUntil > state.now()) {
      fail("emergencyCooldown");
    }
    if (grant.state !== "idle") {
      fail("emergencyNotGranted");
    }
    const now = state.now();
    grant.state = "waiting";
    grant.requestedAt = now;
    grant.releaseAt = now + grant.waitDays * DAY;
    grant.cooldownUntil = null;
    return { releaseAt: grant.releaseAt };
  },

  emergency_respond: (state, request) => {
    const sharing = hosted(state);
    const grant = sharing.granted.find((g) => g.granteeAccountId === request.granteeAccountId) ?? fail("emergencyNotGranted");
    if (grant.state !== "waiting") {
      fail("emergencyNotGranted");
    }
    if (request.decision === "approve") {
      checkPassword(state, request.password);
      checkCode(state, request.code);
      grant.state = "approved";
    } else {
      grant.state = "idle";
      grant.requestedAt = null;
      grant.releaseAt = null;
    }
    publishSharing(state);
    return null;
  },

  emergency_claim: (state, request) => claim(state, request.grantorAccountId),
};

function claim(state: MockState, grantorAccountId: string): { vaults: number } {
  const sharing = hosted(state);
  const grant = sharing.received.find((r) => r.grantorAccountId === grantorAccountId) ?? fail("emergencyNotGranted");
  if (grant.state === "idle") {
    fail("emergencyNotGranted");
  }
  if (!(contactOf(sharing, grantorAccountId)?.verified ?? false)) {
    fail("sharingContactUnverified");
  }
  if (grant.state === "waiting" && (grant.releaseAt ?? 0) > state.now()) {
    fail("emergencyWaiting");
  }
  const vaultId = claimedVault(sharing.received.indexOf(grant));
  if (!state.vaults.some((v) => v.vaultId === vaultId)) {
    state.vaults.push({
      vaultId,
      name: `${grant.email.split("@")[0] ?? "Shared"}'s vault`,
      itemCount: 0,
      available: true,
      protection: "accountKey",
      openUntil: null,
    });
    sharing.roles.set(vaultId, "read");
  }
  grant.state = "released";
  state.emit({ type: "syncDataChanged", vaultIds: [vaultId] });
  return { vaults: 1 };
}
