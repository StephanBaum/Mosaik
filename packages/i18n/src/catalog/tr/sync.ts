/**
 * Turkish UI strings: sync. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { sync as en } from "../en/sync.ts";

export const sync = {
  "sync.status.signedOut": "Oturum kapalı",
  "sync.platform.windows": "Windows",
  "sync.platform.macos": "macOS",
  "sync.platform.linux": "Linux",
  "settings.sync.title": "Eşitleme",
  "settings.sync.body":
    "Kasalarınızı tüm aygıtlarınızda güncel tutun. Her şey bu aygıttan çıkmadan önce şifrelenir.",
  "settings.sync.notSetUp": "Eşitleme kapalı. Kasalarınız yalnızca bu aygıtta saklanıyor.",
  "settings.sync.deviceName": "Bu aygıtın adı",
  "settings.sync.deviceName.default": "{platform} üzerinde Mosaic",
  "settings.sync.password": "Ana parola",
  "settings.sync.code": "6 haneli kod",
  "settings.sync.server.title": "Mosaic sunucusu",
  "settings.sync.server.body": "Sizin veya güvendiğiniz birinin işlettiği bir Mosaic sunucusu üzerinden eşitleyin.",
  "settings.sync.server.url": "Sunucu adresi",
  "settings.sync.server.urlHint": "Örneğin https://sync.example.com",
  "settings.sync.server.check": "Sunucuyu kontrol et",
  "settings.sync.server.checking": "Sunucu kontrol ediliyor…",
  "settings.sync.server.registration.open": "Bu sunucu herkesin bir eşitleme hesabı oluşturmasına izin veriyor.",
  "settings.sync.server.registration.invite":
    "Bu sunucuda eşitleme hesabı oluşturmak için bir davet kodu gerekiyor.",
  "settings.sync.server.registration.closed":
    "Bu sunucu yeni eşitleme hesabı kabul etmiyor. Yine de mevcut bir hesapta oturum açabilirsiniz.",
  "settings.sync.server.choice.label": "Eşitleme hesabı",
  "settings.sync.server.choice.signIn": "Bir eşitleme hesabım var",
  "settings.sync.server.choice.create": "Bir tane oluştur",
  "settings.sync.server.invite": "Davet kodu",
  "settings.sync.server.create": "Eşitleme hesabı oluştur",
  "settings.sync.server.signIn": "Oturum aç",
  "settings.sync.server.signInAgain": "Yeniden oturum aç",
  "settings.sync.server.working": "Bağlanılıyor…",
  "settings.sync.server.sees":
    "Sunucu yalnızca şifreli verileri, hesap e-postanızı ve ne zaman eşitlediğinizi görür. Ana parolanızı veya Gizli Anahtarınızı asla görmez.",
  "settings.sync.twoFactor.title": "Kodunuzu girin",
  "settings.sync.twoFactor.body":
    "Bu eşitleme hesabı iki adımlı doğrulama kullanıyor. Kimlik doğrulayıcı uygulamanızdaki 6 haneli kodu girin.",
  "settings.sync.twoFactor.submit": "Doğrula",
  "settings.sync.folder.title": "Kendi bulut klasörünüz",
  "settings.sync.folder.body":
    "Bulut hizmetinizin (Dropbox, OneDrive, iCloud Drive ve diğerleri) güncel tuttuğu bir klasör üzerinden eşitleyin.",
  "settings.sync.folder.choose": "Klasör seç…",
  "settings.sync.folder.chooseOther": "Başka bir klasör seç…",
  "settings.sync.folder.path": "Klasör",
  "settings.sync.folder.contents.empty": "Klasör boş. Mosaic onu kullanabilir.",
  "settings.sync.folder.contents.mosaic": "Bu klasörde zaten Mosaic eşitleme verisi var.",
  "settings.sync.folder.contents.other":
    "Bu klasörde başka dosyalar var. Mosaic oraya hiçbir şey yazmaz. Boş bir klasör seçin veya yeni bir tane oluşturun.",
  "settings.sync.folder.connect": "Klasörü bağla",
  "settings.sync.folder.working": "Bağlanılıyor…",
  "settings.sync.folder.sees":
    "Bulut sağlayıcınız ve klasöre erişimi olan herkes yalnızca şifreli verileri, hesap kimliğini ve dosyaların ne zaman değiştiğini görür. Ana parolanızı veya Gizli Anahtarınızı asla görmezler.",
  "settings.sync.status.label": "Durum",
  "settings.sync.lastSynced": "Son eşitleme: {when}",
  "settings.sync.neverSynced": "Henüz eşitlenmedi",
  "settings.sync.offline":
    "Çevrimdışısınız. Değişiklikleriniz bu aygıtta kayıtlı ve çevrimiçi olduğunuzda eşitlenecek.",
  "settings.sync.now": "Şimdi eşitle",
  "settings.sync.backend.server": "Sunucu",
  "settings.sync.backend.account": "Eşitleme hesabı",
  "settings.sync.backend.folder": "Klasör",
  "settings.sync.backend.device": "Bu aygıt",
  "settings.sync.signedOut.title": "Bu aygıtın oturumu kapatıldı",
  "settings.sync.signedOut.body":
    "Sunucu bu aygıtın oturumunu sonlandırdı. Eşitlemeye devam etmek için yeniden oturum açın. Değişiklikleriniz bu aygıtta güvende.",
  "settings.sync.recover.title": "Ana parolanız değiştirildi",
  "settings.sync.recover.body":
    "Sunucuda veya klasörde bu aygıttakinden daha yeni bir ana parola var. Bu aygıtı güncellemek için yeni ana parolayı girin. Kasalarınız ve Gizli Anahtarınız aynı kalır.",
  "settings.sync.recover.password": "Yeni ana parola",
  "settings.sync.recover.submit": "Bu aygıtı güncelle",
  "settings.sync.disconnect": "Bağlantıyı kes…",
  "settings.sync.disconnect.title": "Eşitlemenin bağlantısı kesilsin mi?",
  "settings.sync.disconnect.body":
    "Bu aygıt eşitlemeyi durdurur ve sunucudan çıkış yapar. Kasalarınız bu aygıtta kalır ve diğer aygıtlarınız kendi kopyalarını korur.",
  "settings.sync.disconnect.confirm": "Bağlantıyı kes",
  "settings.sync.devices.title": "Aygıtlar",
  "settings.sync.devices.thisDevice": "Bu aygıt",
  "settings.sync.devices.unnamed": "Adsız aygıt",
  "settings.sync.devices.lastSeen": "Son görülme: {when}",
  "settings.sync.devices.neverSeen": "Henüz görülmedi",
  "settings.sync.devices.rename": "Yeniden adlandır",
  "settings.sync.devices.renameNamed": "{name} aygıtını yeniden adlandır",
  "settings.sync.devices.renameLabel": "Bu aygıt için yeni ad",
  "settings.sync.devices.remove": "Kaldır",
  "settings.sync.devices.removeNamed": "{name} aygıtını kaldır",
  "settings.sync.devices.removeTitle": "{name} aygıtı kaldırılsın mı?",
  "settings.sync.devices.removeBody":
    "Bir aygıtı kaldırmak onun eşitleme oturumunu kapatır. Zaten sahip olduğu kasa kopyalarınızı almaz. Bir aygıtı kalıcı olarak dışarıda bırakmak için ana parolanızı ve Gizli Anahtarınızı değiştirin.",
  "settings.sync.devices.removeConfirm": "Aygıtı kaldır",
  "settings.sync.devices.folderNote":
    "Bir aygıtı dışarıda bırakmak için ana parolanızı ve Gizli Anahtarınızı değiştirin.",
  "settings.sync.phone.title": "Bir telefon kurun",
  "settings.sync.phone.body":
    "Telefonunuzun taradığı bir kod gösterin; böylece Gizli Anahtarınızı yazmanız gerekmez. Telefonun yine de ana parolanıza ihtiyacı var.",
  "settings.sync.phone.button": "Bir telefon kur",
  "settings.sync.phone.reauth":
    "Kod Gizli Anahtarınızı içerir, bu yüzden ana parolanızı gerektirir.",
  "settings.sync.phone.dialogTitle": "Bu kodu telefonunuzla tarayın",
  "settings.sync.phone.steps":
    "Telefonunuzda Mosaic’i açın, kurmayı seçin, sonra bu kodu tarayın.",
  "settings.sync.phone.hint":
    "Kod 60 saniye sonra veya bu pencereden ayrıldığınızda gizlenir. Fotoğrafını paylaşmayın.",
  "settings.sync.phone.canvas": "Telefonunuz için kurulum kodu",
  "settings.sync.phone.hidden.time": "60 saniye doldu, bu yüzden kod gizlendi.",
  "settings.sync.phone.hidden.away": "Bu pencereden ayrıldınız, bu yüzden kod gizlendi.",
  "settings.sync.phone.again": "Yeniden göster",
  "settings.sync.phone.folderHint":
    "Telefonunuz bulut depolamanızda oturum açar ve aynı klasörü açar.",
  "settings.sync.totp.title": "Kimlik doğrulayıcı uygulaması",
  "settings.sync.totp.body":
    "Biri eşitleme hesabınızda oturum açtığında kimlik doğrulayıcı uygulamasından bir kod ister. Sunucudaki hesabı korur, kasalarınızı değil: onlar ana parolanız ve Gizli Anahtarınızla şifreli kalır.",
  "settings.sync.totp.on": "Kimlik doğrulayıcı uygulama kodları açık.",
  "settings.sync.totp.off": "Kimlik doğrulayıcı uygulama kodları kapalı.",
  "settings.sync.totp.enable": "Aç…",
  "settings.sync.totp.disable": "Kapat…",
  "settings.sync.totp.beginTitle": "Kimlik doğrulayıcı uygulama kodlarını açın",
  "settings.sync.totp.beginBody": "Başlamak için ana parolanızı girin.",
  "settings.sync.totp.begin": "Devam",
  "settings.sync.totp.setupBody":
    "Bu anahtarı kimlik doğrulayıcı uygulamanıza ekleyin, sonra gösterdiği 6 haneli kodu girin.",
  "settings.sync.totp.secret": "Kurulum anahtarı",
  "settings.sync.totp.confirm": "Aç",
  "settings.sync.totp.disableTitle": "Kimlik doğrulayıcı uygulama kodlarını kapatın",
  "settings.sync.totp.disableBody": "Ana parolanızı ve geçerli bir kodu girin.",
  "settings.sync.totp.disableConfirm": "Kapat",
  "settings.sync.totp.enabledToast": "Kimlik doğrulayıcı uygulama kodları açık.",
  "settings.sync.totp.disabledToast": "Kimlik doğrulayıcı uygulama kodları kapalı.",
  "settings.sync.key.title": "Güvenlik anahtarları",
  "settings.sync.key.body":
    "Bir güvenlik anahtarı, eşitleme hesabınızda oturum açarken ikinci adım olabilir. Sunucudaki hesabı korur, kasalarınızı değil. Bir anahtar eklemek veya kullanmak tarayıcınızı açar.",
  "settings.sync.key.none": "Eklenmiş güvenlik anahtarı yok.",
  "settings.sync.key.added": "Eklenme: {when}",
  "settings.sync.key.lastUsed": "Son kullanım: {when}",
  "settings.sync.key.neverUsed": "Henüz kullanılmadı",
  "settings.sync.key.add": "Güvenlik anahtarı ekle…",
  "settings.sync.key.addTitle": "Bir güvenlik anahtarı ekleyin",
  "settings.sync.key.addBody":
    "Anahtara bir ad verin ve ana parolanızı girin. Mosaic sonra anahtarı kullanacağınız tarayıcınızı açar.",
  "settings.sync.key.label": "Anahtarın adı",
  "settings.sync.key.labelHint": "Örneğin “Masa anahtarı”. En fazla 64 karakter.",
  "settings.sync.key.addConfirm": "Devam",
  "settings.sync.key.addedToast": "Güvenlik anahtarı eklendi.",
  "settings.sync.key.noBackup":
    "Bir güvenlik anahtarı tek ikinci adımınız olur. Kaybederseniz eşitleme hesabınızda bir daha oturum açamayabilirsiniz. Bir yedek öneririz: ikinci bir anahtar veya bir kimlik doğrulayıcı uygulama kodu.",
  "settings.sync.key.recommend":
    "İki yöntem öneririz, örneğin iki anahtar veya bir anahtar ile bir kimlik doğrulayıcı uygulama kodu.",
  "settings.sync.key.remove": "Kaldır",
  "settings.sync.key.removeLabel": "{name} anahtarını kaldır",
  "settings.sync.key.removeTitle": "{name} anahtarı kaldırılsın mı?",
  "settings.sync.key.removeBody":
    "Tarayıcınızda bir güvenlik anahtarıyla bir kez onaylarsınız. Ondan sonra bu anahtar oturum açmak için çalışmaz.",
  "settings.sync.key.removeLast":
    "Bu, kalan son ikinci adım yönteminiz. İki adımlı doğrulama kapanır ve eşitleme hesabınız yalnızca ana parolanız ve Gizli Anahtarınızla korunur.",
  "settings.sync.key.removeCode": "Kimlik doğrulayıcı uygulama kodu",
  "settings.sync.key.removeCodeHint":
    "Anahtarı hemen kaldırmak için geçerli bir kod girin. Bunun yerine tarayıcınızda bir güvenlik anahtarıyla onaylamak için boş bırakın.",
  "settings.sync.key.removeConfirm": "Anahtarı kaldır",
  "settings.sync.key.removedToast": "Güvenlik anahtarı kaldırıldı.",
  "settings.sync.key.confirmTitle": "Bir güvenlik anahtarıyla onaylayın",
  "settings.sync.key.confirmBody":
    "Bir anahtarı kaldırmak ikinci bir adım gerektirir. Ana parolanızı girin, sonra tarayıcınızda bir anahtarla onaylayın.",
  "settings.sync.key.step.methods": "Bu hesabın nasıl oturum açtığı kontrol ediliyor…",
  "settings.sync.key.step.useKey": "Güvenlik anahtarını kullan (tarayıcınızı açar)",
  "settings.sync.key.step.keyOnlyTitle": "Güvenlik anahtarınızı kullanın",
  "settings.sync.key.step.keyOnlyBody":
    "Bu hesap bir güvenlik anahtarıyla korunuyor. Mosaic, onu kullanacağınız bir sayfayı tarayıcınızda açar.",
  "settings.sync.key.step.or": "Veya bir kod kullanın",
  "settings.sync.key.wait.login": "Güvenlik anahtarınızla oturum açın",
  "settings.sync.key.wait.register": "Güvenlik anahtarınızı ekleyin",
  "settings.sync.key.wait.confirm": "Güvenlik anahtarınızla onaylayın",
  "settings.sync.key.wait.lead":
    "Tarayıcınız eşitleme sunucunuzdan bir Mosaic sayfası açtı. Bu kodu orada yazın, sonra güvenlik anahtarınızı kullanın.",
  "settings.sync.key.wait.codeLabel": "Tarayıcınıza yazılacak kod",
  "settings.sync.key.wait.codeSpoken": "Tarayıcınıza yazılacak kod: {digits}",
  "settings.sync.key.wait.expires": "Bu adım {time} zamanında sona erer.",
  "settings.sync.key.wait.waiting": "Güvenlik anahtarınız bekleniyor…",
  "settings.sync.key.wait.done": "Bitti.",
  "settings.sync.key.wait.expired": "Bu adımın süresi doldu. Yeniden başlatın.",
  "settings.sync.key.wait.failed":
    "Adım başarısız oldu: kod çok sık yanlış girildi veya güvenlik anahtarı kabul edilmedi. Hiçbir şey değiştirilmedi. Yeniden başlayın.",
  "settings.sync.key.wait.reopen": "Tarayıcı sayfasını yeniden aç",
  "settings.sync.key.wait.retry": "Yeniden başla",
  "sync.warning.region": "Eşitleme uyarıları",
  "sync.warning.unknownDevice": "(bilinmeyen)",
  "sync.warning.unknownVault": "(bilinmeyen)",
  "sync.warning.freezeSuspected":
    "{device} aygıtından gelen değişiklikler bir günden uzun süredir ulaşmadı. Diğer aygıtın açık ve çevrimiçi olduğunu kontrol edin.",
  "sync.warning.missingFiles":
    "{device} aygıtından gelen dosyalar klasörde eksik. Bulut hizmetinizin eşitlemeyi bitirmesine izin verin, sonra yeniden eşitleyin.",
  "sync.warning.noCommonView":
    "Bu aygıt ve {device} aygıtı iki gündür aynı sunucu verisini görmedi. Sunucu onlara farklı sürümler gösteriyor olabilir.",
  "sync.warning.rollbackDetected":
    "Eşitleme verileriniz bu aygıttan daha eski görünüyor. Sunucuyu veya klasörü kontrol edin, sonra “Kontrol ettim”i seçin.",
  "sync.warning.identityDuplicated":
    "Mosaic’in başka bir kopyası bu aygıtın kimliğiyle yazıyor, örneğin bir yedek geri yüklendikten sonra. Eşitleme duraklatıldı. Yalnızca bir kopyanın kullanıldığından emin olun, sonra “Kontrol ettim”i seçin.",
  "sync.warning.serverInconsistent":
    "Sunucu bu aygıta başka bir aygıttan farklı veriler gösterdi. Sunucuyu kontrol edin, sonra “Kontrol ettim”i seçin.",
  "sync.warning.serverRollback":
    "Sunucudaki {vault} kasasının kopyası bu aygıtınkinden daha eski. Sunucuyu kontrol edin, sonra “Kontrol ettim”i seçin.",
  "sync.warning.undecryptable":
    "{count, plural, one {Alınan # öğe} other {Alınan # öğe}} bu aygıtta okunamadı. Mosaic’i tüm aygıtlarınızda güncelleyin, sonra yeniden eşitleyin.",
  "sync.warning.hardwareVaultsLocal":
    "Donanımla korunan kasalar bu aygıtta kalır. {count, plural, one {# kasa eşitlenmiyor.} other {# kasa eşitlenmiyor.}}",
  "sync.warning.attachmentsNotRepaired":
    "Sunucudaki veya klasördeki {count, plural, one {# dosya} other {# dosya}} bu aygıttan onarılamadı. Dosyaya hâlâ sahip olan bir aygıtta öğeyi açın ve yeniden ekleyin.",
  "sync.warning.acknowledge": "Kontrol ettim…",
  "sync.acknowledge.title": "Kontrol gerektiren eşitleme uyarıları temizlensin mi?",
  "sync.acknowledge.body":
    "Yalnızca sunucuyu veya klasörü kendiniz kontrol ettikten sonra devam edin. Bu, kontrol gerektiren her uyarıyı temizler ve eşitleme devam eder. Verileriniz gerçekten geri alındıysa veya değiştirildiyse Mosaic bunu sonradan anlayamaz.",
  "sync.acknowledge.eachDevice": "Bunu her aygıtınızda yapın.",
  "toast.syncAttention":
    "Eşitleme, başka bir aygıttan bazı öğeleri birleştirdi veya onardı. Neyin değiştiğine bir göz atın.",
  "sync.revoked.rekey":
    "Bu hesabın anahtarları başka bir aygıtta yenilendi: en yeni Acil Durum Kitinizdeki yeni Gizli Anahtar ile oturum açın.",
  "sync.revoked.passwordChange":
    "Ana parolanız başka bir aygıtta değiştirildi: yeni parolayla oturum açın.",
  "sync.revoked.deviceRemoved":
    "Bu aygıt hesabınızdan başka bir aygıtta kaldırıldı. Geri eklemek için yeniden oturum açın.",
  "sync.revoked.operator":
    "Sunucunun işletmecisi bu oturumu sonlandırdı. Eşitlemeye devam etmek için yeniden oturum açın.",
} satisfies Record<keyof typeof en, string>;
