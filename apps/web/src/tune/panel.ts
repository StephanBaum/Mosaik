/**
 * Mosaic tuning panel for the owner's live review (design concept §3 M1 and build step 2,
 * checkpoint A), like the study pages in docs/design/holo. Dev server only, behind `?tune=1`:
 * main.ts imports it under `import.meta.env.DEV`, so production builds never contain it.
 * A developer tool: its labels are parameter names, not page copy.
 */
import type { HeroController } from "../hero/index.ts";
import { defaultParams, type MosaicParams } from "../hero/params.ts";
import "./panel.css";

type Group = keyof MosaicParams;
type Slider = readonly [group: Group, key: string, label: string, min: number, max: number, step: number];

const SLIDERS: readonly Slider[] = [
  ["foil", "vibrance", "Vibrance", 0, 1, 0.01],
  ["foil", "lift", "Saturation lift", 1, 2, 0.01],
  ["foil", "hueBase", "Hue base", 0, 12, 0.01],
  ["foil", "hueSpread", "Hue spread", 0, 3, 0.01],
  ["foil", "hueJitter", "Hue jitter", 0, 8, 0.01],
  ["foil", "stripeScale", "Stripe scale", 0.4, 2, 0.01],
  ["foil", "timeScale", "Sheen speed", 0, 3, 0.01],
  ["foil", "sheenLag", "Sheen lag (s)", 0, 1.5, 0.01],
  ["foil", "radius", "Corner radius", 0, 0.5, 0.005],
  ["glyph", "share", "Glyph share", 0, 1, 0.01],
  ["glyph", "scale", "Glyph size", 0.2, 0.8, 0.01],
  ["glyph", "etch", "Glyph etch", 0, 1, 0.01],
  ["cursor", "sigma", "Light reach σ", 0.5, 6, 0.05],
  ["cursor", "hoverGain", "Light gain", 0, 2, 0.01],
  ["cursor", "glow", "Light glow", 0, 0.8, 0.01],
  ["cursor", "tilt", "Light reach (max)", 0, 1.2, 0.01],
  ["cursor", "followRate", "Follow rate", 2, 40, 0.5],
  ["cursor", "settleRate", "Settle rate", 1, 20, 0.5],
  ["ripple", "speed", "Ripple speed", 1, 10, 0.1],
  ["ripple", "wavelength", "Wavelength", 0.3, 4, 0.05],
  ["ripple", "runout", "Runout", 0.1, 2, 0.05],
  ["ripple", "duration", "Duration (s)", 0.4, 4, 0.05],
  ["ripple", "outline", "Outline", 0, 0.2, 0.005],
  ["ripple", "refraction", "Refraction", 0, 0.15, 0.002],
  ["ripple", "shading", "Shading", 0, 1, 0.01],
  ["intro", "stagger", "Sweep stagger (s)", 0, 0.06, 0.001],
  ["intro", "duration", "Tile in (s)", 0.1, 1, 0.01],
  ["still", "clock", "Still: clock", 0, 30, 0.1],
  ["still", "lightX", "Still: light x", -2, 12, 0.1],
  ["still", "lightY", "Still: light y", -2, 10, 0.1],
  ["still", "presence", "Still: light", 0, 1, 0.01],
];

function el<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function read(params: MosaicParams, group: Group, key: string): number {
  return (params[group] as unknown as Record<string, number>)[key] ?? 0;
}

function write(params: MosaicParams, group: Group, key: string, value: number): void {
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
  const replay = el("button", "tune__button", "Replay sweep");
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
  const summary = el("summary", "tune__summary", "Mosaic tuning");
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
    const s = hero.mosaic?.stats();
    if (s) {
      stats.textContent = `${s.fps.toFixed(0)} fps · ${String(s.scale)}x · ${(s.pixels / 1e6).toFixed(2)} Mpx${s.still ? " · still" : ""}`;
      stats.dataset["fps"] = s.fps.toFixed(1);
    }
  }, 500);
}
