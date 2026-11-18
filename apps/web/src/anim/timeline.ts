/**
 * A small GSAP-shaped timeline: `to` / `set` / `call` children placed with GSAP's position
 * syntax, per-target stagger, per-child repeat and yoyo, timeline repeat with a delay. It
 * animates numbers on plain objects and, for elements, a closed list of properties it writes
 * through `el.style` (never `setAttribute("style", …)`, never HTML sinks: the site's CSP).
 */
import { power1Out, type Ease } from "./ease.ts";
import { ticker, type Ticker } from "./ticker.ts";

export type Target = Element | Record<string, number>;
export type Position = number | "<" | `+=${number}` | `-=${number}`;

export interface Motion {
  duration?: number;
  ease?: Ease;
  stagger?: number;
  repeat?: number;
  yoyo?: boolean;
  onUpdate?: () => void;
}

export interface TimelineOptions {
  /** Extra iterations after the first; -1 repeats forever. */
  repeat?: number;
  repeatDelay?: number;
  onComplete?: () => void;
  ticker?: Ticker;
}

const TRANSFORM_PROPS = ["x", "y", "xPercent", "yPercent", "scale", "scaleX", "scaleY", "rotateX", "rotateY"] as const;
const DEFAULTS: Readonly<Record<string, number>> = {
  opacity: 1,
  x: 0,
  y: 0,
  xPercent: 0,
  yPercent: 0,
  scale: 1,
  scaleX: 1,
  scaleY: 1,
  rotateX: 0,
  rotateY: 0,
  strokeDashoffset: 0,
};

const isTransformProp = (name: string): boolean => (TRANSFORM_PROPS as readonly string[]).includes(name);
const isElement = (t: Target): t is Element => "style" in t;

function checkProp(name: string): void {
  if (!(name in DEFAULTS) && !name.startsWith("--")) {
    throw new Error(`timeline: unsupported prop ${name}`);
  }
}

/** Numeric values tracked per element, so one `transform` string can compose several props. */
const tracked = new WeakMap<object, Record<string, number>>();

function stateOf(el: Element): Record<string, number> {
  let state = tracked.get(el);
  if (state === undefined) {
    state = {};
    tracked.set(el, state);
  }
  return state;
}

function readValue(target: Target, prop: string): number {
  if (!isElement(target)) {
    return target[prop] ?? 0;
  }
  return stateOf(target)[prop] ?? DEFAULTS[prop] ?? 0;
}

function transformOf(s: Readonly<Record<string, number>>): string {
  const v = (name: string): number => s[name] ?? DEFAULTS[name] ?? 0;
  return (
    `translate(${String(v("xPercent"))}%, ${String(v("yPercent"))}%) ` +
    `translate(${String(v("x"))}px, ${String(v("y"))}px) ` +
    `rotateX(${String(v("rotateX"))}deg) rotateY(${String(v("rotateY"))}deg) ` +
    `scale(${String(v("scale") * v("scaleX"))}, ${String(v("scale") * v("scaleY"))})`
  );
}

function writeValue(target: Target, prop: string, value: number): void {
  if (!isElement(target)) {
    target[prop] = value;
    return;
  }
  const el = target as HTMLElement;
  const state = stateOf(target);
  state[prop] = value;
  if (prop === "opacity") {
    el.style.opacity = String(value);
  } else if (prop === "strokeDashoffset") {
    el.style.strokeDashoffset = String(value);
  } else if (prop.startsWith("--")) {
    el.style.setProperty(prop, String(value));
  } else if (isTransformProp(prop)) {
    el.style.transform = transformOf(state);
  }
}

function writeRaw(target: Target, prop: string, value: number | string): void {
  const numeric = typeof value === "number" ? value : value.trim() !== "" && Number.isFinite(Number(value)) ? Number(value) : undefined;
  if (numeric !== undefined) {
    writeValue(target, prop, numeric);
    return;
  }
  if (!isElement(target) || isTransformProp(prop) || prop === "opacity") {
    throw new Error(`timeline: prop ${prop} needs a number`);
  }
  const el = target as HTMLElement;
  if (prop === "strokeDashoffset") {
    el.style.strokeDashoffset = String(value);
  } else {
    el.style.setProperty(prop, String(value));
  }
}

interface Item {
  target: Target;
  start: number;
  from: Record<string, number> | undefined;
  lastP: number;
}

interface ToChild {
  kind: "to";
  items: Item[];
  props: Readonly<Record<string, number>>;
  duration: number;
  ease: Ease;
  repeat: number;
  yoyo: boolean;
  onUpdate: (() => void) | undefined;
  /** Start of the first item, for the `"<"` position. */
  start: number;
  end: number;
}

interface SetChild {
  kind: "set";
  targets: readonly Target[];
  props: Readonly<Record<string, number | string>>;
  start: number;
  end: number;
}

interface CallChild {
  kind: "call";
  fn: () => void;
  start: number;
  end: number;
}

type Child = ToChild | SetChild | CallChild;

const asList = (targets: Target | readonly Target[]): readonly Target[] =>
  Array.isArray(targets) ? (targets as readonly Target[]) : [targets as Target];

export class Timeline {
  private readonly children: Child[] = [];
  private readonly repeatCount: number;
  private readonly repeatDelay: number;
  private readonly onComplete: (() => void) | undefined;
  private readonly clock: Ticker | undefined;
  private length = 0;
  private time = 0;
  /** Run time of the last forward-rendered seek, so `call`s fire once per crossing. */
  private last = -1;
  private completed = false;
  private stop: (() => void) | undefined;

  constructor(opts: TimelineOptions = {}) {
    this.repeatCount = opts.repeat ?? 0;
    this.repeatDelay = opts.repeatDelay ?? 0;
    this.onComplete = opts.onComplete;
    this.clock = opts.ticker;
  }

  /** One iteration, in seconds. */
  get duration(): number {
    return this.length;
  }

  /** Completed iterations. */
  get loops(): number {
    const period = this.length + this.repeatDelay;
    if (period <= 0) {
      return this.time > 0 ? this.cap(1) : 0;
    }
    const n = Math.floor(this.time / period);
    const within = this.time - n * period;
    return this.cap(n + (within >= this.length ? 1 : 0));
  }

  private cap(n: number): number {
    return this.repeatCount < 0 ? n : Math.min(n, this.repeatCount + 1);
  }

  private place(at: Position | undefined): number {
    if (at === undefined) {
      return this.length;
    }
    if (typeof at === "number") {
      return at;
    }
    if (at === "<") {
      return this.children[this.children.length - 1]?.start ?? 0;
    }
    const offset = Number.parseFloat(at.slice(2));
    return at.startsWith("+=") ? this.length + offset : this.length - offset;
  }

  private push(child: Child): this {
    this.children.push(child);
    this.length = Math.max(this.length, child.end);
    return this;
  }

  to(targets: Target | readonly Target[], props: Readonly<Record<string, number>>, motion: Motion = {}, at?: Position): this {
    const list = asList(targets);
    for (const prop of Object.keys(props)) {
      if (list.some(isElement)) {
        checkProp(prop);
      }
    }
    const start = this.place(at);
    const duration = motion.duration ?? 0.5;
    const stagger = motion.stagger ?? 0;
    const repeat = motion.repeat ?? 0;
    const items = list.map((target, i): Item => ({ target, start: start + i * stagger, from: undefined, lastP: -1 }));
    const end = start + Math.max(list.length - 1, 0) * stagger + duration * (repeat + 1);
    return this.push({
      kind: "to",
      items,
      props,
      duration,
      ease: motion.ease ?? power1Out,
      repeat,
      yoyo: motion.yoyo ?? false,
      onUpdate: motion.onUpdate,
      start,
      end,
    });
  }

  set(targets: Target | readonly Target[], props: Readonly<Record<string, number | string>>, at?: Position): this {
    const list = asList(targets);
    for (const prop of Object.keys(props)) {
      if (list.some(isElement)) {
        checkProp(prop);
      }
    }
    const start = this.place(at);
    return this.push({ kind: "set", targets: list, props, start, end: start });
  }

  call(fn: () => void, at?: Position): this {
    const start = this.place(at);
    return this.push({ kind: "call", fn, start, end: start });
  }

  /** Jumps the whole run (repeats included) to `time`; `call`s crossed on the way forward fire once. */
  seek(time: number): void {
    const t = Math.max(0, time);
    const period = this.length + this.repeatDelay;
    const finite = this.repeatCount >= 0;
    const total = this.length * (this.repeatCount + 1) + this.repeatDelay * this.repeatCount;
    const done = finite && t >= total;
    this.time = done ? total : t;
    const iteration = done ? this.repeatCount : period > 0 ? Math.floor(this.time / period) : 0;
    const local = done ? this.length : Math.min(this.time - iteration * period, this.length);

    if (this.time > this.last) {
      this.runCrossings(period, finite);
    }
    this.render(local);
    this.last = this.time;

    if (done && !this.completed) {
      this.completed = true;
      this.stop?.();
      this.stop = undefined;
      this.onComplete?.();
    } else if (!done) {
      this.completed = false;
    }
  }

  progress(p: number): void {
    this.seek(p * this.length);
  }

  play(): void {
    if (this.stop !== undefined) {
      return;
    }
    const clock = this.clock ?? ticker;
    this.stop = clock.add((_now, dt) => {
      this.seek(this.time + dt);
    });
  }

  pause(): void {
    this.stop?.();
    this.stop = undefined;
  }

  /** Stops the clock and forgets the recorded start values. */
  kill(): void {
    this.pause();
    for (const child of this.children) {
      if (child.kind === "to") {
        for (const item of child.items) {
          item.from = undefined;
          item.lastP = -1;
        }
      }
    }
  }

  /**
   * Calls crossed on a forward jump run in time order, each after the picture at its own time is
   * drawn, so a jump to the end leaves the same state as playing through.
   */
  private runCrossings(period: number, finite: boolean): void {
    for (const c of this.crossings(this.last, this.time, period, finite)) {
      this.render(c.local);
      c.fn();
    }
  }

  /** The calls crossed between run times `from` (exclusive) and `to` (inclusive), in time order. */
  private crossings(from: number, to: number, period: number, finite: boolean): { at: number; local: number; fn: () => void }[] {
    const found: { at: number; local: number; fn: () => void }[] = [];
    const lastIteration = finite ? this.repeatCount : Number.POSITIVE_INFINITY;
    for (const child of this.children) {
      if (child.kind !== "call") {
        continue;
      }
      for (let k = period > 0 ? Math.max(0, Math.floor((from - child.start) / period)) : 0; k <= lastIteration; k++) {
        const at = k * period + child.start;
        if (at > to) {
          break;
        }
        if (at > from) {
          found.push({ at, local: child.start, fn: child.fn });
        }
        if (period <= 0) {
          break;
        }
      }
    }
    return found.sort((x, y) => x.at - y.at);
  }

  private render(local: number): void {
    // Tweens that have not started yet fall back to their start values, latest first, so on one
    // property the earliest tween owns the rewound value (GSAP reverts in reverse order).
    for (let i = this.children.length - 1; i >= 0; i--) {
      const child = this.children[i];
      if (child?.kind === "to") {
        renderTo(child, local, true);
      }
    }
    for (const child of this.children) {
      if (child.kind === "set" && local >= child.start) {
        applySet(child);
      } else if (child.kind === "to") {
        renderTo(child, local, false);
      }
    }
  }
}

function applySet(child: SetChild): void {
  for (const target of child.targets) {
    for (const [prop, value] of Object.entries(child.props)) {
      writeRaw(target, prop, value);
    }
  }
}

/** Cycles run so far (0 to repeat + 1, fractional) for one item at `local`. */
function cyclesRun(child: ToChild, item: Item, local: number): number {
  const total = child.repeat + 1;
  if (child.duration <= 0) {
    return local >= item.start ? total : 0;
  }
  return Math.min(Math.max((local - item.start) / child.duration, 0), total);
}

/** The eased progress of a cycle count, mirrored on odd cycles when yoyoing. */
function easedProgress(child: ToChild, run: number): number {
  const total = child.repeat + 1;
  const cycle = run >= total ? total - 1 : Math.floor(run);
  const frac = run >= total ? 1 : run - cycle;
  return child.ease(!child.yoyo || cycle % 2 === 0 ? frac : 1 - frac);
}

/** `rewind` renders only items that have run before but are not started at `local`; otherwise only started items. */
function renderTo(child: ToChild, local: number, rewind: boolean): void {
  let changed = false;
  for (const item of child.items) {
    if (rewind ? local >= item.start || item.from === undefined : local < item.start) {
      continue;
    }
    const from = (item.from ??= Object.fromEntries(Object.keys(child.props).map((prop) => [prop, readValue(item.target, prop)])));
    const run = cyclesRun(child, item, local);
    const p = easedProgress(child, run);
    for (const [prop, to] of Object.entries(child.props)) {
      const start = from[prop] ?? 0;
      writeValue(item.target, prop, start + (to - start) * p);
    }
    changed ||= item.lastP !== run;
    item.lastP = run;
  }
  if (changed) {
    child.onUpdate?.();
  }
}

export function timeline(opts?: TimelineOptions): Timeline {
  return new Timeline(opts);
}
