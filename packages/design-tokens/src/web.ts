/**
 * The marketing site's palette (apps/web): the dark "Linear calm" ground, warm paper type, the
 * art colours and every translucent tint the stylesheets use. Same rule as the app tokens:
 * stylesheets and canvas code take colours from here, never from literals.
 */

export const webColors = {
  ink: "#0a0a0f",
  inkRaised: "#111118",
  inkDeep: "#07070b",
  paper: "#f5f2ea",
  paper2: "#c9c6d6",
  paper3: "#9693a8",
  white: "#ffffff",
  cCobalt: "#2b4bff",
  cUltramarine: "#1a1f7a",
  cYellow: "#ffc21a",
  cTangerine: "#ff7a1a",
  cCoral: "#ff5a4e",
  cTeal: "#18a3a3",
  cMint: "#4de3b0",
  cMintSoft: "#7bebc4",
  cViolet: "#8a5cff",
  cSky: "#5ec8ff",
  cPink: "#ff7ac8",
  cAubergine: "#2a1240",
  tint: "#8a7dff",
  tintText: "#b9b0ff",
  selection: "#6b4dff",
  maskSolid: "#000000",
  /** Art-gradient stops that only the legacy page art uses. */
  cLilac: "#b79cff",
  cPeriwinkle: "#8fa2ff",
  cAzure: "#2b8bff",
  cOrchid: "#a35cff",
} as const;

/** Translucent colours and RGB triples (for `rgb(var(--x-rgb) / a)`), by CSS name without `--`. */
export const webAlphas: Readonly<Record<string, string>> = {
  "ink-glass": "rgb(18 18 26 / 0.82)",
  "ink-veil": "rgb(12 11 20 / 0.74)",
  rule: "rgb(230 226 255 / 0.08)",
  "rule-strong": "rgb(230 226 255 / 0.16)",
  "rule-bright": "rgb(230 226 255 / 0.3)",
  "grid-line": "rgb(230 226 255 / 0.045)",
  "tint-14": "rgb(122 92 255 / 0.14)",
  "tint-18": "rgb(122 92 255 / 0.18)",
  "tint-26": "rgb(122 92 255 / 0.26)",
  "sky-12": "rgb(94 200 255 / 0.12)",
  "mint-10": "rgb(77 227 176 / 0.1)",
  "mint-35": "rgb(77 227 176 / 0.35)",
  "coral-10": "rgb(255 90 78 / 0.1)",
  "coral-40": "rgb(255 90 78 / 0.4)",
  "yellow-08": "rgb(255 194 26 / 0.08)",
  "yellow-35": "rgb(255 194 26 / 0.35)",
  "paper-06": "rgb(245 242 234 / 0.06)",
  "paper-08": "rgb(245 242 234 / 0.08)",
  "paper-78": "rgb(243 239 231 / 0.78)",
  "white-025": "rgb(255 255 255 / 0.025)",
  "white-06": "rgb(255 255 255 / 0.06)",
  "white-14": "rgb(255 255 255 / 0.14)",
  "shadow-deep": "rgb(0 0 0 / 0.75)",
  "shadow-mid": "rgb(0 0 0 / 0.6)",
  "shadow-soft": "rgb(0 0 0 / 0.4)",
  scrim: "rgb(0 0 0 / 0.6)",
  "tint-rgb": "138 125 255",
  "paper-rgb": "245 242 234",
  "paper-warm-rgb": "243 239 231",
  "rule-rgb": "230 226 255",
  "veil-rgb": "12 11 20",
  "glass-rgb": "18 18 26",
  "raised-rgb": "20 19 31",
  "violet-rgb": "122 92 255",
  "lilac-rgb": "183 156 255",
  "sky-rgb": "94 200 255",
  "pink-rgb": "255 122 200",
  "mint-rgb": "77 227 176",
  "yellow-rgb": "255 194 26",
  "white-rgb": "255 255 255",
  "black-rgb": "0 0 0",
};

/** RGB triples the canvas code (data layer, panel rings) draws with. */
export const fieldColors = {
  link: [150, 140, 255],
  node: [190, 182, 255],
  label: [205, 198, 255],
  ring: [170, 160, 255],
  cover: [10, 10, 15],
} as const;

/** `paper2` becomes `paper-2`, `cCobalt` becomes `c-cobalt`. */
const kebab = (name: string): string =>
  name.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`).replace(/([a-z])(\d)/g, "$1-$2");

/**
 * Z layers only the website uses (`--layer-*`), above the shared ones in `layer`. Kept here so
 * the shared token block other surfaces embed (the server's security-key page) stays unchanged.
 */
export const webLayer = { veil: 50 } as const;

/** One `:root` block with every web token as a custom property. */
export function webCssVariables(): string {
  const lines = [
    ...Object.entries(webColors).map(([name, value]) => `  --${kebab(name)}:${value};`),
    ...Object.entries(webAlphas).map(([name, value]) => `  --${name}:${value};`),
    ...Object.entries(webLayer).map(([name, value]) => `  --layer-${kebab(name)}:${String(value)};`),
  ];
  return `:root {\n${lines.join("\n")}\n}`;
}
