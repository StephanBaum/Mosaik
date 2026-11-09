import { describe, expect, it } from "vitest";
import type { ItemDetail, ItemDraft } from "../contract/index.ts";
import { createMockIpc } from "../mock.ts";

const VAULT = "019a2b3c-4d5e-7010-8000-000000000001";
const GITHUB = { vaultId: VAULT, itemId: "019a2b3c-4d5e-7020-8000-000000000001" };
const DEPLOY_KEY = { vaultId: VAULT, itemId: "019a2b3c-4d5e-7020-8000-000000000007" };
const OPENSSH = "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----";

const NO_EDITS: ItemDraft = {
  text: [],
  secrets: [],
  urls: [],
  customFields: [],
  tags: null,
  otp: null,
  sshKey: null,
};

function setup() {
  const ipc = createMockIpc({ start: "unlocked" });
  async function update(item: typeof GITHUB, draft: Partial<ItemDraft>) {
    return ipc.call("update_item", { ...item, draft: { ...NO_EDITS, ...draft } });
  }
  async function detail(item: typeof GITHUB): Promise<ItemDetail> {
    const result = await ipc.call("get_item", item);
    if (!result.ok) {
      throw new Error("get_item failed");
    }
    return result.value;
  }
  async function reveal(item: typeof GITHUB, elementId: string) {
    return ipc.call("reveal_secret", { ...item, secret: { kind: "customField", elementId } });
  }
  return { ipc, update, detail, reveal };
}

describe("mock websites", () => {
  it("updates and removes a website", async () => {
    const { update, detail } = setup();
    const url = (await detail(GITHUB)).urls[0];
    expect(url).toBeDefined();
    const elementId = url?.elementId ?? "";
    const updated = await update(GITHUB, {
      urls: [
        { op: "update", elementId, url: "https://gist.github.com", matchMode: "etld1", allowHttp: false },
      ],
    });
    expect(updated.ok && updated.value.urls).toEqual([
      { elementId, url: "https://gist.github.com", matchMode: "etld1", allowHttp: false },
    ]);
    expect(updated.ok && updated.value.summary.host).toBe("gist.github.com");
    const removed = await update(GITHUB, { urls: [{ op: "remove", elementId }] });
    expect(removed.ok && removed.value.urls).toEqual([]);
    expect(removed.ok && removed.value.summary.host).toBeNull();
    const unknown = await update(GITHUB, { urls: [{ op: "remove", elementId: "nope" }] });
    expect(unknown).toEqual({ ok: false, error: { code: "invalidInput" } });
  });
});

describe("mock custom fields", () => {
  it("keeps a hidden value out of the detail and reveals it through reveal_secret", async () => {
    const { update, detail, reveal } = setup();
    const added = await update(GITHUB, {
      customFields: [
        { op: "add", label: "Door code", kind: "hidden", value: "1234" },
        { op: "add", label: "Site", kind: "url", value: "https://example.org" },
      ],
    });
    expect(added.ok).toBe(true);
    const fields = (await detail(GITHUB)).customFields;
    const [door, site] = fields;
    expect(door).toMatchObject({ label: "Door code", kind: "hidden", value: null });
    expect(site).toMatchObject({ label: "Site", kind: "url", value: "https://example.org" });
    const revealed = await reveal(GITHUB, door?.elementId ?? "");
    expect(revealed.ok && revealed.value.value).toBe("1234");
    // The quoted value: a random element ID can contain the digits.
    expect(JSON.stringify(await detail(GITHUB))).not.toContain('"1234"');
  });

  it("renames a hidden field without a value and keeps its secret", async () => {
    const { update, detail, reveal } = setup();
    await update(GITHUB, { customFields: [{ op: "add", label: "Door code", kind: "hidden", value: "1234" }] });
    const elementId = (await detail(GITHUB)).customFields[0]?.elementId ?? "";
    const renamed = await update(GITHUB, {
      customFields: [{ op: "update", elementId, label: "Gate code", kind: "hidden", value: null }],
    });
    expect(renamed.ok && renamed.value.customFields[0]?.label).toBe("Gate code");
    const revealed = await reveal(GITHUB, elementId);
    expect(revealed.ok && revealed.value.value).toBe("1234");
    const retyped = await update(GITHUB, {
      customFields: [{ op: "update", elementId, label: "Gate code", kind: "text", value: null }],
    });
    expect(retyped).toEqual({ ok: false, error: { code: "invalidInput" } });
    const replaced = await update(GITHUB, {
      customFields: [{ op: "update", elementId, label: "Gate code", kind: "hidden", value: "9999" }],
    });
    expect(replaced.ok).toBe(true);
    const again = await reveal(GITHUB, elementId);
    expect(again.ok && again.value.value).toBe("9999");
  });

  it("removes a field and its secret, and has nothing to reveal for an unknown ID", async () => {
    const { update, detail, reveal } = setup();
    await update(GITHUB, { customFields: [{ op: "add", label: "Door code", kind: "hidden", value: "1234" }] });
    const elementId = (await detail(GITHUB)).customFields[0]?.elementId ?? "";
    const removed = await update(GITHUB, { customFields: [{ op: "remove", elementId }] });
    expect(removed.ok && removed.value.customFields).toEqual([]);
    expect(await reveal(GITHUB, elementId)).toEqual({ ok: false, error: { code: "fieldEmpty" } });
    expect(await reveal(GITHUB, "unknown")).toEqual({ ok: false, error: { code: "fieldEmpty" } });
  });
});

describe("mock SSH keys", () => {
  it("derives a summary from a pasted key", async () => {
    const { update, detail } = setup();
    const set = await update(DEPLOY_KEY, { sshKey: { op: "set", privateKey: OPENSSH } });
    expect(set.ok && set.value.sshKey?.algorithm).toBe("ssh-ed25519");
    const encrypted = await update(DEPLOY_KEY, {
      sshKey: { op: "set", privateKey: `${OPENSSH}\naes256-ctr` },
    });
    expect(encrypted.ok && encrypted.value.sshKey?.encrypted).toBe(true);
    expect(JSON.stringify(await detail(DEPLOY_KEY))).not.toContain("AAAA\\n");
  });

  it("refuses text that is not a key and keys on other kinds", async () => {
    const { update } = setup();
    const garbage = await update(DEPLOY_KEY, { sshKey: { op: "set", privateKey: "hunter2" } });
    expect(garbage).toEqual({ ok: false, error: { code: "sshKeyInvalid" } });
    const login = await update(GITHUB, { sshKey: { op: "set", privateKey: OPENSSH } });
    expect(login).toEqual({ ok: false, error: { code: "invalidInput" } });
  });

  it("copies the public key of a key item only", async () => {
    const { ipc } = setup();
    const copy = (item: typeof GITHUB) =>
      ipc.call("copy_to_clipboard", { source: { kind: "item", ...item, field: { kind: "sshPublicKey" } } });
    expect((await copy(DEPLOY_KEY)).ok).toBe(true);
    expect(await copy(GITHUB)).toEqual({ ok: false, error: { code: "fieldEmpty" } });
  });
});
