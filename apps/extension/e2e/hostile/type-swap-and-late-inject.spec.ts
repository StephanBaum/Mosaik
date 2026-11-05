import { BANK, PASSWORD, USERNAME, bankItem, expect, inspect, open, test } from "../fixtures.ts";

test("fields changed or injected between the probe and the fill are classified again", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/type-swap-and-late-inject.html`);
  const tabId = await fake.tabIdOf(BANK);

  await fake.pauseAfterProbe();
  const pending = fake.gesture({ kind: "popupFill", tabId, itemId: "i1" });
  await expect.poll(() => fake.isPaused()).toBe(true);

  // After the probe saw a username and a password field, the page turns the password field
  // into a hidden one and adds a password field that cannot be seen.
  await page.evaluate(() => {
    (document.getElementById("pass") as HTMLInputElement).type = "hidden";
    const late = document.createElement("input");
    late.id = "late";
    late.type = "password";
    late.setAttribute("autocomplete", "current-password");
    late.style.opacity = "0";
    document.getElementById("login")?.append(late);
  });
  await fake.release();

  // The username field is still there and visible; the password has nowhere to go.
  expect(await pending).toMatchObject({ ok: true, summary: { username: 1, password: 0 } });
  const [frame] = await inspect(page);
  expect(frame?.values).toEqual({ user: USERNAME, pass: "", late: "" });
  expect(frame?.leaks.map((leak) => leak.id)).toEqual(["user", "user"]);
  expect(JSON.stringify(frame)).not.toContain(PASSWORD);
});
