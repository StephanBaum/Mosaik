/** German UI strings of the mobile apps, second part (M10-G4). Informal "du". Same keys as English. */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner":
    "Die Synchronisierung hat ein Problem gefunden. Öffne die Einstellungen, um es zu prüfen.",
  "mobile.provider.errorTitle":
    "Mosaic kann nicht weitermachen",
  "mobile.onboarding.details.keyWiped":
    "Mosaic hat den gescannten Secret Key entfernt, als du die App verlassen hast. Scanne den Code noch einmal oder tippe den Schlüssel ein.",
  "mobile.common.showField":
    "{field} anzeigen",
  "mobile.common.hideField":
    "{field} verbergen",
  "mobile.error.storeInUse":
    "Mosaic wird auf diesem Handy gerade an anderer Stelle benutzt. Warte einen Moment und versuche es dann noch einmal.",
  "mobile.error.noAccount":
    "Mosaic ist auf diesem Handy noch nicht eingerichtet. Öffne Mosaic, um es einzurichten.",
  "mobile.error.wrongCredentials":
    "Das Master-Passwort oder der Secret Key hat nicht funktioniert. Prüfe auf Tippfehler und die Tastatursprache und versuche es noch einmal.",
  "mobile.error.invalidInput":
    "Etwas in deinen Eingaben ist ungültig. Prüfe sie und versuche es noch einmal.",
  "mobile.error.vaultUnavailable":
    "Dieser Tresor lässt sich gerade nicht öffnen. Synchronisiere und versuche es noch einmal oder entsperre Mosaic neu.",
} satisfies Record<keyof typeof en, string>;
