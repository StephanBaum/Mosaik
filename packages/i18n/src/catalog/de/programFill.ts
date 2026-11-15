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
} as const satisfies Record<keyof typeof en, string>;
