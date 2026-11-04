/**
 * Vault choreography (study L720-743 and L764-810): finite eases, so every element is exactly
 * at rest when it settles (no overshoot). Groups are the vault pieces that move on their own:
 * nav items, footer controls, the top bar, list header, rows, detail pane and full pages.
 * The sidebar brand is deliberately not a group: the logo stays identical throughout (ADR-0027).
 */
import type { GroupKind } from "./ops.ts";
import type { Point, Scene } from "./scene.ts";
import { clamp, easeOutCubic, firstU, GLASS, lockDur, lockR, unlockR } from "./timing.ts";

export const GROUP_SELECTOR =
  ".nav-item, .sidebar__footer > *, .topbar, .list-pane__header, .item-row, .split__detail, .page";
/** Small pieces that pop in with their parent group. */
export const POP_SELECTOR = ".item-row .avatar";
/** The control a reveal grows from or closes into. */
export const ORIGIN_SELECTOR = '[data-reveal-origin="lock"]';

export interface Pose {
  /** Opacity. */
  o: number;
  tx: number;
  ty: number;
  /** Scale. */
  s: number;
  /** Blur in px. */
  blur: number;
}

/** What the schedules need to know about a group. */
export interface GroupGeom {
  readonly kind: GroupKind;
  /** Index of the enclosing group, or -1. */
  readonly parent: number;
  /** Centre in stage px. */
  readonly cx: number;
  readonly cy: number;
  /** The group holds the clicked lock control. */
  readonly origin: boolean;
}

/** Per-group distances, directions and start (unlock) or exit (lock) times, plus the run length. */
export interface Schedule {
  readonly dist: readonly number[];
  readonly dir: readonly Point[];
  readonly times: readonly number[];
  readonly total: number;
}

export const REST_POSE: Pose = { o: 1, tx: 0, ty: 0, s: 1, blur: 0 };
export const HIDDEN_POSE: Pose = { o: 0, tx: 0, ty: 0, s: 1, blur: 0 };

export function enterPose(u: number, kind: GroupKind, dir: Point): Pose {
  if (kind === "pop") {
    if (u <= 0) {
      return { o: 0, tx: 0, ty: 0, s: 0.45, blur: 0 };
    }
    return { o: clamp(u / 0.1), tx: 0, ty: 0, s: 0.45 + 0.55 * easeOutCubic(u / 0.4), blur: 0 };
  }
  if (u <= 0) {
    return { o: 0, tx: -dir[0] * 16, ty: -dir[1] * 16, s: 0.985, blur: 4 };
  }
  const e = easeOutCubic(u / 0.5);
  const k = 16 * (1 - e);
  return {
    o: easeOutCubic(u / 0.24),
    tx: -dir[0] * k,
    ty: -dir[1] * k,
    s: 0.985 + 0.015 * e,
    blur: 4 * (1 - easeOutCubic(u / 0.24)),
  };
}

export function exitPose(u: number, kind: GroupKind, dir: Point): Pose {
  const v = clamp(u / 0.17);
  const e = v * v;
  if (kind === "pop") {
    return { o: 1 - e, tx: 0, ty: 0, s: 1 - 0.5 * e, blur: 0 };
  }
  return { o: 1 - e, tx: dir[0] * 10 * e, ty: dir[1] * 10 * e, s: 1 - 0.02 * e, blur: 5 * e };
}

/** The clicked lock control: already a faint blur on the click frame, gone 80 ms later (L806). */
export function originPose(u: number): Pose {
  return { o: 0.35 * (1 - clamp(u / 0.08)), tx: 0, ty: 0, s: 0.97, blur: 3 };
}

function geometry(groups: readonly GroupGeom[], scene: Scene): { dist: number[]; dir: Point[] } {
  const dist: number[] = [];
  const dir: Point[] = [];
  for (const g of groups) {
    const dx = g.cx - scene.o[0];
    const dy = g.cy - scene.o[1];
    const length = Math.hypot(dx, dy) || 1;
    dist.push(length);
    dir.push([dx / length, dy / length]);
  }
  return { dist, dir };
}

/** When each group starts to enter: as the rim comes within `lead` px of it (L777-779). */
export function unlockSchedule(groups: readonly GroupGeom[], scene: Scene, reduced: boolean): Schedule {
  const { dist, dir } = geometry(groups, scene);
  const times = dist.map((d) =>
    reduced ? 0 : firstU((u) => unlockR(u, scene.maxD) >= d - GLASS.lead, GLASS.duration),
  );
  groups.forEach((g, i) => {
    if (g.kind === "pop" && g.parent >= 0) {
      times[i] = (times[g.parent] ?? 0) + 0.07;
    }
  });
  const total = reduced ? 0.3 : Math.max(GLASS.duration, Math.max(0, ...times) + 0.55);
  return { dist, dir, times, total };
}

/** When each group starts to leave: as the closing rim passes it. The clicked control goes at once (L782-784). */
export function lockSchedule(groups: readonly GroupGeom[], scene: Scene, reduced: boolean): Schedule {
  const { dist, dir } = geometry(groups, scene);
  const times = groups.map((g, i) =>
    reduced || g.origin
      ? 0
      : firstU((u) => lockR(u, scene.maxD) <= (dist[i] ?? 0) + 20, lockDur()),
  );
  const cross = firstU((u) => lockR(u, scene.maxD) <= 0, lockDur());
  const total = reduced ? 0.3 : Math.max(cross + 0.15, Math.max(0, ...times) + 0.2);
  return { dist, dir, times, total };
}

/** One pose per group at run time `u` (L800-810). */
export function posesAt(
  direction: "unlock" | "lock",
  u: number,
  groups: readonly GroupGeom[],
  schedule: Schedule,
): Pose[] {
  return groups.map((g, i) => {
    const dir = schedule.dir[i] ?? [0, 0];
    const time = schedule.times[i] ?? 0;
    if (direction === "unlock") {
      return enterPose(u - time, g.kind, dir);
    }
    if (g.origin) {
      return originPose(u);
    }
    const parentExit = g.parent >= 0 ? (schedule.times[g.parent] ?? 0) : 0;
    const pu = g.kind === "pop" && g.parent >= 0 ? u - parentExit + 0.04 : u - time;
    return pu <= 0 ? REST_POSE : exitPose(pu, g.kind, dir);
  });
}

/** Writes the poses to the group elements (CSSOM only, never the style attribute: CSP). */
export function applyPoses(els: readonly HTMLElement[], poses: readonly Pose[]): void {
  els.forEach((el, i) => {
    const p = poses[i];
    if (p === undefined) {
      return;
    }
    const moved = Math.abs(p.tx) + Math.abs(p.ty) > 0.01 || Math.abs(p.s - 1) > 0.0005;
    el.style.opacity = p.o >= 0.999 ? "" : p.o.toFixed(3);
    el.style.transform = moved ? `translate(${p.tx.toFixed(2)}px,${p.ty.toFixed(2)}px) scale(${p.s.toFixed(4)})` : "";
    el.style.filter = p.blur > 0.05 ? `blur(${p.blur.toFixed(2)}px)` : "";
  });
}

/** A string that changes exactly when the composed picture would (study L831). */
export function poseKey(poses: readonly Pose[]): string {
  return poses
    .map((p) => `${p.o.toFixed(3)},${p.tx.toFixed(1)},${p.ty.toFixed(1)},${p.s.toFixed(3)},${p.blur.toFixed(1)}`)
    .join("|");
}
