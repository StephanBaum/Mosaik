import { describe, expect, it } from "vitest";
import {
  enterPose,
  exitPose,
  lockSchedule,
  posesAt,
  REST_POSE,
  unlockSchedule,
  type GroupGeom,
} from "./choreography.ts";
import { sceneFor } from "./scene.ts";
import { BAND_PIXEL_BUDGET, lockDur, firstU, lockR } from "./timing.ts";

const scene = sceneFor({ width: 1280, height: 800 }, [1000, 400], 1, BAND_PIXEL_BUDGET);

function group(cx: number, cy: number, patch: Partial<GroupGeom> = {}): GroupGeom {
  return { kind: "item", parent: -1, cx, cy, origin: false, ...patch };
}

const groups: GroupGeom[] = [
  group(900, 400),
  group(100, 100),
  group(500, 700),
  group(1200, 50),
  group(640, 400, { kind: "pop", parent: 0 }),
  group(1000, 780, { origin: true }),
];

describe("poses", () => {
  it("never leave the unit range and settle exactly at rest", () => {
    for (const kind of ["item", "pop"] as const) {
      for (let u = -0.2; u <= 1.5; u += 0.01) {
        const pose = enterPose(u, kind, [0.6, -0.8]);
        expect(pose.o).toBeGreaterThanOrEqual(0);
        expect(pose.o).toBeLessThanOrEqual(1);
        expect(pose.s).toBeLessThanOrEqual(1);
      }
      const settled = enterPose(0.5, kind, [0.6, -0.8]);
      expect(settled.o).toBe(REST_POSE.o);
      expect(settled.s).toBe(REST_POSE.s);
      expect(settled.blur).toBe(REST_POSE.blur);
      expect(Math.abs(settled.tx) + Math.abs(settled.ty)).toBe(0);
    }
  });

  it("are fully transparent by the end of an exit", () => {
    expect(exitPose(0.17, "item", [1, 0]).o).toBe(0);
    expect(exitPose(0.17, "pop", [1, 0]).o).toBe(0);
  });
});

describe("unlockSchedule", () => {
  it("starts groups in order of their distance from the origin", () => {
    const { dist, times } = unlockSchedule(groups, scene, false);
    const items = groups.map((_, i) => i).filter((i) => groups[i]?.kind === "item");
    const byDistance = items.toSorted((a, b) => (dist[a] ?? 0) - (dist[b] ?? 0));
    const starts = byDistance.map((i) => times[i] ?? 0);
    expect(starts).toEqual(starts.toSorted((a, b) => a - b));
  });

  it("lasts at least the duration, and a pop follows its parent", () => {
    const { times, total } = unlockSchedule(groups, scene, false);
    expect(total).toBeGreaterThanOrEqual(1.9);
    expect(times[4]).toBeCloseTo((times[0] ?? 0) + 0.07, 9);
  });

  it("is a short fade when reduced", () => {
    expect(unlockSchedule(groups, scene, true).total).toBe(0.3);
  });
});

describe("lockSchedule", () => {
  it("lets the clicked control go at once and outlasts the closing rim", () => {
    const { times, total } = lockSchedule(groups, scene, false);
    expect(times[5]).toBe(0);
    const cross = firstU((u) => lockR(u, scene.maxD) <= 0, lockDur());
    expect(total).toBeGreaterThanOrEqual(cross + 0.15);
  });

  it("is a short fade when reduced", () => {
    expect(lockSchedule(groups, scene, true).total).toBe(0.3);
  });

  it("poses the origin faintly, then not at all", () => {
    const schedule = lockSchedule(groups, scene, false);
    expect(posesAt("lock", 0, groups, schedule)[5]?.o).toBeCloseTo(0.35, 9);
    expect(posesAt("lock", 0.1, groups, schedule)[5]?.o).toBe(0);
  });
});
