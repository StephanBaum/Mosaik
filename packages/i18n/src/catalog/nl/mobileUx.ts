/**
 * Dutch UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner":
    "Synchronisatie vond een probleem. Open Instellingen om het te controleren.",
  "mobile.provider.errorTitle": "Mosaic kan niet doorgaan",
  "mobile.onboarding.details.keyWiped":
    "Mosaic heeft de gescande Geheime Sleutel verwijderd toen je de app verliet. Scan de code opnieuw of typ de sleutel.",
  "mobile.common.showField": "{field} tonen",
  "mobile.common.hideField": "{field} verbergen",
  "mobile.error.storeInUse":
    "Mosaic is elders op deze telefoon bezig. Wacht even en probeer het opnieuw.",
  "mobile.error.noAccount":
    "Mosaic is nog niet ingesteld op deze telefoon. Open Mosaic om het in te stellen.",
  "mobile.error.wrongCredentials":
    "Dat hoofdwachtwoord of die Geheime Sleutel werkte niet. Controleer op typefouten en de toetsenbordtaal en probeer het opnieuw.",
  "mobile.error.invalidInput":
    "Iets wat je hebt ingevoerd is ongeldig. Controleer je invoer en probeer het opnieuw.",
  "mobile.error.vaultUnavailable":
    "Deze kluis kan nu niet worden geopend. Synchroniseer en probeer het opnieuw, of ontgrendel Mosaic opnieuw.",
  "mobile.unlock.quickOptionTouchId": "De volgende keer ontgrendelen met Touch ID",
  "mobile.detail.readCode": "Code voorlezen",
  "mobile.detail.labelValue": "{label}: {value}",
  "mobile.autofill.warningWithName": "Controleer eerst het adres | {name}",
  "mobile.autofill.linkEntry": "{login} koppelen en invullen",
  "mobile.link.imported": "Geïmporteerd voor deze app",
  "mobile.link.confirmTitle": "{login} aan {app} koppelen?",
  "mobile.link.confirmSubtitle": "App-ID {appId} · Handtekening {fingerprint}",
  "mobile.link.confirmFallbackHint": "Tik na het koppelen opnieuw op het veld om in te vullen.",
  "mobile.link.importedNotVerified":
    "Geïmporteerde login. De website heeft deze app niet bevestigd.",
} satisfies Record<keyof typeof en, string>;
