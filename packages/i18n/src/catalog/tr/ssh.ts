/**
 * Turkish UI strings: ssh. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { ssh as en } from "../en/ssh.ts";

export const ssh = {
  "ssh.title": "Bir uygulama SSH anahtarınızı kullanmak istiyor",
  "ssh.key": "Anahtar",
  "ssh.fingerprint": "Parmak izi",
  "ssh.client": "İsteyen",
  "ssh.client.unknown": "Bilinmeyen bir program",
  "ssh.expires": "{seconds, plural, one {# saniye} other {# saniye}} içinde otomatik olarak reddedilecek",
  "ssh.deny": "Reddet",
  "ssh.allowOnce": "Bir kez izin ver",
  "ssh.allowFor15Minutes": "15 dakika izin ver",
  "ssh.queue": "{count, plural, one {Bekleyen # istek daha var} other {Bekleyen # istek daha var}}",
  "ssh.signFailed.passphraseRequired":
    "Anahtar şifreli ama öğede parola yok. Öğenin Parola alanına ekleyin.",
  "ssh.signFailed.wrongPassphrase":
    "Öğedeki parola anahtarın kilidini açmıyor. Öğenin Parola alanında düzeltin.",
  "ssh.signFailed.keyMalformed":
    "Bu öğedeki özel anahtar okunamıyor. OpenSSH biçiminde yeniden yapıştırın.",
  "ssh.signFailed.kdfTooExpensive":
    "Anahtarın parola koruması, Mosaic’in izin verdiğinden daha fazla tur kullanıyor. \"ssh-keygen -p -a 64\" ile yeniden şifreleyin.",
  "ssh.signFailed.publicKeyMismatch":
    "Özel anahtar, öğenin genel anahtarıyla eşleşmiyor. Her iki alanı da kontrol edin.",
  "ssh.signFailed.unsupported": "Mosaic yalnızca Ed25519 ve ECDSA P-256 anahtarlarıyla imzalar.",
  "ssh.signFailed.internal": "İmzalama başarısız oldu. Yeniden deneyin.",
  "ssh.skip.unsupported": "Sunulmadı: Mosaic yalnızca Ed25519 ve ECDSA P-256 anahtarlarıyla imzalar.",
  "ssh.skip.malformed": "Sunulmadı: anahtar okunamıyor.",
  "ssh.skip.duplicate": "Sunulmadı: başka bir öğede aynı anahtar var.",
  "ssh.description": "Bir uygulama, Mosaic SSH aracısından bu anahtarla oturum açmasını istedi.",
  "ssh.origin.forwarded": "Bağlı olduğunuz bir sunucu bu anahtarı kullanmak istiyor.",
  "ssh.origin.bindFailed":
    "Mosaic bu isteğin nereden geldiğini bilemiyor: bağlantının ana makine denetimi başarısız oldu. Yalnızca beklediğiniz bir istekse izin verin.",
  "ssh.escDenies": "reddetmek için",
} satisfies Record<keyof typeof en, string>;
