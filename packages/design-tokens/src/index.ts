/**
 * Mosaic design tokens. The single source of spacing, type, radius, motion and
 * color for every UI surface. See docs/UX_GUIDELINES.md §2.
 */

export const space = [0, 2, 4, 8, 12, 16, 24, 32, 48, 64] as const;

export const radius = { control: 6, card: 12, dialog: 14, pill: 9999 } as const;

/** Stroke widths (px): hairline borders and the focus ring (UX_GUIDELINES §7: ≥ 2 px). */
export const borderWidth = { hairline: 1, focus: 2 } as const;

export const fontSize = {
  xs: { size: 12, lineHeight: 16 },
  sm: { size: 14, lineHeight: 20 },
  md: { size: 16, lineHeight: 24 },
  lg: { size: 20, lineHeight: 28 },
  xl: { size: 24, lineHeight: 32 },
  xxl: { size: 32, lineHeight: 40 },
} as const;

/**
 * DM Sans (UI) and IBM Plex Mono (secrets, codes; unambiguous 0/O and 1/l/I),
 * both SIL OFL 1.1. Apps bundle the font files locally; they are never loaded
 * from a CDN.
 */
export const fontFamily = {
  ui: '"DM Sans", system-ui, -apple-system, "Segoe UI", Roboto, "Noto Sans", sans-serif',
  mono: '"IBM Plex Mono", ui-monospace, "Cascadia Mono", "SF Mono", Menlo, Consolas, monospace',
} as const;

export const motion = {
  micro: "120ms",
  panel: "200ms",
  easing: "cubic-bezier(0.2, 0, 0, 1)",
} as const;

export interface ColorScheme {
  bg: string;
  bgSubtle: string;
  surface: string;
  /** Decorative separators only; never the sole boundary of a control. */
  border: string;
  /** Boundaries of inputs and other controls (WCAG 1.4.11, ≥ 3:1). */
  borderStrong: string;
  text: string;
  textMuted: string;
  /** Fill of the single primary action (near-black; inverted in dark mode). */
  accent: string;
  accentText: string;
  danger: string;
  warning: string;
  success: string;
  focusRing: string;
}

export const colors: Readonly<Record<"light" | "dark", ColorScheme>> = {
  light: {
    bg: "#FFFFFF",
    bgSubtle: "#FAFAFA",
    surface: "#FFFFFF",
    border: "#E4E4E7",
    borderStrong: "#8A8A93",
    text: "#09090B",
    textMuted: "#71717A",
    accent: "#09090B",
    accentText: "#FAFAFA",
    danger: "#DC2626",
    warning: "#C2410C",
    success: "#15803D",
    focusRing: "#09090B",
  },
  dark: {
    bg: "#09090B",
    bgSubtle: "#0C0C0E",
    surface: "#18181B",
    border: "#27272A",
    borderStrong: "#71717A",
    text: "#FAFAFA",
    textMuted: "#A1A1AA",
    accent: "#FAFAFA",
    accentText: "#09090B",
    danger: "#F87171",
    warning: "#FB923C",
    success: "#4ADE80",
    focusRing: "#FAFAFA",
  },
};

/**
 * The single primary action of a screen: a round pill or circle in holographic
 * foil ("angled rainbow stripes"), rendered by the WebGL module prototyped in
 * docs/design/holo/. See ADR-0014. Without WebGL2 the button uses `fallback`.
 */
export const primaryAction = {
  shape: "pill",
  texture: "angled-rainbow-stripes",
  stripePeriodCssPx: 9.1,
  label: "#09090B",
  rim: { light: "#D4D4D8", dark: "rgba(230, 230, 235, 0.75)" },
  fallback: "#E4E4E7",
} as const;

/** WCAG 2.x relative luminance of a `#RRGGBB` color. */
export function relativeLuminance(hex: string): number {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!match) {
    throw new Error(`Expected a #RRGGBB color, got "${hex}"`);
  }
  const [r, g, b] = match.slice(1).map((part) => {
    const c = parseInt(part, 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG 2.x contrast ratio between two `#RRGGBB` colors (1 to 21). */
export function contrastRatio(a: string, b: string): number {
  const [hi, lo] = [relativeLuminance(a), relativeLuminance(b)].sort((x, y) => y - x) as [
    number,
    number,
  ];
  return (hi + 0.05) / (lo + 0.05);
}
