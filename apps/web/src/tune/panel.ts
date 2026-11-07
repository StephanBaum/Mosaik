/**
 * Tuning panel for the hero art, for the owner's live review. Dev server only, behind `?tune=1`:
 * main.ts imports it under `import.meta.env.DEV`, so production builds never contain it.
 * A developer tool: its labels are parameter names, not page copy.
 */
import type { HeroController } from "../hero/index.ts";
import { defaultParams, type ArtParams } from "../hero/params.ts";
import "./panel.css";

type Group = keyof ArtParams;
type Slider = readonly [group: Group, key: string, label: string, min: number, max: number, step: number];

const SLIDERS: readonly Slider[] = [
  ["block", "base", "Block size (px)", 8, 96, 1],
  ["block", "split", "Random split", 0, 1, 0.01],
  ["block", "snap", "Palette snap", 0, 1, 0.01],
  ["block", "dither", "Dither jitter", 0, 0.3, 0.005],
  ["block", "bevel", "Block bevel", 0, 0.2, 0.005],
  ["gradient", "speed", "Gradient speed", 0, 3, 0.01],
  ["gradient", "warp", "Warp", 0, 2.5, 0.01],
  ["gradient", "chroma", "Chroma lift", 0.5, 2, 0.01],
  ["gradient", "hueShift", "Hue rotate", 0, 5, 0.01],
  ["grain", "amount", "Grain", 0, 0.25, 0.001],
  ["grain", "fps", "Grain fps", 1, 60, 1],
  ["mask", "radius", "Key radius (px)", 40, 420, 1],
  ["mask", "inertia", "Key follow rate", 1, 30, 0.5],
  ["mask", "softness", "Edge softness", 0.01, 0.8, 0.01],
  ["mask", "lens", "Lens", 0, 0.8, 0.01],
  ["mask", "ring", "Ring width (px)", 0, 6, 0.1],
  ["mask", "ringAmount", "Ring amount", 0, 1, 0.01],
  ["mask", "breathe", "Breathing reveal", 0, 1, 0.01],
  ["sort", "length", "Sort length (px)", 20, 600, 1],
  ["sort", "threshold", "Sort threshold", 0, 1, 0.01],
  ["sort", "density", "Sort density", 0, 1, 0.01],
  ["sort", "strength", "Sort strength", 0, 1, 0.01],
  ["glitch", "amount", "Glitch amount", 0, 1, 0.01],
  ["glitch", "rate", "Glitch rate (/s)", 0.5, 30, 0.5],
  ["decode", "duration", "Decode (s)", 0.4, 4, 0.05],
  ["decode", "hold", "Clear hold (s)", 0, 2, 0.05],
  ["decode", "collapse", "Collapse (s)", 0.2, 3, 0.05],
];

function el<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function read(params: ArtParams, group: Group, key: string): number {
  return (params[group] as unknown as Record<string, number>)[key] ?? 0;
}

function write(params: ArtParams, group: Group, key: string, value: number): void {
  (params[group] as unknown as Record<string, number>)[key] = value;
}

function sliderRow(hero: HeroController, slider: Slider, inputs: Map<string, () => void>): HTMLElement {
  const [group, key, label, min, max, step] = slider;
  const row = el("label", "tune__row");
  const name = el("span", "tune__name", label);
  const value = el("output", "tune__value");
  const input = document.createElement("input");
  Object.assign(input, { type: "range", min: String(min), max: String(max), step: String(step) });
  const sync = (): void => {
    input.value = String(read(hero.params, group, key));
    value.textContent = read(hero.params, group, key).toFixed(3);
  };
  input.addEventListener("input", () => {
    write(hero.params, group, key, Number(input.value));
    value.textContent = Number(input.value).toFixed(3);
    hero.refresh();
  });
  sync();
  inputs.set(`${group}.${key}`, sync);
  row.append(name, input, value);
  return row;
}

function controls(hero: HeroController, inputs: Map<string, () => void>, out: HTMLPreElement): HTMLElement {
  const bar = el("div", "tune__bar");
  const replay = el("button", "tune__button", "Replay decode");
  const reset = el("button", "tune__button", "Reset");
  const copy = el("button", "tune__button tune__button--primary", "Copy values");
  replay.addEventListener("click", () => {
    hero.replay();
  });
  reset.addEventListener("click", () => {
    const fresh = defaultParams();
    for (const group of Object.keys(fresh) as Group[]) {
      Object.assign(hero.params[group], fresh[group]);
    }
    for (const sync of inputs.values()) {
      sync();
    }
    hero.refresh();
  });
  copy.addEventListener("click", () => {
    const json = JSON.stringify(hero.params, null, 2);
    out.textContent = json;
    void navigator.clipboard.writeText(json).then(() => {
      copy.textContent = "Copied";
      window.setTimeout(() => (copy.textContent = "Copy values"), 1400);
    });
  });
  bar.append(replay, reset, copy);
  return bar;
}

export function mountTune(hero: HeroController): void {
  const panel = el("details", "tune");
  panel.open = true;
  const summary = el("summary", "tune__summary", "Hero art tuning");
  const stats = el("output", "tune__stats");
  stats.dataset["testid"] = "tune-stats";
  summary.append(stats);
  const body = el("div", "tune__body");
  const inputs = new Map<string, () => void>();
  let group = "";
  for (const slider of SLIDERS) {
    if (slider[0] !== group) {
      group = slider[0];
      body.append(el("p", "tune__group", group));
    }
    body.append(sliderRow(hero, slider, inputs));
  }
  const out = el("pre", "tune__out");
  body.append(controls(hero, inputs, out), out);
  panel.append(summary, body);
  document.body.append(panel);
  // Scripted tuning sessions (Playwright) read and set the same live parameters.
  (window as unknown as { mosaicTune?: HeroController }).mosaicTune = hero;
  window.setInterval(() => {
    const s = hero.stats();
    if (s) {
      stats.textContent = `${s.fps.toFixed(0)} fps · ${String(s.scale)}x · ${(s.pixels / 1e6).toFixed(2)} Mpx${s.still ? " · still" : ""}`;
      stats.dataset["fps"] = s.fps.toFixed(1);
    }
  }, 500);
}
