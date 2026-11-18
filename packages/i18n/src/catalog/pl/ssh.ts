/**
 * Polish UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "Aplikacja chce użyć Twojego klucza SSH",
  "ssh.key": "Klucz",
  "ssh.fingerprint": "Odcisk klucza",
  "ssh.client": "Żąda",
  "ssh.client.unknown": "Nieznany program",
  "ssh.expires":
    "Zostanie odrzucone automatycznie za {seconds, plural, one {# sekundę} few {# sekundy} many {# sekund} other {# sekundy}}",
  "ssh.deny": "Odmów",
  "ssh.allowOnce": "Zezwól jednorazowo",
  "ssh.allowFor15Minutes": "Zezwól na 15 minut",
  "ssh.queue":
    "{count, plural, one {# kolejne żądanie czeka} few {# kolejne żądania czekają} many {# kolejnych żądań czeka} other {# kolejnego żądania czeka}}",
  "ssh.signFailed.passphraseRequired":
    "Klucz jest zaszyfrowany, ale element nie ma frazy hasłowej. Dodaj ją w polu Hasło elementu.",
  "ssh.signFailed.wrongPassphrase":
    "Fraza hasłowa w elemencie nie odblokowuje klucza. Popraw ją w polu Hasło elementu.",
  "ssh.signFailed.keyMalformed":
    "Nie można odczytać klucza prywatnego w tym elemencie. Wklej go ponownie w formacie OpenSSH.",
  "ssh.signFailed.kdfTooExpensive":
    "Ochrona frazą hasłową tego klucza używa więcej rund, niż dopuszcza Mosaic. Zaszyfruj go ponownie poleceniem „ssh-keygen -p -a 64”.",
  "ssh.signFailed.publicKeyMismatch":
    "Klucz prywatny nie pasuje do klucza publicznego elementu. Sprawdź oba pola.",
  "ssh.signFailed.unsupported": "Mosaic podpisuje tylko kluczami Ed25519 i ECDSA P-256.",
  "ssh.signFailed.internal": "Podpisywanie nie powiodło się. Spróbuj ponownie.",
  "ssh.skip.unsupported": "Nieudostępniany: Mosaic podpisuje tylko kluczami Ed25519 i ECDSA P-256.",
  "ssh.skip.malformed": "Nieudostępniany: nie można odczytać klucza.",
  "ssh.skip.duplicate": "Nieudostępniany: inny element ma ten sam klucz.",
  "ssh.description": "Aplikacja poprosiła agenta SSH Mosaic o zalogowanie tym kluczem.",
  "ssh.origin.forwarded": "Serwer, z którym jesteś połączony, prosi o użycie tego klucza.",
  "ssh.origin.bindFailed":
    "Mosaic nie może ustalić, skąd pochodzi to żądanie: sprawdzenie hosta połączenia nie powiodło się. Zezwól tylko wtedy, gdy się go spodziewasz.",
  "ssh.escDenies": "aby odmówić",
} satisfies Record<keyof typeof en, string>;
