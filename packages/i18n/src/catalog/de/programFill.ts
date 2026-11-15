/** German UI strings: filling logins in other programs. Informal "du". Same keys as English (checked by the type). */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
  // Der Chip und die Liste am Feld in einem anderen Programm
  "programFill.chip.label": "Mit Mosaic ausfüllen",
  "programFill.chip.locked": "Entsperre Mosaic zum Ausfüllen",
  "programFill.list.title": "Logins für {program}",
  "programFill.list.none": "Noch kein Login für {program}",
  "programFill.list.signerChanged":
    "Der Herausgeber dieses Programms hat sich geändert. Erneut verknüpfen?",
  "programFill.list.fileChanged":
    "Dieses Programm hat sich seit dem Verknüpfen geändert. Erneut verknüpfen?",
  "programFill.list.suggestions": "Vorschläge",
  "programFill.list.search": "Logins suchen",
  "programFill.list.link": "Login verknüpfen …",
  "programFill.list.block": "Für dieses Programm nicht mehr anbieten",
  // Der Verknüpfungsschritt
  "programFill.link.packaged": "{name} · Store-App · Herausgeber {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · Store-App",
  "programFill.link.signed": "{exe} · Signiert von {signer}, von Windows geprüft",
  "programFill.link.unsigned": "{exe} · Nicht signiert · {path}",
  "programFill.link.unsignedNote":
    "Mosaic füllt nur genau diese Datei aus. Wenn sie sich ändert, fragt Mosaic erneut.",
  "programFill.link.lookalike":
    "Vorsicht: Dieser Name sieht aus wie {similar}, das du schon verknüpft hast.",
  "programFill.link.hashChange": "Verknüpfte Datei {old} …, diese Datei {new} …",
  "programFill.link.login": "Login: {title}",
  "programFill.link.confirm": "Verknüpfen und anmelden",
  "programFill.link.confirmFill": "Verknüpfen und ausfüllen",
  "programFill.link.confirmUnsigned": "Dieses unsignierte Programm verknüpfen",
  "programFill.link.confirmAgain": "Zum Verknüpfen erneut bestätigen",
  // Hinweise
  "programFill.notice.elevated":
    "Mosaic kann keine Programme ausfüllen, die als Administrator laufen. Kopiere das Passwort stattdessen aus Mosaic.",
  "programFill.notice.noField": "Mosaic erkennt hier kein Anmeldefeld.",
  "programFill.notice.aborted":
    "Mosaic hat das Ausfüllen abgebrochen, weil sich das Fenster geändert hat. Prüf das Feld.",
  "programFill.notice.abortedCleared":
    "Mosaic hat das Ausfüllen abgebrochen, weil sich das Fenster geändert hat, und das bereits Getippte entfernt.",
  "programFill.notice.modifiersHeld":
    "Lass Umschalt, Strg, Alt und die Windows-Taste los und wähle den Login dann noch einmal.",
  "programFill.notice.notResponding":
    "Dieses Programm hat nicht rechtzeitig reagiert. Es wurde nichts weiter ausgefüllt.",
  "programFill.notice.close": "Schließen",
  // Einstellungen
  "programFill.settings.title": "In Programmen ausfüllen",
  "programFill.settings.description":
    "Mosaic beobachtet auf diesem Gerät, welches Feld in anderen Programmen den Fokus hat, um dir dort deine Logins anzubieten. Nichts wird gespeichert oder gesendet.",
  "programFill.settings.enable": "Logins in Programmen anbieten",
  "programFill.settings.submit": "Nach dem Ausfüllen anmelden",
  "programFill.settings.hotkey": "Tastenkürzel für die Liste",
  "programFill.settings.hotkeyNone": "Kein Tastenkürzel",
  "programFill.settings.hotkeySave": "Kürzel für die Liste speichern",
  "programFill.settings.blocked": "Programme, für die Mosaic keine Logins anbietet",
  "programFill.settings.blockedNone": "Keine",
  "programFill.settings.unblock": "Wieder anbieten",
  "programFill.settings.notWindows": "Vorerst nur unter Windows.",
  "programFill.settings.mosaicElevated":
    "Mosaic läuft als Administrator und füllt deshalb keine Programme aus. Starte es normal.",
  "programFill.settings.hostFailed":
    "Mosaic konnte das Beobachten von Programmen nicht starten. Starte Mosaic neu, um es noch einmal zu versuchen.",
  // Die Karte nach dem ersten Entsperren
  "programFill.card.title": "Logins auch in Programmen ausfüllen",
  "programFill.card.turnOn": "Einschalten",
  "programFill.card.notNow": "Nicht jetzt",
  // Eine Programmverknüpfung in einem Eintrag
  "programFill.item.program": "Programm",
  "programFill.item.storeApp": "Store-App",
  "programFill.item.signedBy": "Signiert von {signer}",
  "programFill.item.unsigned": "Nicht signiert: {path}",
  "programFill.item.submit": "Nach dem Ausfüllen anmelden",
  "programFill.item.unlink": "Verknüpfung lösen",
} as const satisfies Record<keyof typeof en, string>;
