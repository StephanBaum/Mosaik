import {
  BANK,
  BANK_HTTP,
  EVIL,
  PASSWORD,
  USERNAME,
  bankItem,
  expect,
  expectNothingFilled,
  inspect,
  open,
  test,
} from "../fixtures.ts";

test("a confirmed frame whose src changes to another origin after the probe receives nothing", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/same-origin-iframe.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  await fake.pauseAfterProbe();
  const pending = fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: true });
  await expect.poll(() => fake.isPaused()).toBe(true);
  await page.evaluate((url) => {
    (document.getElementById("frame") as HTMLIFrameElement).src = url;
  }, `${EVIL}/login.html`);
  await expect.poll(() => page.frames().some((f) => f.url().startsWith(EVIL))).toBe(true);
  await page.frames()[1]?.waitForLoadState("load");
  await fake.release();

  expect(await pending).toMatchObject({ ok: false });
  await expectNothingFilled(page);
});

test("a confirmed frame shrunk to nothing after the probe receives nothing", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/same-origin-iframe.html`, 2);
  const tabId = await fake.tabIdOf(BANK);

  await fake.pauseAfterProbe();
  const pending = fake.gesture({ kind: "popupFill", tabId, itemId: "i1", confirmFrame: true });
  await expect.poll(() => fake.isPaused()).toBe(true);
  await page.evaluate(() => {
    const frame = document.getElementById("frame") as HTMLIFrameElement;
    frame.width = "0";
    frame.height = "0";
  });
  await fake.release();

  // The 0 x 0 frame keeps a 4 x 4 px border box, but its viewport is empty: no field in it is on
  // screen, so the re-detection at fill time finds nothing to fill.
  expect(await pending).toEqual({ ok: false, error: "noFields" });
  await expectNothingFilled(page);
});

test("an https page that downgrades to http after the probe receives nothing", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem({ allowHttp: true })] });
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);

  await fake.pauseAfterProbe();
  const pending = fake.gesture({ kind: "popupFill", tabId, itemId: "i1" });
  await expect.poll(() => fake.isPaused()).toBe(true);
  await page.goto(`${BANK_HTTP}/login.html`);
  await fake.release();

  expect(await pending).toMatchObject({ ok: false });
  await expectNothingFilled(page);
  expect(page.url().startsWith(BANK_HTTP)).toBe(true);
});

test("rapid repeated gestures deliver the one credential to the top frame only, capped in flight", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);

  // Eight at once: each one fills the one credential, or is refused as `busy` once four desktop
  // requests are in flight (MAX_IN_FLIGHT); none goes anywhere else.
  const outcomes = await Promise.all(Array.from({ length: 8 }, () => fake.gesture({ kind: "shortcut", tabId })));
  const succeeded = outcomes.filter((outcome) => outcome?.ok === true);
  expect(succeeded.length).toBeGreaterThan(0);
  for (const outcome of outcomes) {
    expect([
      { ok: true, summary: { username: 1, password: 1, otp: 0, newPassword: 0 } },
      { ok: false, error: "busy" },
    ]).toContainEqual(outcome);
  }
  const [frame] = await inspect(page);
  expect(frame?.values).toEqual({ user: USERNAME, pass: PASSWORD, code: "" });
  const fills = await fake.fills();
  expect(fills).toHaveLength(succeeded.length);
  for (const entry of fills) {
    expect(entry).toEqual({ op: "fill", origin: BANK, topOrigin: BANK, frame: "top" });
  }
});
