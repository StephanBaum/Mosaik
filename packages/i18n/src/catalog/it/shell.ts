/**
 * Italian UI strings: shell. Same keys as English (checked by the type).
 * ICU plural categories of Italian: one, many, other (CLDR; `other` is required).
 */
import type { shell as en } from "../en/shell.ts";

export const shell = {
  "shell.emergencyKit.title": "Kit di emergenza di Mosaic",
  "shell.emergencyKit.intro":
    "Conserva questa pagina in un posto sicuro, ad esempio in un cassetto chiuso a chiave. La tua Chiave segreta e la tua password principale insieme sbloccano la tua cassaforte su un nuovo dispositivo. Nessuno, nemmeno Mosaic, può recuperare per te nessuna delle due.",
  "shell.emergencyKit.email": "Email dell'account",
  "shell.emergencyKit.accountId": "ID account",
  "shell.emergencyKit.signInUrl": "Indirizzo di accesso",
  "shell.emergencyKit.signInUrlLocal":
    "Solo questo dispositivo (nessun server di sincronizzazione ancora configurato)",
  "shell.emergencyKit.secretKey": "Chiave segreta",
  "shell.emergencyKit.masterPassword": "Password principale",
  "shell.emergencyKit.masterPasswordHint":
    "Scrivila a mano sulla riga solo se questa pagina resta in un posto sicuro. Altrimenti tienila a mente.",
  "shell.emergencyKit.footer":
    "Mosaic è un software in fase pre-audit: non è ancora stato verificato da un revisore di sicurezza esterno.",
  "shell.shortcut.conflict":
    "Un'altra app usa già {shortcut}. Chiudi quell'app o cambia la sua scorciatoia, poi riavvia Mosaic per usare la ricerca rapida.",
  "keys.ctrl": "Ctrl",
  "keys.shift": "Maiusc",
  "keys.alt": "Alt",
  "keys.space": "Spazio",
  "keys.esc": "Esc",
  "keys.enter": "Invio",
} satisfies Record<keyof typeof en, string>;
