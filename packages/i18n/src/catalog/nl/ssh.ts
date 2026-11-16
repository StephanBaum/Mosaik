/**
 * Dutch UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of Dutch: one, other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "Een app wil je SSH-sleutel gebruiken",
  "ssh.key": "Sleutel",
  "ssh.fingerprint": "Vingerafdruk",
  "ssh.client": "Aangevraagd door",
  "ssh.client.unknown": "Een onbekend programma",
  "ssh.expires":
    "Wordt automatisch geweigerd over {seconds, plural, one {# seconde} other {# seconden}}",
  "ssh.deny": "Weigeren",
  "ssh.allowOnce": "Eenmalig toestaan",
  "ssh.allowFor15Minutes": "15 minuten toestaan",
  "ssh.queue":
    "{count, plural, one {# ander verzoek wacht} other {# andere verzoeken wachten}}",
  "ssh.signFailed.passphraseRequired":
    "De sleutel is versleuteld, maar het item heeft geen wachtzin. Voeg hem toe in het veld Wachtwoord van het item.",
  "ssh.signFailed.wrongPassphrase":
    "De wachtzin in het item ontgrendelt de sleutel niet. Corrigeer hem in het veld Wachtwoord van het item.",
  "ssh.signFailed.keyMalformed":
    "De persoonlijke sleutel in dit item kan niet worden gelezen. Plak hem opnieuw in OpenSSH-formaat.",
  "ssh.signFailed.kdfTooExpensive":
    "De wachtzinbescherming van de sleutel gebruikt meer rondes dan Mosaic toestaat. Versleutel hem opnieuw met \"ssh-keygen -p -a 64\".",
  "ssh.signFailed.publicKeyMismatch":
    "De persoonlijke sleutel komt niet overeen met de openbare sleutel van het item. Controleer beide velden.",
  "ssh.signFailed.unsupported":
    "Mosaic ondertekent alleen met Ed25519- en ECDSA P-256-sleutels.",
  "ssh.signFailed.internal": "Ondertekenen mislukt. Probeer het opnieuw.",
  "ssh.skip.unsupported":
    "Niet aangeboden: Mosaic ondertekent alleen met Ed25519- en ECDSA P-256-sleutels.",
  "ssh.skip.malformed": "Niet aangeboden: de sleutel kan niet worden gelezen.",
  "ssh.skip.duplicate": "Niet aangeboden: een ander item heeft dezelfde sleutel.",
  "ssh.description":
    "Een app vroeg de Mosaic SSH-agent om met deze sleutel in te loggen.",
  "ssh.origin.forwarded": "Een server waarmee je verbonden bent, vraagt deze sleutel te gebruiken.",
  "ssh.origin.bindFailed":
    "Mosaic kan niet zien waar dit verzoek vandaan komt: de hostcontrole van de verbinding is mislukt. Sta het alleen toe als je het verwacht.",
  "ssh.escDenies": "om te weigeren",
} satisfies Record<keyof typeof en, string>;
