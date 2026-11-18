/**
 * Dutch UI strings: app. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { app as en } from "../en/app.ts";

export const app = {
  "app.name": "Mosaic",
  "app.preAudit": "Software vóór een audit: nog niet beoordeeld door een externe beveiligingsauditor.",
  "app.previewTag": "Preview",
  "app.previewBuild":
    "Previewversie, geen release. Niet ondertekend, nog niet beoordeeld, alleen om uit te proberen.",
  "action.cancel": "Annuleren",
  "action.continue": "Doorgaan",
  "action.retry": "Opnieuw proberen",
  "action.copy": "Kopiëren",
  "action.reveal": "Tonen",
  "action.hide": "Verbergen",
  "action.lock": "Vergrendelen",
  "action.details": "Details",
  "sync.offline":
    "Je synchronisatieserver is niet bereikbaar. Je wijzigingen zijn op dit apparaat opgeslagen en worden gesynchroniseerd zodra je weer online bent.",
  "app.loading": "Mosaic wordt geopend…",
  "app.skipToContent": "Naar de inhoud",
  "action.save": "Bewaren",
  "action.close": "Sluiten",
  "action.showPassword": "Wachtwoord tonen",
  "action.hidePassword": "Wachtwoord verbergen",
  "action.back": "Terug",
  "action.confirm": "Bevestigen",
  "action.edit": "Bewerken",
  "action.delete": "Naar prullenmand",
  "action.restore": "Terugzetten",
  "action.purge": "Definitief verwijderen",
  "action.generate": "Genereren",
  "action.useThis": "Dit gebruiken",
  "action.done": "Klaar",
  "nav.main": "Hoofdnavigatie",
  "nav.vault": "Kluis",
  "nav.security": "Beveiliging",
  "nav.generator": "Generator",
  "nav.settings": "Instellingen",
  "nav.trash": "Prullenmand",
  "nav.breadcrumb": "Kruimelpad",
  "nav.securityCount":
    "{count, plural, one {# bevinding om te bekijken} other {# bevindingen om te bekijken}}",
  "sync.status.local": "Opgeslagen op dit apparaat",
  "sync.status.upToDate": "Gesynchroniseerd",
  "sync.status.attention": "Aandacht nodig",
  "sync.status.syncing": "Synchroniseren…",
  "sync.status.offline": "Offline",
  "sync.status.error": "Synchroniseren mislukt",
  "action.moreActions": "Meer acties",
  "action.regenerate": "Opnieuw genereren",
} satisfies Record<keyof typeof en, string>;
