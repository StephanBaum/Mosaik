/** Mock export: the passkeys of the vault are "written" after the master password is checked. */
import { fail, requireUnlocked, type MockHandlers } from "./state.ts";
import { requireFreshReauth } from "./quickUnlock.ts";

export const exportHandlers: MockHandlers = {
  export_passkeys_cxf: (state, request) => {
    requireUnlocked(state);
    const vault = state.vaults.find((v) => v.vaultId === request.vaultId) ?? fail("vaultNotFound");
    if (!vault.available || vault.protection !== "accountKey") {
      fail("vaultUnavailable");
    }
    const count = state.items.filter(
      (item) =>
        item.detail.summary.vaultId === vault.vaultId &&
        item.detail.passkey?.algorithm === "es256" &&
        item.detail.summary.trashedAt === null,
    ).length;
    if (count === 0) {
      fail("itemNotFound");
    }
    if (request.password === null) {
      requireFreshReauth(state, true, "passkeyExport");
    } else if (request.password !== state.account?.password) {
      fail("wrongCredentials");
    }
    return { kind: "saved", count };
  },
};
