/**
 * Turkish UI strings: browser. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { browser as en } from "../en/browser.ts";

export const browser = {
  "browser.pair.title": "Bir tarayıcı Mosaic’e bağlanmak istiyor",
  "browser.pair.description":
    "Buna yalnızca Mosaic uzantısında az önce Bağlan’a tıkladıysanız ve bu kodu gösteriyorsa izin verin.",
  "browser.pair.code": "Kod",
  "browser.pair.browser": "Tarayıcı",
  "browser.pair.label": "Bu tarayıcının adı",
  "browser.pair.labelHint": "Bu adı Ayarlar’da göreceksiniz.",
  "browser.pair.deny": "Reddet",
  "browser.pair.allow": "İzin ver",
  "browser.expires": "{seconds, plural, one {# saniye} other {# saniye}} içinde otomatik olarak kapanır",
  "browser.escDenies": "reddetmek için",
  "browser.hint.chrome": "Chrome",
  "browser.hint.edge": "Edge",
  "browser.hint.brave": "Brave",
  "browser.hint.opera": "Opera",
  "browser.hint.vivaldi": "Vivaldi",
  "browser.hint.chromium": "Chromium",
  "browser.hint.firefox": "Firefox",
  "browser.hint.other": "Tarayıcı",
  "browser.family.chromium": "Chromium tabanlı tarayıcı",
  "browser.family.firefox": "Firefox",
  "browser.escCancels": "iptal etmek için",
  "browser.save.titleNew": "Bu giriş Mosaic’e kaydedilsin mi?",
  "browser.save.titleUpdate": "Bu giriş Mosaic’te güncellensin mi?",
  "browser.save.description":
    "{browser} bir girişi kaydetmeyi öneriyor. Mosaic onu yalnızca Kaydet’i seçerseniz yazar.",
  "browser.save.descriptionUpdate":
    "{browser} bu giriş için yeni bir parola öneriyor. Mosaic onu yalnızca Parolayı güncelle’yi seçerseniz değiştirir.",
  "browser.save.site": "Web sitesi",
  "browser.save.username": "Kullanıcı adı",
  "browser.save.noUsername": "Yok",
  "browser.save.password": "Parola",
  "browser.save.passwordHidden": "Gizli",
  "browser.save.target": "Şu şekilde kaydet",
  "browser.save.updateOption": "“{title}” öğesini güncelle",
  "browser.save.newOption": "Yeni bir giriş",
  "browser.save.itemTitle": "Başlık",
  "browser.save.vault": "Kasa",
  "browser.save.allowHttp": "Bu şifrelenmemiş sayfada doldurmaya izin ver",
  "browser.save.allowHttpHint":
    "Site HTTPS kullanmıyor. Ağdaki herkes doldurduğunuz bilgileri okuyabilir.",
  "browser.save.cancel": "İptal",
  "browser.save.saveNew": "Kaydet",
  "browser.save.update": "Parolayı güncelle",
  "browser.passkey.secure": "Güvenli bağlantı (https)",
  "browser.passkey.getTitle": "{rpId} sitesinde geçiş anahtarıyla oturum açın",
  "browser.passkey.createTitle": "Yeni bir geçiş anahtarı kaydet",
  "browser.passkey.excludedTitle": "Geçiş anahtarı zaten kayıtlı",
  "browser.passkey.createBody": "Mosaic’te {rpId} için bir geçiş anahtarı oluşturulsun mu?",
  "browser.passkey.excludedBody": "Mosaic’te {rpId} için zaten bir geçiş anahtarınız var.",
  "browser.passkey.uvNote":
    "Site kullanıcı doğrulaması istedi. Siz olduğunuzu onaylamak için ana parolanızı girin.",
  "browser.passkey.password": "Ana parola",
  "browser.passkey.account": "Hesap",
  "browser.passkey.noName": "Ad yok",
  "browser.passkey.vault": "Kasa",
  "browser.passkey.signIn": "Geçiş anahtarıyla oturum aç",
  "browser.passkey.save": "Geçiş anahtarını kaydet",
  "browser.passkey.anotherDevice": "Başka bir aygıt veya güvenlik anahtarı kullan",
  "browser.passkey.cancel": "İptal",
  "browser.passkey.ok": "Anladım",
  "browser.passkey.escCloses": "kapatmak için",
  "browser.passkey.passwordNeeded": "Siz olduğunuzu onaylamak için ana parolanızı girin.",
  "browser.settings.title": "Tarayıcılar",
  "browser.settings.body":
    "Girişleri bu uygulamadan dolduran tarayıcı uzantıları. Her bağlantıyı burada onaylarsınız.",
  "browser.settings.empty":
    "Henüz bağlı bir tarayıcı yok. Mosaic uzantısını yükleyin, sonra içinde Bağlan’ı seçin.",
  "browser.settings.connected": "Şu anda bağlı",
  "browser.settings.lastUsed": "Son kullanım: {when}",
  "browser.settings.neverUsed": "Henüz kullanılmadı",
  "browser.settings.disconnect": "Bağlantıyı kes",
  "browser.settings.disconnectNamed": "{name} bağlantısını kes",
  "browser.settings.disconnectTitle": "“{name}” bağlantısı kesilsin mi?",
  "browser.settings.disconnectBody":
    "Bu tarayıcı, uzantıdan yeniden bağlanana kadar girişleri dolduramaz.",
  "browser.settings.endpointInUse":
    "Başka bir program veya başka bir Mosaic penceresi tarayıcı bağlantısını zaten kullanıyor. Onu kapatın, sonra Mosaic’i yeniden başlatın.",
  "browser.settings.endpointUnavailable":
    "Mosaic tarayıcı bağlantısını açamadı. Mosaic’i yeniden başlatın. Yine olursa kullanıcı klasörünüzün yazılabilir olduğunu kontrol edin.",
  "browser.settings.registryUnreadable":
    "Kayıtlı tarayıcı bağlantıları okunamıyor. Bu düzeltilene kadar Mosaic onları değiştirmeyecek.",
  "browser.connector.title": "Bir tarayıcı bağlantısı başka bir yeri gösteriyor",
  "browser.connector.body":
    "{browser} bağlantısı Mosaic yerine farklı bir program başlatıyor: {path}. Eski bir Mosaic olabilir veya kendini kaydettiren başka bir şey. Onar, bağlantıyı yeniden bu Mosaic’e yönlendirir.",
  "browser.connector.item": "{browser}: {path}",
  "browser.connector.repair": "Onar",
  "browser.connector.repaired": "Tarayıcı bağlantısı onarıldı.",
} satisfies Record<keyof typeof en, string>;
