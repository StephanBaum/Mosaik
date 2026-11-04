/** German UI strings: one-time code setup (ADR-0031). Informal "du". Same keys as English (checked by the type). */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "Einmalcode (2FA)",
  "otp.section.empty": "Füge den Code von der 2FA-Einrichtungsseite der Website hinzu.",
  "otp.section.required": "Füge einen Einmalcode hinzu, um diesen Eintrag zu speichern.",
  "otp.source.screen": "Bildschirm scannen",
  "otp.source.clipboard": "Screenshot einfügen",
  "otp.source.file": "Bild auswählen …",
  "otp.source.manual": "Scannen klappt nicht? Schlüssel eingeben",
  "otp.source.screenHint":
    "Mosaic geht kurz aus dem Weg, schaut einmal auf deinen Bildschirm und behält nur den Code.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Drück Cmd+Ctrl+Shift+4, wähl den QR-Code aus und füge ihn hier ein.} windows {Drück Win+Shift+S, wähl den QR-Code aus und füge ihn hier ein.} other {Kopier einen Screenshot des QR-Codes und füge ihn hier ein.}}",
  "otp.dialog.title": "Einmalcode einrichten",
  "otp.working.screen": "Suche nach einem QR-Code auf deinem Bildschirm …",
  "otp.working.image": "QR-Code wird gelesen …",
  "otp.manual.title": "Einrichtungsschlüssel eingeben",
  "otp.manual.input": "Einrichtungsschlüssel oder otpauth://-Link",
  "otp.manual.inputHint":
    "Die Website zeigt ihn neben dem QR-Code, oft unter „Scannen nicht möglich?“ oder „Manuell eingeben“. Leerzeichen spielen keine Rolle.",
  "otp.manual.issuer": "Dienst",
  "otp.manual.account": "Konto",
  "otp.manual.advanced": "Erweitert",
  "otp.manual.algorithm": "Algorithmus",
  "otp.manual.digits": "Stellen",
  "otp.manual.period": "Neuer Code alle",
  "otp.manual.periodValue": "{seconds} Sekunden",
  "otp.manual.advancedHint":
    "Ändere das nur, wenn die Website es verlangt. Die meisten nutzen SHA-1, 6 Stellen und 30 Sekunden.",
  "otp.manual.continue": "Weiter",
  "otp.confirm.title": "Ist das der richtige Code?",
  "otp.confirm.issuer": "Dienst",
  "otp.confirm.account": "Konto",
  "otp.confirm.details":
    "{scheme, select, hotp {Zählerbasiert, {digits} Stellen} steam {Steam Guard, 5 Zeichen} other {{digits} Stellen, neuer Code alle {period} Sekunden}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "Aktueller Code",
  "otp.confirm.codeHint": "Wenn die Website zum Abschluss nach einem Code fragt, gib diesen ein.",
  "otp.confirm.hotpNote":
    "Zählerbasierte Codes erscheinen nach dem Speichern, damit der Zähler zur Website passt.",
  "otp.confirm.weak":
    "Dieses Geheimnis ist ungewöhnlich kurz. Es funktioniert, aber der zweite Faktor dieser Website ist schwächer als üblich.",
  "otp.confirm.use": "Diesen Code verwenden",
  "otp.confirm.back": "Erneut scannen",
  "otp.confirm.copy": "Code kopieren",
  "otp.summary.noIssuer": "Einmalcode",
  "otp.summary.pending": "Wird mit diesem Eintrag gespeichert.",
  "otp.summary.removed": "Der Einmalcode wird beim Speichern entfernt.",
  "otp.action.replace": "Ersetzen",
  "otp.action.remove": "Einmalcode entfernen",
  "otp.action.keep": "Behalten",
  "otp.action.cancelSetup": "Nicht hinzufügen",
  "otp.storage.label": "Wo der Code liegt",
  "otp.storage.item": "Bei diesem Eintrag",
  "otp.storage.itemTradeoff":
    "Bequem, aber wer diesen Tresor öffnet, hat Passwort und zweiten Faktor zusammen.",
  "otp.storage.hardware": "In {vault} (braucht deinen Sicherheitsschlüssel)",
  "otp.storage.hardwareClosed": "Öffne zuerst {vault} mit deinem Sicherheitsschlüssel.",
  "otp.toast.clipboardCleared": "Screenshot gelesen. Mosaic hat die Zwischenablage geleert.",
  "otp.toast.savedInVault": "Code in {vault} gespeichert.",
  "otp.clipboardCopies":
    "{platform, select, windows {Falls der Zwischenablageverlauf oder das Snipping Tool eine Kopie des Screenshots behalten hat, lösch sie dort.} mac {Falls du den Screenshot auch als Datei gespeichert hast, lösch sie.} other {Falls ein Zwischenablage-Manager eine Kopie des Screenshots behalten hat, lösch sie dort.}}",
} satisfies Record<keyof typeof en, string>;
