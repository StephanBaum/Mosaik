/** German strings new on the Features page (website v2 §5.2). */
import type { features as en } from "../../en/web/features.ts";

export const features = {
  "web.featuresPage.eyebrow": "Funktionen",
  "web.featuresPage.browser": "Im Browser",
  "web.featuresPage.devices": "Auf jedem Gerät",
  "web.featuresPage.more": "Mehr als Passwörter",
  "web.featuresPage.developers": "Für Entwickler",
  "web.featuresPage.fill.site": "login.deinebank.de",
  "web.featuresPage.fill.user": "anna@example.com",
  "web.featuresPage.fill.action": "Ausfüllen",
  "web.featuresPage.fill.label": "Beispiel: das Ausfüll-Fenster auf der Website, zu der der Login gehört.",
  "web.featuresPage.cta.title": "Alles hier, kostenlos auf jedem Gerät.",
  "web.featuresPage.cta.secondary": "Tarife vergleichen",
} satisfies Record<keyof typeof en, string>;
