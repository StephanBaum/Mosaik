import { BANK, PASSWORD, USERNAME, bankItem, expect, inspect, open, test } from "../fixtures.ts";

test("fields hidden by filter, mask, scale, content-visibility or an off-screen clip stay empty", async ({
  page,
  fake,
}) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/hidden-css-tricks.html`);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({
    ok: true,
    summary: { username: 1, password: 1 },
  });
  const [frame] = await inspect(page);
  const filled = Object.entries(frame?.values ?? {}).filter(([, value]) => value !== "");
  expect(filled).toEqual([
    ["user", USERNAME],
    ["pass", PASSWORD],
  ]);
  expect((frame?.leaks ?? []).map((leak) => leak.id).sort()).toEqual(["pass", "pass", "user", "user"]);
});
