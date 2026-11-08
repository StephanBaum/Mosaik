/**
 * Tuning panel for the hero art, for the owner's live review. Dev server only, behind `?tune=1`:
 * main.ts imports it under `import.meta.env.DEV`, so production builds never contain it.
 * A developer tool: its labels are parameter names, not page copy.
 */
import type { HeroController } from "../hero/index.ts";
import { defaultParams, type SortParams } from "../hero/params.ts";
import "./panel.css";

type Group = keyof SortParams;
type Slider = readonly [group: Group, key: string, label: string, min: number, max: number, step: number];

const SLIDERS: readonly Slider[] = [
  ["sort", "threshold", "Sort threshold", 0, 1, 0.005],
  ["sort", "length", "Sort length (px)", 20, 700, 1],
  ["sort", "strength", "Sort strength", 0, 1, 0.01],
  ["band", "position", "Band position", -0.2, 1, 0.005],
  ["band", "width", "Band width", 0.01, 0.6, 0.005],
  ["band", "sky", "Sky strength", 0, 1, 0.01],
  ["band", "edge", "Right edge strength", 0, 1, 0.01],
  ["wind", "drift", "Drift speed (px/s)", 0, 120, 0.5],
  ["cursor", "radius", "Cursor relax radius (px)", 40, 600, 1],
  ["cursor", "inertia", "Cursor inertia (follow /s)", 0.5, 30, 0.5],
  ["scroll", "gain", "Scroll gain", 0, 4, 0.05],
  ["keep", "x", "Barn clean x", 0, 1, 0.005],
  ["keep", "y", "Barn clean y", 0, 1, 0.005],
  ["keep", "rx", "Barn clean radius x", 0, 0.4, 0.005],
  ["keep", "ry", "Barn clean radius y", 0, 0.4, 0.005],
  ["copy", "calm", "Calm behind copy", 0, 1, 0.01],
  ["copy", "gradient", "Gradient strength", 0, 1.2, 0.01],
];

function el<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function read(params: SortParams, group: Group, key: string): number {
  return (params[group] as unknown as Record<string, number>)[key] ?? 0;
}

function write(params: SortParams, group: Group, key: string, value: number): void {
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
  const reset = el("button", "tune__button", "Reset");
  const copy = el("button", "tune__button tune__button--primary", "Copy values");
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
  bar.append(reset, copy);
  return bar;
}

export function mountTune(hero: HeroController): void {
  const panel = el("details", "tune");
  panel.open = true;
  const summary = el("summary", "tune__summary", "Hero pixel sort");
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
      stats.textContent = `${s.fps.toFixed(0)} fps · ${String(s.scale)}x · ${(s.pixels / 1e6).toFixed(2)} Mpx`;
      stats.dataset["fps"] = s.fps.toFixed(1);
    }
  }, 500);
}
