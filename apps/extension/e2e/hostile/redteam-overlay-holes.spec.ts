/**
 * RT-EXT red team: two ways to pass the hit test of `isFillable` for a field the user cannot see,
 * with an overlay that takes pointer events (so the documented `pointer-events: none` limit does
 * not apply): an opaque `<label for=decoy>` over the decoy (the test accepts a label of the
 * field), and an opaque overlay with holes at exactly the five sample points.
 */
import type { Page } from "@playwright/test";
import { BANK, PASSWORD, USERNAME, bankItem, expect, inspect, open, test } from "../fixtures.ts";

async function filledOnly(page: Page): Promise<[string, string][]> {
  const [frame] = await inspect(page);
  return Object.entries(frame?.values ?? {}).filter(([, value]) => value !== "");
}

/** What the user sees at the decoy's centre is the overlay, not the field. */
async function expectCovered(page: Page, overlayId: string): Promise<void> {
  const topAtCentre = await page.evaluate(() => {
    const box = document.getElementById("decoy")?.getBoundingClientRect();
    if (box === undefined) {
      return null;
    }
    // A point beside the centre: the user's eye, not one of the five sample points.
    return document.elementFromPoint(box.left + box.width / 2 + 10, box.top + box.height / 2 + 6)?.id ?? null;
  });
  expect(topAtCentre).toBe(overlayId);
}

// RT-EXT-1: only the field or its own descendants count as a hit (a label does not).
test("RT-EXT-1: an opaque label laid over a decoy password field does not make it fillable", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/redteam-label-overlay.html`);
  await expectCovered(page, "banner");
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true, summary: { username: 1, password: 1 } });
  expect(await filledOnly(page)).toEqual([
    ["user", USERNAME],
    ["pass", PASSWORD],
  ]);
});

// RT-EXT-2: the centre and a majority of four random points must hit, so fixed holes fail.
test("RT-EXT-2: an opaque overlay with holes at the five sample points does not make the decoy fillable", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/redteam-pinhole-overlay.html`);
  await expectCovered(page, "");
  // The holes are where the field test looks: the field is on top at each of them.
  expect(
    await page.evaluate(() =>
      (window as unknown as { __holes: { x: number; y: number }[] }).__holes.map((p) => document.elementFromPoint(p.x, p.y)?.id),
    ),
  ).toEqual(["decoy", "decoy", "decoy", "decoy", "decoy"]);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true, summary: { username: 1, password: 1 } });
  expect(await filledOnly(page)).toEqual([
    ["user", USERNAME],
    ["pass", PASSWORD],
  ]);
});
