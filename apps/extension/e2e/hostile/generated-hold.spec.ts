import { BANK, EVIL, expect, inspect, open, test } from "../fixtures.ts";

test("a password generated in the popup on one site is never filled into another site", async ({
  page,
  context,
  fake,
}) => {
  await fake.reset({ items: [] });
  await open(page, `${BANK}/signup.html`);
  const other = await context.newPage();
  try {
    await open(other, `${EVIL}/signup.html`);
    const bankTab = await fake.tabIdOf(BANK);
    const evilTab = await fake.tabIdOf(EVIL);

    // The user generates a password for the site in the popup (and copies it, say).
    const forBank = await fake.popupGenerate(bankTab);

    // Then uses "Generate password" on another site: that site gets a fresh password.
    expect(await fake.gesture({ kind: "contextMenu", tabId: evilTab, menuItemId: "generate" })).toMatchObject({
      ok: true,
      summary: { newPassword: 2 },
    });
    const [evil] = await inspect(other);
    expect(evil?.values["new"]).not.toBe("");
    expect(evil?.values["new"]).not.toBe(forBank);
    expect(JSON.stringify(evil)).not.toContain(forBank);

    // Control: on the site it was generated for, the held password is the one filled.
    const again = await fake.popupGenerate(bankTab);
    expect(await fake.gesture({ kind: "contextMenu", tabId: bankTab, menuItemId: "generate" })).toMatchObject({ ok: true });
    const [bank] = await inspect(page);
    expect(bank?.values).toMatchObject({ new: again, confirm: again });
  } finally {
    await other.close();
  }
});
