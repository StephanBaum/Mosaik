/**
 * The tokens as CSS custom properties, so app stylesheets use `var(--…)` and never raw
 * values (docs/UX_GUIDELINES.md §2). Apps bundle the result as a static stylesheet (the
 * desktop app through a Vite virtual module), which keeps the Tauri CSP free of
 * `style-src 'unsafe-inline'`.
 *
 * Theme selection: `data-theme="light" | "dark"` on the root element, or no attribute to
 * follow `prefers-color-scheme` (the default, "system").
 */
import {
  borderWidth,
  colors,
  fontFamily,
  fontSize,
  motion,
  primaryAction,
  radius,
  space,
  type ColorScheme,
} from "./index.ts";

const kebab = (name: string): string => name.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`);

function colorDeclarations(scheme: ColorScheme): string {
  return (Object.keys(scheme) as (keyof ColorScheme)[])
    .map((name) => `  --color-${kebab(name)}: ${scheme[name]};`)
    .join("\n");
}

/** Every token as a custom property, plus light, dark and system themes. */
export function cssVariables(): string {
  const base = [
    ...space.map((value, index) => `  --space-${String(index)}: ${String(value)}px;`),
    ...Object.entries(radius).map(([name, value]) => `  --radius-${name}: ${String(value)}px;`),
    ...Object.entries(borderWidth).map(
      ([name, value]) => `  --border-width-${name}: ${String(value)}px;`,
    ),
    ...Object.entries(fontSize).flatMap(([name, { size, lineHeight }]) => [
      `  --font-size-${name}: ${String(size)}px;`,
      `  --line-height-${name}: ${String(lineHeight)}px;`,
    ]),
    `  --font-ui: ${fontFamily.ui};`,
    `  --font-mono: ${fontFamily.mono};`,
    `  --motion-micro: ${motion.micro};`,
    `  --motion-panel: ${motion.panel};`,
    `  --motion-easing: ${motion.easing};`,
    // The holo-foil primary action (ADR-0014): its label, and the flat fill without WebGL2.
    `  --primary-action-label: ${primaryAction.label};`,
    `  --primary-action-fallback: ${primaryAction.fallback};`,
  ].join("\n");
  const light = `${colorDeclarations(colors.light)}\n  --primary-action-rim: ${primaryAction.rim.light};`;
  const dark = `${colorDeclarations(colors.dark)}\n  --primary-action-rim: ${primaryAction.rim.dark};`;
  return [
    `:root {\n${base}\n}`,
    `:root,\n:root[data-theme="light"] {\n  color-scheme: light;\n${light}\n}`,
    `@media (prefers-color-scheme: dark) {\n  :root:not([data-theme="light"]) {\n    color-scheme: dark;\n${dark.replace(/^ {2}/gm, "    ")}\n  }\n}`,
    `:root[data-theme="dark"] {\n  color-scheme: dark;\n${dark}\n}`,
    `@media (prefers-reduced-motion: reduce) {\n  :root {\n    --motion-micro: 0ms;\n    --motion-panel: 0ms;\n  }\n}`,
  ].join("\n\n");
}
