import { describe, expect, it } from "vitest";
import { createMockIpc } from "../mock.ts";
import { MOCK_PASSKEYS } from "./data.ts";

describe("mock export_passkeys_cxf", () => {
  it("needs the master password and counts the vault's passkeys", async () => {
    const ipc = createMockIpc({ start: "unlocked", browserPasskey: "get" });
    const vaults = await ipc.call("list_vaults", null);
    if (!vaults.ok) {
      throw new Error("list_vaults failed");
    }
    const vaultId = vaults.value[0]?.vaultId ?? "";
    const without = await ipc.call("export_passkeys_cxf", { vaultId, password: null });
    expect(without).toMatchObject({ ok: false, error: { code: "reauthRequired" } });
    const wrong = await ipc.call("export_passkeys_cxf", { vaultId, password: "nope" });
    expect(wrong).toMatchObject({ ok: false, error: { code: "wrongCredentials" } });
    const saved = await ipc.call("export_passkeys_cxf", { vaultId, password: "mosaic" });
    expect(saved).toEqual({ ok: true, value: { kind: "saved", count: 1 } });
    const nobody = await ipc.call("export_passkeys_cxf", {
      vaultId: "019a2b3c-0000-7000-8000-0000000000aa",
      password: "mosaic",
    });
    expect(nobody).toMatchObject({ ok: false, error: { code: "vaultNotFound" } });
  });

  it("refuses a vault without passkeys", async () => {
    // The plain mock holds one passkey, in the personal vault.
    const ipc = createMockIpc({ start: "unlocked" });
    const vaultId = MOCK_PASSKEYS.work.vaultId;
    const result = await ipc.call("export_passkeys_cxf", { vaultId, password: "mosaic" });
    expect(result).toMatchObject({ ok: false, error: { code: "itemNotFound" } });
  });
});
