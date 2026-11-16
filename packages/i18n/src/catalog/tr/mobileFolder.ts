/**
 * Turkish UI strings: mobileFolder. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { mobileFolder as en } from "../en/mobileFolder.ts";

export const mobileFolder = {
  "mobile.sync.never": "Henüz eşitlenmedi",
  "mobile.sync.last": "Eşitlendi: {time}",
  "mobile.sync.localOnly": "Bu kasa yalnızca bu telefonda. Eşitleme kurulmadı.",
  "mobile.sync.pending":
    "{count, plural, one {Yüklenmeyi bekleyen # değişiklik} other {Yüklenmeyi bekleyen # değişiklik}}",
  "mobile.sync.now": "Şimdi eşitle",
  "mobile.sync.working": "Eşitleniyor",
  "mobile.sync.warnings": "Eşitleme uyarıları",
  "mobile.sync.warning.freezeSuspected":
    "Başka bir aygıttan gelen değişiklikler bir süredir ulaşmadı. Diğer aygıtın açık ve çevrimiçi olduğunu kontrol edin.",
  "mobile.sync.warning.noCommonView":
    "Bu telefon ve başka bir aygıt iki gündür aynı sunucu verisini görmedi. Sunucu onlara farklı sürümler gösteriyor olabilir. Diğer aygıtınızı eşitleyin, sonra burada eşitleyin.",
  "mobile.sync.warning.rollbackDetected":
    "Eşitleme verileriniz bu telefondan daha eski görünüyor. Mosaic’i başka bir aygıtta açın ve en son değişikliklerinizin orada olduğunu kontrol edin. Sonra “Kontrol ettim”i seçin.",
  "mobile.sync.warning.identityDuplicated":
    "Mosaic’in başka bir kopyası bu telefonun kimliğiyle yazıyor, örneğin bir yedek geri yüklendikten sonra. Eşitleme duraklatıldı. Yalnızca bir kopyanın kullanıldığından emin olun, sonra “Kontrol ettim”i seçin.",
  "mobile.sync.warning.serverInconsistent":
    "Sunucu bu telefona başka bir aygıttan farklı veriler gösterdi. Mosaic’i başka bir aygıtta açın ve en son değişikliklerinizin orada olduğunu kontrol edin. Sonra “Kontrol ettim”i seçin.",
  "mobile.sync.warning.serverRollback":
    "Sunucudaki kasalarınızdan birinin kopyası bu telefonunkinden daha eski. Mosaic’i başka bir aygıtta açın ve en son değişikliklerinizin orada olduğunu kontrol edin. Sonra “Kontrol ettim”i seçin.",
  "mobile.sync.warning.sharingAttention":
    "Paylaşılan bir kasanın ilgilenmeniz gerekiyor: bir anahtar değişikliği bekliyor veya bir üye denetlenemedi. Hangisi olduğunu görmek için Mosaic’i bilgisayarınızda açın. Sonra “Kontrol ettim”i seçin.",
  "mobile.sync.warning.undecryptable":
    "{count, plural, one {Alınan # öğe} other {Alınan # öğe}} bu telefonda okunamadı. Mosaic’i tüm aygıtlarınızda güncelleyin, sonra yeniden eşitleyin.",
  "mobile.sync.warning.accountRollback":
    "Sunucu bu telefondakinden daha eski bir hesap anahtarı gösteriyor. Hiçbir şey değiştirilmedi. Mosaic’i başka bir aygıtta açın ve en son değişikliklerinizin orada olduğunu kontrol edin. Sonra “Kontrol ettim”i seçin.",
  "mobile.sync.acknowledge": "Kontrol ettim…",
  "mobile.sync.acknowledgeTitle": "Eşitleme uyarıları temizlensin mi?",
  "mobile.sync.acknowledgeBody":
    "Yalnızca sunucuyu kendiniz kontrol ettikten sonra devam edin. Bu, tüm uyarıları temizler ve eşitleme devam eder. Verileriniz gerçekten geri alındıysa veya değiştirildiyse Mosaic bunu sonradan anlayamaz. Bunu her aygıtınızda yapın.",
  "mobile.sync.acknowledgeConfirm": "Kontrol ettim",
  "mobile.folder.intro": "Bu hesap {provider} klasörünüz üzerinden eşitlenir.",
  "mobile.folder.signIn": "{provider} hesabında oturum aç",
  "mobile.folder.scope":
    "{provider} tüm dosyalarınıza erişim isteyecek. Mosaic yalnızca seçtiğiniz klasörü açar ve başka hiçbir şeye dokunmaz.",
  "mobile.folder.waiting": "Oturum açmayı tarayıcınızda tamamlayın, sonra buraya dönün.",
  "mobile.folder.serverLabel": "Nextcloud veya WebDAV adresi",
  "mobile.folder.serverHint": "https://cloud.example.com",
  "mobile.folder.serverConfirm":
    "Mosaic {server} adresinde oturum açacak. Yalnızca bu kendi sunucunuzsa devam edin.",
  "mobile.folder.locating": "Mosaic klasörünüz aranıyor…",
  "mobile.folder.browseTitle": "Mosaic klasörünüzü seçin",
  "mobile.folder.browseHint":
    "Bilgisayarınızın eşitlediği, içinde mosaic-sync.json adlı bir dosya olan klasördür.",
  "mobile.folder.currentFolder": "Geçerli klasör: {path}",
  "mobile.folder.noSubfolders": "Burada klasör yok.",
  "mobile.folder.useThis": "Bu klasörü kullan",
  "mobile.folder.up": "Bir üst düzey",
  "mobile.folder.chooseProvider": "Hesabınız nerede eşitleniyor?",
  "mobile.folder.noDrive":
    "Google Drive ve iCloud Drive henüz Android’de çalışmıyor. Dropbox, OneDrive, Nextcloud veya Mosaic eşitlemesini kullanın.",
  "mobile.folder.useFolder": "Bunun yerine bir bulut klasörü üzerinden eşitle",
  "mobile.folder.providerDropbox": "Dropbox",
  "mobile.folder.providerOneDrive": "OneDrive",
  "mobile.folder.providerWebDav": "Nextcloud veya WebDAV",
  "mobile.folder.appPasswordToggle": "Bunun yerine bir uygulama parolası kullan",
  "mobile.folder.davLabel": "WebDAV klasör adresi",
  "mobile.folder.loginLabel": "Kullanıcı adı",
  "mobile.folder.appPasswordLabel": "Uygulama parolası",
  "mobile.folder.appPasswordHint":
    "Sunucunuzun güvenlik ayarlarında bir uygulama parolası oluşturun. Ana parolanızı kullanmayın.",
  "mobile.folder.appPasswordAction": "Uygulama parolasıyla oturum aç",
  "mobile.folder.pickAccount": "Bu telefon hangi hesaba katılıyor?",
  "mobile.folder.accountOption": "Hesap {id}",
  "mobile.folder.linked": "Oturum açıldı. Yeniden eşitleniyor.",
  "mobile.folder.settingsLine": "{provider} üzerinden eşitlenir: {path}",
  "mobile.folder.relink": "Yeniden oturum aç",
  "mobile.folder.msRevokeHint":
    "Mosaic’in erişimini tamamen kaldırmak için Microsoft hesap sayfanızda Uygulamalar altından kaldırın.",
  "mobile.reset.revokeDropbox":
    "Hesap kaldırıldı. Mosaic’in Dropbox’taki erişimi sonlandırılamadı: Dropbox’ta Ayarlar, Bağlı uygulamalar altından kaldırın.",
  "mobile.reset.revokeWebdav":
    "Hesap kaldırıldı. Sunucunuzdaki uygulama parolası silinemedi: orada Ayarlar, Güvenlik, Aygıtlar ve oturumlar altından kaldırın.",
  "mobile.reset.title": "Bu hesabı bu telefondan kaldır",
  "mobile.reset.body":
    "Bu, bu telefonda saklanan kasa kopyasını, Gizli Anahtarı ve bulut oturumunu siler. Bulut klasöründeki veya sunucudaki verileriniz kalır. Daha sonra Gizli Anahtarınız ve ana parolanızla yeniden katılabilirsiniz.",
  "mobile.reset.action": "Hesabı kaldır…",
  "mobile.reset.confirm": "Hesabı kaldır",
  "mobile.reset.phraseLabel": "Onay",
  "mobile.reset.typePhrase": "Onaylamak için {phrase} yazın",
} satisfies Record<keyof typeof en, string>;
