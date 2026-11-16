/**
 * Turkish UI strings: unlock. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { unlock as en } from "../en/unlock.ts";

export const unlock = {
  "lock.unlockTitle": "Mosaic’in kilidini aç",
  "lock.masterPassword": "Ana parola",
  "lock.wrongPassword":
    "Bu ana parola çalışmadı. Caps Lock’u ve klavye düzeninizi kontrol edin, sonra yeniden deneyin.",
  "lock.locked": "Mosaic kilitli.",
  "session.unlocked": "Mosaic’in kilidi açık.",
  "lock.secretKey": "Gizli Anahtar",
  "lock.secretKeyHint":
    "Bu aygıtta Gizli Anahtarınız henüz yok. Acil Durum Kitinizden yazın; Mosaic onu bu aygıtta hatırlar.",
  "lock.submit": "Kilidi aç",
  "lock.working": "Kilit açılıyor…",
  "lock.reason.manual": "Mosaic’i kilitlediniz.",
  "lock.reason.idle": "Mosaic, bir süre hareketsiz kalınca kendini kilitledi.",
  "lock.reason.sleep": "Bilgisayar uyku moduna geçince Mosaic kilitlendi.",
  "lock.reason.screenLock": "Mosaic, ekranınızla birlikte kilitlendi.",
  "lock.or": "veya",
  "lock.quick.biometricWindows": "Windows Hello’yu kullan",
  "lock.quick.biometricMac": "Touch ID’yi kullan",
  "lock.quick.biometricLinux": "Biyometriyi kullan",
  "lock.quick.hardwareKey": "Güvenlik anahtarını kullan",
  "lock.autoLockInfo": "{minutes, plural, one {# dakika} other {# dakika}} hareketsiz kalınca otomatik kilitlenir.",
  "lock.forgot.title": "Ana parolanızı mı unuttunuz?",
  "lock.forgot.body":
    "Mosaic onu sıfırlayamaz, çünkü yalnızca siz biliyorsunuz. Acil Durum Kitinize yazıp yazmadığınızı kontrol edin. Aksi halde yeni bir hesap başlatın.",
  "lock.quick.waitingWindows": "Windows Hello bekleniyor…",
  "lock.quick.waitingMac": "Touch ID bekleniyor…",
  "lock.quick.touchKey": "Anahtarınızı takın ve yanıp sönünce dokunun.",
  "lock.quick.pin": "Güvenlik anahtarı PIN’i",
  "lock.quick.pinSubmit": "Devam",
  "lock.quick.cancel": "İptal",
} satisfies Record<keyof typeof en, string>;
