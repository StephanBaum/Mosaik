/**
 * Turkish UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner": "Eşitleme bir sorun buldu. Kontrol etmek için Ayarlar’ı açın.",
  "mobile.provider.errorTitle": "Mosaic devam edemiyor",
  "mobile.onboarding.details.keyWiped":
    "Uygulamadan ayrıldığınızda Mosaic taranan Gizli Anahtar’ı sildi. Kodu yeniden tarayın veya anahtarı yazın.",
  "mobile.common.showField": "{field} alanını göster",
  "mobile.common.hideField": "{field} alanını gizle",
  "mobile.error.storeInUse":
    "Mosaic bu telefonda başka bir yerde meşgul. Biraz bekleyin, sonra yeniden deneyin.",
  "mobile.error.noAccount": "Mosaic bu telefonda henüz kurulmadı. Kurmak için Mosaic’i açın.",
  "mobile.error.wrongCredentials":
    "Bu ana parola veya Gizli Anahtar çalışmadı. Yazım hatalarını ve klavye dilini kontrol edin, sonra yeniden deneyin.",
  "mobile.error.invalidInput":
    "Girdiğiniz bir şey geçerli değil. Girdilerinizi kontrol edin ve yeniden deneyin.",
  "mobile.error.vaultUnavailable":
    "Bu kasa şu anda açılamıyor. Eşitleyip yeniden deneyin veya Mosaic’in kilidini yeniden açın.",
  "mobile.unlock.quickOptionTouchId": "Bir sonraki sefer Touch ID ile kilidi aç",
  "mobile.detail.readCode": "Kodu oku",
  "mobile.detail.labelValue": "{label}: {value}",
  "mobile.autofill.warningWithName": "Önce adresi kontrol edin | {name}",
  "mobile.autofill.linkEntry": "{login} girişini bağla ve doldur",
  "mobile.link.imported": "Bu uygulama için içe aktarıldı",
  "mobile.link.confirmTitle": "{login} girişi {app} uygulamasına bağlansın mı?",
  "mobile.link.confirmSubtitle": "Uygulama kimliği {appId} · İmza {fingerprint}",
  "mobile.link.confirmFallbackHint": "Bağladıktan sonra doldurmak için alana yeniden dokunun.",
  "mobile.link.importedNotVerified": "İçe aktarılan giriş. Web sitesi bu uygulamayı doğrulamadı.",
} satisfies Record<keyof typeof en, string>;
