/**
 * Turkish UI strings: otp. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { otp as en } from "../en/otp.ts";

export const otp = {
  "otp.section.label": "Tek kullanımlık kod (2FA)",
  "otp.section.empty": "Sitenin iki adımlı doğrulama kurulum sayfasındaki kodu ekleyin.",
  "otp.section.required": "Bu öğeyi kaydetmek için bir tek kullanımlık kod ekleyin.",
  "otp.source.screen": "Ekranı tara",
  "otp.source.clipboard": "Ekran görüntüsünü yapıştır",
  "otp.source.file": "Görsel seç…",
  "otp.source.manual": "Taranamıyor mu? Anahtarı girin",
  "otp.source.screenHint":
    "Mosaic bir an kenara çekilir, ekranınıza bir kez bakar ve koddan başka hiçbir şey saklamaz.",
  "otp.source.clipboardHint":
    "{platform, select, mac {Cmd+Ctrl+Shift+4 tuşlarına basın, QR kodunu seçin, sonra buraya yapıştırın.} windows {Win+Shift+S tuşlarına basın, QR kodunu seçin, sonra buraya yapıştırın.} other {QR kodunun ekran görüntüsünü kopyalayın, sonra buraya yapıştırın.}}",
  "otp.dialog.title": "Tek kullanımlık kod kurun",
  "otp.working.screen": "Ekranınızda QR kodu aranıyor…",
  "otp.working.image": "QR kodu okunuyor…",
  "otp.manual.title": "Kurulum anahtarını girin",
  "otp.manual.input": "Kurulum anahtarı veya otpauth:// bağlantısı",
  "otp.manual.inputHint":
    "Site bunu QR kodunun yanında, çoğunlukla \"Taranamıyor mu?\" veya \"Elle gir\" altında gösterir. Boşluklar önemli değil.",
  "otp.manual.issuer": "Hizmet",
  "otp.manual.account": "Hesap",
  "otp.manual.advanced": "Gelişmiş",
  "otp.manual.algorithm": "Algoritma",
  "otp.manual.digits": "Hane sayısı",
  "otp.manual.period": "Yeni kod her",
  "otp.manual.periodValue": "{seconds} saniye",
  "otp.manual.advancedHint":
    "Bunları yalnızca site söylerse değiştirin. Çoğu site SHA-1, 6 hane ve 30 saniye kullanır.",
  "otp.manual.continue": "Devam",
  "otp.confirm.title": "Bu doğru kod mu?",
  "otp.confirm.issuer": "Hizmet",
  "otp.confirm.account": "Hesap",
  "otp.confirm.details":
    "{scheme, select, hotp {Sayaç tabanlı, {digits} hane} steam {Steam Guard, 5 karakter} other {{digits} hane, her {period} saniyede yeni kod}}",
  "otp.confirm.algorithmValue":
    "{algorithm, select, sha256 {SHA-256} sha512 {SHA-512} other {SHA-1}}",
  "otp.confirm.codeLabel": "Geçerli kod",
  "otp.confirm.codeHint": "Site kurulumu bitirmek için bir kod isterse bunu girin.",
  "otp.confirm.hotpNote":
    "Sayaç tabanlı kodlar kaydettikten sonra görünür; böylece sayaç site ile uyumlu kalır.",
  "otp.confirm.weak":
    "Bu gizli anahtar alışılmadık derecede kısa. Çalışır, ancak bu sitenin iki adımlı doğrulama koruması normalden zayıf.",
  "otp.confirm.use": "Bu kodu kullan",
  "otp.confirm.back": "Yeniden tara",
  "otp.confirm.copy": "Kodu kopyala",
  "otp.summary.noIssuer": "Tek kullanımlık kod",
  "otp.summary.pending": "Bu öğeyle birlikte kaydedilir.",
  "otp.summary.removed": "Kaydettiğinizde tek kullanımlık kod kaldırılacak.",
  "otp.action.replace": "Değiştir",
  "otp.action.remove": "Tek kullanımlık kodu kaldır",
  "otp.action.keep": "Koru",
  "otp.action.cancelSetup": "Ekleme",
  "otp.storage.label": "Kodun nerede saklanacağı",
  "otp.storage.item": "Bu öğeyle birlikte",
  "otp.storage.itemTradeoff":
    "Kolay, ancak bu kasayı açan herkes parolanızı ve ikinci adımınızı birlikte elde eder.",
  "otp.storage.hardware": "{vault} içinde (güvenlik anahtarınız gerekir)",
  "otp.storage.hardwareClosed": "Önce {vault} kasasını güvenlik anahtarınızla açın.",
  "otp.toast.clipboardCleared": "Ekran görüntüsü okundu. Mosaic panoyu temizledi.",
  "otp.toast.savedInVault": "Kod {vault} içine kaydedildi.",
  "otp.clipboardCopies":
    "{platform, select, windows {Pano geçmişi veya Ekran Alıntısı Aracı ekran görüntüsünün bir kopyasını sakladıysa, onu oradan silin.} mac {Ekran görüntüsünü ayrıca dosya olarak kaydettiyseniz, onu silin.} other {Bir pano yöneticisi ekran görüntüsünün bir kopyasını sakladıysa, onu oradan silin.}}",
} satisfies Record<keyof typeof en, string>;
