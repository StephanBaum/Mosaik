import { bankItem, expect, expectNothingFilled, open, test } from "../fixtures.ts";

const PORT = "4843";

test("a look-alike host does not get the saved login", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `https://bank-login.test:${PORT}/login.html`);
  const tabId = await fake.tabIdOf("https://bank-login.test");

  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "noMatch" });
  expect(await fake.fills()).toEqual([]);
  await expectNothingFilled(page);
});

test("a homoglyph host (xn-- label) is flagged with its Unicode form and gets no fill", async ({ page, fake }) => {
  const host = new URL("https://bаnk.test").hostname; // Cyrillic a: xn--bnk-6cd.test
  expect(host.startsWith("xn--")).toBe(true);
  await fake.reset({ items: [bankItem()] });
  await open(page, `https://${host}:${PORT}/login.html`);
  const tabId = await fake.tabIdOf(`https://${host}`);

  const state = await fake.popupState(tabId);
  expect(state.tab).toMatchObject({ host, unicodeHost: "bаnk.test", secure: true, supported: true });
  expect(state.warnings).toContainEqual({ kind: "idn", unicodeHost: host });
  expect(state.matches).toEqual([]);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toEqual({ ok: false, error: "noMatch" });
  expect(await fake.gesture({ kind: "popupFill", tabId, itemId: "i1" })).toEqual({ ok: false, error: "originMismatch" });
  expect(await fake.fills()).toHaveLength(1); // the forced popup fill was refused by the desktop
  await expectNothingFilled(page);
});
