/**
 * Turkish UI strings: settings. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { settings as en } from "../en/settings.ts";

export const settings = {
  "settings.title": "Ayarlar",
  "settings.security": "Güvenlik",
  "settings.autoLock": "Hareketsizlikten sonra kilitle",
  "settings.autoLock.never": "Asla",
  "settings.autoLock.minutes": "{minutes, plural, one {# dakika} other {# dakika}}",
  "settings.lockOnSleep": "Bilgisayar uyku moduna geçince kilitle",
  "settings.lockOnScreenLock": "Ekran kilitlenince kilitle",
  "settings.screenLock.unsupported":
    "Mosaic bu sistemde ekranın ne zaman kilitlendiğini anlayamıyor. Yine de hareketsizlikten sonra ve bilgisayar uyandığında kilitlenir.",
  "settings.lockTrigger.required":
    "En az bir kilidi açık tutun. Mosaic bu sistemde kilitli bir ekranı algılayamıyor.",
  "settings.clipboard": "Panoyu şu süreden sonra temizle",
  "settings.clipboard.never": "Asla",
  "settings.clipboard.seconds": "{seconds, plural, one {# saniye} other {# saniye}}",
  "settings.appearance": "Görünüm",
  "settings.theme": "Tema",
  "settings.theme.system": "Sistemi izle",
  "settings.theme.light": "Açık",
  "settings.theme.dark": "Koyu",
  "settings.language": "Dil",
  "settings.language.system": "Sistemi izle",
  "settings.language.machineNote": "Yapay zekâ yardımıyla çevrildi. Kulağa yanlış gelen bir yer varsa bize bildirin.",
  "settings.import": "İçe aktar",
  "settings.import.body":
    "Girişlerinizi bir tarayıcıdan veya başka bir parola yöneticisinden getirin. Mosaic dışa aktarma sürecinde size yol gösterir ve sonrasında dosyayı silmenize yardımcı olur.",
  "settings.import.open": "Parolaları içe aktar",
  "settings.ssh": "SSH aracısı",
  "settings.ssh.enable": "SSH anahtarlarımı Mosaic’ten kullan",
  "settings.ssh.hint":
    "Varsayılan olarak kapalı. Aracı yalnızca Mosaic’in kilidi açıkken çalışır ve her kullanımdan önce size sorar.",
  "settings.ssh.status":
    "{count, plural, =0 {Sunulan anahtar yok} one {# anahtar sunuluyor} other {# anahtar sunuluyor}}",
  "settings.ssh.endpoint": "SSH_AUTH_SOCK değerini şuna ayarlayın",
  "settings.ssh.skipped": "Sunulmayan anahtarlar",
  "settings.kit": "Acil Durum Kiti",
  "settings.kit.save": "Acil Durum Kitini yeniden kaydet",
  "settings.saved": "Ayarlar kaydedildi.",
  "reauth.title": "Ana parolanızla onaylayın",
  "reauth.body": "Bu değişiklik Mosaic’i daha az güvenli yapar, bu yüzden ana parolanızı gerektirir.",
  "reauth.bodyKit":
    "Gizli Anahtarınız bir dosyada gösterilmek üzere, bu yüzden Mosaic ana parolanıza ihtiyaç duyuyor.",
  "reauth.password": "Ana parola",
  "reauth.submit": "Onayla",
  "settings.intro": "Mosaic’in bu aygıtta nasıl kilitlendiği, gizli bilgileri nasıl temizlediği ve anahtarlarınızı nasıl koruduğu.",
  "settings.section.autoLock": "Otomatik kilit",
  "settings.section.autoLock.body": "Siz istemeden Mosaic’in kendini ne zaman kilitlediği.",
  "settings.section.clipboard": "Pano",
  "settings.section.clipboard.body": "Kopyalanan gizli bilgiler bu süreden sonra temizlenir.",
  "settings.appearance.body": "Mosaic’in teması ve dili.",
  "settings.ssh.body": "Programların kasanızdaki SSH anahtarlarıyla oturum açmasına izin verin. Her kullanımı siz onaylarsınız.",
  "settings.kit.body": "Acil Durum Kitinizin yeni bir kopyasını kaydedin. Bu, ana parolanızı gerektirir.",
  "settings.quick": "Hızlı kilit açma",
  "settings.quick.body": "Ana parolanıza ek olarak içeri girmenin daha hızlı yolları.",
  "settings.quick.windowsHello": "Windows Hello",
  "settings.quick.touchId": "Touch ID",
  "settings.quick.biometric.body":
    "Yüzünüz, parmak iziniz veya PIN’inizle kilidi açın. Yeniden başlatmadan sonra ana parolanız yine gerekir.",
  "settings.quick.biometric.bodyMac":
    "Parmak izinizle kilidi açın. Yeniden başlatmadan sonra ana parolanız yine gerekir.",
  "settings.quick.expires": "Mosaic, {date, date, medium} tarihinde ana parolanızı yeniden isteyecek.",
  "settings.quick.maxAge": "Ana parolayı en az şu aralıkla iste",
  "settings.quick.maxAgeDays": "{days, plural, one {# gün} other {# gün}}",
  "settings.quick.biometricLinux": "Biyometrik kilit açma Linux’ta kullanılamıyor.",
  "settings.quick.biometricUnavailable": "Bu aygıtta kurulmadı.",
  "settings.quick.keys": "Güvenlik anahtarları",
  "settings.quick.keysCount":
    "{count, plural, =0 {Kayıtlı yok} one {# kayıtlı} other {# kayıtlı}}",
  "settings.quick.keysUnsupported": "Güvenlik anahtarları bu sistemde henüz desteklenmiyor.",
  "settings.quick.keyRow": "Güvenlik anahtarı · {tag} · eklenme {addedAt, date, medium}",
  "settings.quick.addKey": "Güvenlik anahtarı ekle",
  "settings.quick.addKeyBody":
    "Eklemek istediğiniz anahtarı takın ve PIN’ini girin. Sonra yanıp sönünce dokunun.",
  "settings.quick.manageKeys": "Yönet",
  "settings.quick.removeKey": "Kaldır",
  "settings.quick.removeKeyNamed": "{tag} güvenlik anahtarını kaldır",
  "settings.quick.removeKeyConfirm": "{tag} güvenlik anahtarı kaldırılsın mı?",
  "settings.quick.keyRisk":
    "Bu anahtara ve PIN’ine sahip olan herkes ana parolanız olmadan kasanızı açabilir.",
  "reauth.bodyQuick": "Bu, Mosaic’e girmeyi kolaylaştırır, bu yüzden ana parolanızı gerektirir.",
  "reauth.bodyKey": "Bu kasaya bir anahtar eklemek ana parolanızı gerektirir.",
  "settings.passkeyExport": "Geçiş anahtarlarını dışa aktar",
  "settings.passkeyExport.body":
    "Bir kasanın geçiş anahtarlarını, başka bir uygulamaya taşımak için bir FIDO CXF dosyasına kaydedin.",
  "settings.passkeyExport.vault": "Kasa",
  "settings.passkeyExport.warning":
    "Dosya geçiş anahtarlarınızı şifrelenmemiş olarak içerir. Dosyaya sahip olan herkes sizin adınıza oturum açabilir. İçe aktardıktan sonra silin.",
  "settings.passkeyExport.none": "Bu kasada dışa aktarılacak geçiş anahtarı yok.",
  "settings.passkeyExport.button": "Geçiş anahtarlarını dışa aktar",
  "settings.passkeyExport.reauth":
    "Dosya geçiş anahtarlarınızı şifrelenmemiş olarak içerecek, bu yüzden dışa aktarmak ana parolanızı gerektirir.",
  "settings.passkeyExport.saved":
    "{count, plural, one {# geçiş anahtarı dışa aktarıldı.} other {# geçiş anahtarı dışa aktarıldı.}} İçe aktardıktan sonra dosyayı silin.",
  "reauth.bodyMove":
    "Bir öğeyi güvenlik anahtarıyla korunan bir kasadan çıkarmak ana parolanızı gerektirir.",
  "toast.settingsReset":
    "Güvenlik ayarlarınız Mosaic dışında değiştirildi ve güvenli varsayılanlara sıfırlandı.",
  "settings.shortcut.title": "Hızlı arama kısayolu",
  "settings.shortcut.body": "Mosaic’in aramasını açmak için herhangi bir programda bu tuş birleşimine basın.",
  "settings.shortcut.label": "Kısayol",
  "settings.shortcut.hint":
    "Alana tıklayın, sonra istediğiniz tuşlara basın: Ctrl, Alt veya ⌘ (Mac’te) ve bir tuş daha.",
  "settings.shortcut.save": "Kısayolu kaydet",
  "settings.shortcut.reset": "Varsayılanı kullan",
  "settings.shortcut.saved": "Kısayol kaydedildi: {shortcut}.",
  "settings.shortcut.invalid":
    "Bu, Mosaic’in kullanabileceği bir kısayol değil. Ctrl, Alt veya ⌘ (Mac’te) ile birlikte bir tuş daha basın.",
  "settings.shortcut.unsupported":
    "Sisteminiz uygulamaların genel bir kısayol ayarlamasına izin vermiyor, bu yüzden burada değiştirilemez.",
} satisfies Record<keyof typeof en, string>;
