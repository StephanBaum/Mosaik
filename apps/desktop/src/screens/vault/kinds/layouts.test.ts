import { describe, expect, it } from "vitest";
import { itemKinds, secretFieldKeys, textFieldKeys } from "../../../ipc/contract/index.ts";
import { hasOtpSection, hasSshKey, hasUrls, KIND_LAYOUT, layoutOf } from "./layouts.ts";

describe("kind layouts", () => {
  it("has a layout for every creatable kind", () => {
    const creatable = itemKinds.filter((kind) => kind !== "passkey" && kind !== "unsupported");
    expect(Object.keys(KIND_LAYOUT).sort()).toEqual([...creatable].sort());
  });

  it("lists only known keys, each once, with the secret flag of its enum", () => {
    for (const [kind, layout] of Object.entries(KIND_LAYOUT)) {
      const keys = layout.map((spec) => spec.key);
      expect(new Set(keys).size, kind).toBe(keys.length);
      for (const spec of layout) {
        const inSecrets = (secretFieldKeys as readonly string[]).includes(spec.key);
        const inText = (textFieldKeys as readonly string[]).includes(spec.key);
        expect(spec.secret ? inSecrets : inText, `${kind}.${spec.key}`).toBe(true);
        expect(spec.key === "title" || spec.key === "notes").toBe(false);
      }
    }
  });

  it("has none for passkeys and newer kinds", () => {
    expect(layoutOf("passkey")).toEqual([]);
    expect(layoutOf("unsupported")).toEqual([]);
  });

  it("offers websites, a one-time code and an SSH key to the right kinds", () => {
    expect(itemKinds.filter(hasUrls)).toEqual(["login"]);
    expect(itemKinds.filter(hasOtpSection)).toEqual(["login", "totp"]);
    expect(itemKinds.filter(hasSshKey)).toEqual(["sshKey"]);
  });

  it("copies the holder, the number and the code of a card, not its PIN", () => {
    const copyable = layoutOf("creditCard")
      .filter((spec) => spec.copy)
      .map((spec) => spec.key);
    expect(copyable).toEqual(["cardholderName", "cardNumber", "cardVerificationCode"]);
  });

  it("labels the SSH passphrase as such", () => {
    expect(KIND_LAYOUT.sshKey[0]).toMatchObject({ key: "password", labelKey: "item.field.sshPassphrase" });
  });
});
