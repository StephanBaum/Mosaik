/** German strings new on the Home page (website v2 §5.1). */
import type { home as en } from "../../en/web/home.ts";

export const home = {
  "web.home.reasons.link": "Zum ganzen Vergleich",
  "web.home.where.link": "So ist {name} gebaut",
  "web.home.features.link": "Alles, was es kann",
  "web.home.tool.eyebrow": "Kostenloser Selbstcheck",
  "web.home.tool.title": "Wie angreifbar bist du?",
  "web.home.tool.text":
    "Sieben Fragen zum Ankreuzen, zwei Minuten. Niemand fragt nach einem Passwort, und deine Antworten verlassen die Seite nie.",
  "web.home.tool.link": "Zum Selbstcheck",
  "web.home.pricing.link": "Tarife vergleichen",
} satisfies Record<keyof typeof en, string>;
