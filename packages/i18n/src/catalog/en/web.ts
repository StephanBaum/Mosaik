/**
 * English UI strings: the landing page (apps/web). ICU MessageFormat, formatted at build
 * time only. Copy: marketing/landing-copy.en.md (hero option 1). "{name}" is the working
 * product name and "{domain}" the placeholder domain; neither is final.
 */
export const web = {
  "web.brand.name": "Mosaic",
  "web.site.domain": "mosaic.example",
  "web.meta.title": "{name}: trust no server, not even ours",
  "web.meta.description":
    "A password manager that encrypts everything on your device before it leaves. Even a hacked server can’t read your vault or quietly swap in an old copy.",

  // Header and navigation
  "web.skipLink": "Skip to content",
  "web.nav.label": "Main",
  "web.nav.home": "{name} home",
  "web.nav.howItWorks": "How it works",
  "web.nav.security": "Security",
  "web.nav.pricing": "Pricing",
  "web.nav.download": "Download",
  "web.nav.menu": "Menu",
  "web.nav.close": "Close menu",
  "web.lang.label": "Language",
  "web.lang.en": "EN",
  "web.lang.de": "DE",
  "web.lang.enName": "English",
  "web.lang.deName": "Deutsch",
  "web.header.cta": "Get {name}",

  // Hero (M1)
  "web.hero.label": "End-to-end encrypted password manager",
  "web.hero.titleLine1": "Trust no server.",
  "web.hero.titleLine2": "Not even {emphasis}.",
  "web.hero.titleEmphasis": "ours",
  "web.hero.subhead":
    "Everything is encrypted on your device before it leaves. Even a hacked server can’t read your vault or quietly swap in an old copy.",
  "web.hero.ctaPrimary": "Get {name} free",
  "web.hero.ctaSecondary": "See what the server sees",
  "web.hero.trustLabel": "What we publish",
  "web.hero.trust.spec": "Published crypto spec",
  "web.hero.trust.threatModel": "Published threat model",
  "web.hero.trust.telemetry": "Zero telemetry",

  // Footer
  "web.footer.label": "Footer",

  // 404
  "web.notFound.title": "This page is ciphertext.",
  "web.notFound.body": "Nothing readable lives at this address.",
  "web.notFound.back": "Back to the readable part",
} as const satisfies Record<string, string>;
