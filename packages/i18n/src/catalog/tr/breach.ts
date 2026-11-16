/**
 * Turkish UI strings: breach. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { breach as en } from "../en/breach.ts";

export const breach = {
  "breach.card.title": "Veri ihlali kontrolü",
  "breach.card.off": "Kapalı. Siz açmadıkça Mosaic, Have I Been Pwned ile hiç iletişim kurmaz.",
  "breach.card.about":
    "Parolalarınızı bilinen veri ihlallerine karşı kontrol eder. Her parola için karıştırılmış bir kodun yalnızca 5 karakterlik bir parçası bu aygıttan çıkar.",
  "breach.mode.label": "Bilinen veri ihlallerine karşı kontrol et",
  "breach.mode.off": "Kapalı",
  "breach.mode.manual": "Yalnızca ben istediğimde",
  "breach.mode.weekly": "Haftalık, kilit açıldıktan sonra",
  "breach.card.checkNow": "Şimdi kontrol et",
  "breach.card.stop": "Kontrolü durdur",
  "breach.card.turnOff": "Kapat",
  "breach.progress.starting": "Parolalarınız kontrol ediliyor…",
  "breach.progress.step": "Kontrol ediliyor… {done} / {total}",
  "breach.result.never": "Henüz kontrol edilmedi.",
  "breach.result.last":
    "Son kontrol: {date, date, medium}. {found, plural, =0 {Hiçbir parola bir veri ihlalinde bulunmadı.} one {# parola bir veri ihlalinde bulundu.} other {# parola bir veri ihlalinde bulundu.}}",
  "breach.result.finished":
    "{checked, plural, one {# parola} other {# parola}} kontrol edildi. {found, plural, =0 {Hiçbiri bir veri ihlalinde bulunmadı.} one {# tanesi bir veri ihlalinde bulundu.} other {# tanesi bir veri ihlalinde bulundu.}}",
  "breach.result.upToDate": "Her şey kısa süre önce kontrol edildi. Sorgulanacak yeni bir şey yok.",
  "breach.error.offline":
    "Have I Been Pwned’e ulaşılamadı. Son sonuçlar gösteriliyor. Çevrimiçi olduğunuzda yeniden deneyin.",
  "breach.error.rateLimited":
    "Have I Been Pwned, Mosaic’ten yavaşlamasını istedi. Birkaç dakika sonra yeniden deneyin.",
  "breach.error.protocol":
    "Have I Been Pwned, Mosaic’in güvenmediği bir yanıt gönderdi; bu yüzden bazı parolalar kontrol edilmedi. Daha sonra yeniden deneyin.",
  "breach.error.cancelled": "Kontrol durduruldu. Tamamlanan sonuçlar korunur.",
  "breach.consent.title": "Parolalarınız bilinen veri ihlallerine karşı kontrol edilsin mi?",
  "breach.consent.body":
    "Mosaic her parolayı geri döndürülemeyen karıştırılmış bir koda (hash) çevirir ve bu kodun yalnızca ilk 5 karakterini Have I Been Pwned’e (api.pwnedpasswords.com) gönderir. Hizmet IP adresinizi ve bu 5 karakterlik parçaları öğrenir. Hiçbir parolayı, e-posta adresinizi veya bir parolanın hangi site için olduğunu asla görmez. Sonuçlar bu aygıtta kalır.",
  "breach.consent.detail":
    "Her 5 karakterlik parçayı çok sayıda başka parola paylaşır; bu yüzden yanıt sizinkini açığa çıkarmaz. Yine de başka bir hizmete yapılan bir istektir, bu nedenle siz onaylayana kadar kapalı kalır. İstediğiniz zaman kapatabilirsiniz.",
  "breach.consent.agree": "Aç ve şimdi kontrol et",
  "breach.consent.decline": "Şimdi değil",
} satisfies Record<keyof typeof en, string>;
