/**
 * The painted hero island. Starts after first paint: once the painting has decoded, a WebGL2
 * canvas over it paints the pixel-sorted runs (src/hero/shader.ts) and the baked band of the
 * static fallback fades out. Rendering runs only while the hero is on screen and the tab is
 * visible; under reduced motion, Save-Data, without WebGL2 or without JS the static painting
 * with its baked band stays.
 */
import { canvasScale, coverMap, parseObjectPosition, rectUv, type Cover, type Size } from "./geometry.ts";
import { attachInput, type HeroInput } from "./input.ts";
import { defaultParams, type SortParams } from "./params.ts";
import { createSortRenderer, type Frame, type SortRenderer } from "./renderer.ts";

export interface HeroStats {
  fps: number;
  /** Canvas resolution scale (device px per CSS px). */
  scale: number;
  pixels: number;
}

export interface HeroController {
  readonly params: SortParams;
  /** Applies changed parameters. */
  refresh(): void;
  stats(): HeroStats | null;
}

const LIVE = "hero--live";
/** Most canvas pixels per frame (an integrated GPU holds 60 fps well below this). */
const BUDGET = 2.0e6;
/** A frame slower than this counts toward lowering the resolution. */
const SLOW_MS = 22;
const SLOW_FRAMES = 40;
const MIN_QUALITY = 0.45;
/** Softness of the calm zone around the copy, in view uv. */
const CALM_SOFTNESS = 0.1;

export function prefersStill(): boolean {
  const saveData = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection?.saveData === true;
  return saveData || matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function decoded(img: HTMLImageElement): Promise<boolean> {
  if (img.complete && img.naturalWidth > 0) {
    return Promise.resolve(true);
  }
  return img.decode().then(
    () => img.naturalWidth > 0,
    () => false,
  );
}

interface Parts {
  hero: HTMLElement;
  img: HTMLImageElement;
  copy: HTMLElement;
}

class SortLoop {
  private raf = 0;
  private last = 0;
  private time = 0;
  private visible = true;
  private quality = 1;
  private slow = 0;
  private fps = 60;
  private view: Size = { width: 0, height: 0 };
  private scale = 1;
  private cover: Cover = [1, 1, 0, 0];
  private text: [number, number, number, number] = [0, 0, 0, 0];

  constructor(
    private readonly parts: Parts,
    private readonly renderer: SortRenderer,
    private readonly params: SortParams,
    private readonly input: HeroInput,
  ) {
    new ResizeObserver(() => {
      this.layout();
      this.kick();
    }).observe(parts.hero);
    new IntersectionObserver((entries) => {
      for (const entry of entries) {
        this.visible = entry.isIntersecting;
      }
      this.kick();
    }).observe(parts.hero);
    document.addEventListener("visibilitychange", () => {
      this.kick();
    });
    this.layout();
  }

  layout(): void {
    const { img, copy } = this.parts;
    // The canvas box is the painting's box (shorter than the hero on phones).
    const rect = this.renderer.canvas.getBoundingClientRect();
    this.view = { width: rect.width, height: rect.height };
    this.scale = canvasScale(this.view, window.devicePixelRatio || 1, this.quality, BUDGET);
    this.renderer.resize(Math.round(rect.width * this.scale), Math.round(rect.height * this.scale));
    const [px, py] = parseObjectPosition(getComputedStyle(img).objectPosition);
    this.cover = coverMap(this.view, { width: img.naturalWidth, height: img.naturalHeight }, px, py);
    this.text = rectUv(copy.getBoundingClientRect(), rect);
  }

  kick(): void {
    if (this.raf !== 0 || !this.visible || document.hidden) {
      return;
    }
    this.last = performance.now();
    this.raf = requestAnimationFrame(this.frame);
  }

  stats(): HeroStats {
    const { canvas } = this.renderer;
    return { fps: this.fps, scale: Number(this.scale.toFixed(2)), pixels: canvas.width * canvas.height };
  }

  private readonly frame = (now: number): void => {
    this.raf = 0;
    if (!this.visible || document.hidden) {
      return;
    }
    const dt = Math.min(Math.max((now - this.last) / 1000, 0), 0.1);
    this.last = now;
    this.time += dt;
    this.adapt(dt);
    this.input.step(dt, this.params.cursor.inertia);
    this.renderer.draw(this.uniforms());
    if (this.renderer.lost()) {
      this.parts.hero.classList.remove(LIVE);
      return;
    }
    this.parts.hero.classList.add(LIVE);
    this.raf = requestAnimationFrame(this.frame);
  };

  /** Tracks the frame rate and steps the resolution down while frames run slow. */
  private adapt(dt: number): void {
    if (dt <= 0) {
      return;
    }
    this.fps += (1 / dt - this.fps) * 0.1;
    this.slow = dt * 1000 > SLOW_MS ? this.slow + 1 : Math.max(this.slow - 1, 0);
    if (this.slow > SLOW_FRAMES && this.quality > MIN_QUALITY) {
      this.quality = Math.max(this.quality * 0.8, MIN_QUALITY);
      this.slow = 0;
      this.layout();
    }
  }

  private uniforms(): Frame {
    const s = this.scale;
    const p = this.params;
    const input = this.input.state;
    return {
      cover: this.cover,
      time: this.time,
      sort: [p.sort.threshold, p.sort.length * s, p.wind.drift * s, p.sort.strength],
      band: [p.band.position, p.band.width, p.band.sky, p.band.edge],
      text: this.text,
      calm: [p.copy.calm, CALM_SOFTNESS],
      cursor: [input.x * s, input.y * s, p.cursor.radius * s, input.relax],
      gain: 1 + input.scroll * p.scroll.gain,
    };
  }
}

function findParts(root: Document): Parts & { canvas: HTMLCanvasElement } | null {
  const hero = root.querySelector<HTMLElement>(".hero");
  const img = hero?.querySelector<HTMLImageElement>(".hero__img");
  const canvas = hero?.querySelector<HTMLCanvasElement>(".hero__canvas");
  const copy = hero?.querySelector<HTMLElement>(".hero__text");
  return hero && img && canvas && copy ? { hero, img, canvas, copy } : null;
}

export async function bootHero(root: Document): Promise<HeroController | null> {
  const parts = findParts(root);
  if (parts === null || prefersStill() || !(await decoded(parts.img))) {
    return null;
  }
  const renderer = await createSortRenderer(parts.canvas, parts.img);
  if (renderer === null) {
    return null;
  }
  const params = defaultParams();
  let loop: SortLoop | null = null;
  const input = attachInput(parts.hero, parts.canvas, () => loop?.kick());
  loop = new SortLoop(parts, renderer, params, input);
  loop.kick();
  const running = loop;
  return {
    params,
    refresh: () => {
      parts.hero.style.setProperty("--hero-shade", String(params.copy.gradient));
      running.kick();
    },
    stats: () => running.stats(),
  };
}
