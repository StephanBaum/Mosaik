/** German UI strings: the landing page (apps/web). Informal "du". Same keys as English (checked by the type). Copy: marketing/landing-copy.de.md. */
import type { web as en } from "../en/web.ts";

export const web = {
  "web.brand.name": "Mosaic",
  "web.site.domain": "mosaic.example",
  "web.meta.title": "{name}: Vertrau keinem Server, nicht mal unserem",
  "web.meta.description":
    "Ein Passwortmanager, der alles auf deinem Gerät verschlüsselt, bevor es das Gerät verlässt. Selbst ein gehackter Server kann deinen Tresor weder lesen noch heimlich eine alte Kopie unterschieben.",

  // Header and navigation
  "web.skipLink": "Zum Inhalt springen",
  "web.nav.label": "Hauptnavigation",
  "web.nav.home": "{name} Startseite",
  "web.nav.howItWorks": "So funktioniert’s",
  "web.nav.security": "Sicherheit",
  "web.nav.pricing": "Preise",
  "web.nav.download": "Download",
  "web.nav.menu": "Menü",
  "web.nav.close": "Menü schließen",
  "web.lang.label": "Sprache",
  "web.lang.en": "EN",
  "web.lang.de": "DE",
  "web.lang.enName": "English",
  "web.lang.deName": "Deutsch",
  "web.header.cta": "{name} holen",

  // Hero (M1)
  "web.hero.label": "Ende-zu-Ende-verschlüsselt",
  "web.hero.titleLine1": "Vertrau keinem Server.",
  "web.hero.titleLine2": "Nicht mal {emphasis}.",
  "web.hero.titleEmphasis": "unserem",
  "web.hero.subhead":
    "Alles wird auf deinem Gerät verschlüsselt, bevor es das Gerät verlässt. Selbst ein gehackter Server kann deinen Tresor weder lesen noch heimlich eine alte Kopie unterschieben.",
  "web.hero.ctaPrimary": "{name} kostenlos holen",
  "web.hero.ctaSecondary": "Sieh, was der Server sieht",
  "web.hero.trustLabel": "Was wir veröffentlichen",
  "web.hero.trust.spec": "Kryptospezifikation öffentlich",
  "web.hero.trust.threatModel": "Bedrohungsmodell öffentlich",
  "web.hero.trust.telemetry": "Null Telemetrie",

  // Footer
  "web.footer.label": "Fußzeile",

  // 404
  "web.notFound.title": "Diese Seite ist Chiffretext.",
  "web.notFound.body": "Unter dieser Adresse liegt nichts Lesbares.",
  "web.notFound.back": "Zurück zum lesbaren Teil",
} satisfies Record<keyof typeof en, string>;
