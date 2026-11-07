/**
 * Motion of the hero art: the load decode, the cursor key with inertia, the breathing
 * ambient key and the scroll re-encryption, turned into one `ArtFrame` per paint.
 */
import type { ArtFrame, Key } from "./art.ts";
import type { ArtParams } from "./params.ts";
import { follow, introState, scrollLook } from "./timeline.ts";

/** Pointer activity keeps 60 fps this long (ms); otherwise the idle flow runs at 30 fps. */
const HOT_MS = 1500;
const PRESENCE_RATE = 5;
/** Gradient clock of the still frame: a balanced composition of the six colours. */
const STILL_CLOCK = 7.3;

const clamp = (x: number, lo: number, hi: number): number => Math.min(Math.max(x, lo), hi);

export class ArtMotion {
  /** Seconds since the decode started (large = settled). */
  intro = 0;
  clock = 0;
  scroll = 0;
  private time = 0;
  private target = { x: 0, y: 0, inside: false };
  private key = { x: 0, y: 0 };
  private presence = 0;
  private lastInput = -1e9;
  private placed = false;

  constructor(private readonly params: ArtParams) {}

  /** Pointer position in CSS px within the canvas; `inside` false once it leaves. */
  point(x: number, y: number, inside: boolean, now: number): void {
    this.target = { x, y, inside };
    this.lastInput = now;
    if (!this.placed && inside) {
      this.key = { x, y };
      this.placed = true;
    }
  }

  setScroll(progress: number, now: number): void {
    if (Math.abs(progress - this.scroll) > 1e-4) {
      this.lastInput = now;
    }
    this.scroll = progress;
  }

  replay(now: number): void {
    this.intro = 0;
    this.lastInput = now;
  }

  hot(now: number): boolean {
    const settling = Math.hypot(this.key.x - this.target.x, this.key.y - this.target.y) > 0.5;
    return introState(this.intro, this.params.decode).running || now - this.lastInput < HOT_MS || (settling && this.target.inside);
  }

  step(dt: number, size: { width: number; height: number }): void {
    const p = this.params;
    this.time += dt;
    this.intro += dt;
    this.clock += dt * p.gradient.speed;
    if (!this.placed) {
      this.key = { x: size.width / 2, y: size.height / 2 };
      this.target = { ...this.key, inside: false };
    }
    this.key.x = follow(this.key.x, this.target.x, p.mask.inertia, dt);
    this.key.y = follow(this.key.y, this.target.y, p.mask.inertia, dt);
    this.presence = follow(this.presence, this.target.inside ? 1 : 0, PRESENCE_RATE, dt);
  }

  frame(size: { width: number; height: number }, palette: Float32Array): ArtFrame {
    const p = this.params;
    const { width, height } = size;
    const intro = introState(this.intro, p.decode);
    const look = scrollLook(this.scroll);
    const scale = clamp(Math.min(width, height * 1.2) / 760, 0.62, 1.2);
    const radius = p.mask.radius * scale * look.keyScale;
    const diagonal = Math.hypot(width, height);
    const key: Key = {
      x: this.key.x,
      y: this.key.y,
      radius: radius + (diagonal * 1.1 - radius) * intro.reveal,
      strength: Math.max(this.presence * look.keyScale, intro.reveal),
    };
    return {
      params: p,
      palette,
      clock: this.clock,
      block: (intro.block > 0 ? intro.block : p.block.base * scale) * look.blockScale,
      key,
      ambient: this.ambient(size, radius, intro.block > 0 ? 1 : intro.reveal),
      sortBoost: look.sortBoost,
      pull: look.pull * height + this.clock * 6,
      sweep: intro.sweep,
      glitchTick: Math.floor(this.time * p.glitch.rate),
      grainTick: Math.floor(this.time * p.grain.fps),
      band: false,
    };
  }

  /** The designed still (reduced motion, Save-Data, slow GPUs): clear gradient, one encrypted band. */
  still(size: { width: number; height: number }, palette: Float32Array): ArtFrame {
    const frame = this.frame(size, palette);
    const off: Key = { x: 0, y: 0, radius: 0, strength: 0 };
    return { ...frame, clock: STILL_CLOCK, key: off, ambient: off, sortBoost: 0, pull: 0, sweep: -1, glitchTick: 3, grainTick: 0, band: true };
  }

  /** The breathing key: drifts slowly and swells; it closes while the pointer leads. */
  private ambient(size: { width: number; height: number }, radius: number, hidden: number): Key {
    const c = this.clock;
    const breath = 0.75 + 0.25 * Math.sin(c * 0.9);
    return {
      x: size.width * (0.5 + 0.26 * Math.sin(c * 0.23)),
      y: size.height * (0.48 + 0.22 * Math.cos(c * 0.17)),
      radius: radius * 1.15 * this.params.mask.breathe * breath * (1 - this.presence),
      strength: 1 - hidden,
    };
  }
}
