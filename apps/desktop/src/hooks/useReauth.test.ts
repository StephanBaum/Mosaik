import { describe, expect, it } from "vitest";
import { REAUTH_BODY } from "./useReauth.tsx";

describe("useReauth", () => {
  it("derives a distinct dialog text from each purpose", () => {
    expect(Object.keys(REAUTH_BODY).sort()).toEqual(["hardwareVaultKey", "leaveHardwareVault", "vaultKeyRotation"]);
    expect(new Set(Object.values(REAUTH_BODY)).size).toBe(3);
  });
});
