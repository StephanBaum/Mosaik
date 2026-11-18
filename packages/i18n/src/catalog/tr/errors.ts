/**
 * Turkish UI strings: errors. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { errors as en } from "../en/errors.ts";

export const errors = {
  "error.notImplemented": "Bu henüz kullanılamıyor. Mosaic’in sonraki bir sürümünde gelecek.",
  "error.noAccount":
    "Bu aygıtta henüz hesap yok. Bir hesap oluşturun veya Acil Durum Kitinizle oturum açın.",
  "error.accountExists": "Bu aygıtta zaten bir Mosaic hesabı var. Bunun yerine kilidini açın.",
  "error.locked": "Mosaic kilitli. Devam etmek için kilidini açın.",
  "error.alreadyUnlocked": "Mosaic’in kilidi zaten açık. Kaldığınız yerden devam edebilirsiniz.",
  "error.wrongCredentials":
    "Bu ana parola veya Gizli Anahtar çalışmadı. Caps Lock’u ve klavye düzeninizi kontrol edin, sonra yeniden deneyin.",
  "error.secretKeyRequired":
    "Bu aygıtta Gizli Anahtarınız yok. Acil Durum Kitinizden yazın.",
  "error.secretKeyMalformed":
    "Bu bir Gizli Anahtar değil. Sizinki “M1-” ile başlar ve Acil Durum Kitinizdedir.",
  "error.secretKeyMistyped":
    "Bu Gizli Anahtarda bir yazım hatası var. Acil Durum Kitinizle karakter karakter karşılaştırın.",
  "error.emptyPassword": "Ana parola boş. Devam etmek için girin.",
  "error.quickUnlockUnavailable":
    "Hızlı kilit açma bu aygıtta kurulu değil. Ana parolanızla kilidi açın.",
  "error.storeInUse":
    "Mosaic bu bilgisayarda zaten açık. Diğer pencereyi kapatın, sonra yeniden deneyin.",
  "error.storeUnavailable":
    "Mosaic bu aygıttaki kasa dosyanızı açamadı. Diskin kullanılabilir olduğunu ve boş alanı bulunduğunu kontrol edin, sonra Mosaic’i yeniden başlatın.",
  "error.vaultNotFound": "Bu kasa artık yok. Başka bir aygıtta silinmiş olabilir.",
  "error.vaultUnavailable":
    "Bu kasa bu oturumda açılamıyor. Donanım anahtarıyla kilidini açın veya eşitleyip yeniden deneyin.",
  "error.itemNotFound":
    "Bu öğe artık yok. Başka bir aygıtta taşınmış veya silinmiş olabilir.",
  "error.itemDeleted":
    "Bu öğe kalıcı olarak silindi, bu yüzden açılamaz. Başka bir öğe seçin.",
  "error.sameVault": "Öğe zaten bu kasada. Farklı bir kasa seçin.",
  "error.noOtp": "Bu öğede tek kullanımlık kod yok. Eklemek için öğeyi düzenleyin.",
  "error.fieldEmpty":
    "Bu alan boş, bu yüzden kopyalanacak veya gösterilecek bir şey yok. Doldurmak için öğeyi düzenleyin.",
  "error.invalidInput":
    "Bu formdaki bir şey geçerli değil. Vurgulanan alanları kontrol edin ve yeniden deneyin.",
  "error.sshKeyInvalid":
    "Bu bir OpenSSH özel anahtarı değil. BEGIN ve END satırları dahil anahtarın tamamını yapıştırın.",
  "error.reauthRequired": "Bu değişiklik ana parolanızı gerektirir. Onaylamak için girin.",
  "error.attachmentTooLarge": "Bu dosya 50 MiB’den büyük. Daha küçük bir dosya seçin.",
  "error.attachmentQuota":
    "Dosyalarınız mevcut 1 GiB’den fazla yer kaplayacak. Bazı ekleri kaldırın, sonra yeniden deneyin.",
  "error.attachmentNotFound": "Bu dosya artık öğenin parçası değil. Öğeyi yeniden yükleyin.",
  "error.attachmentUnavailable":
    "Bu dosya bu aygıtta açılamıyor. Henüz eşitlenmemiş veya hasarlı olabilir.",
  "error.attachmentFileUnreadable": "Bu dosya okunamıyor. Başka bir dosya seçin.",
  "error.attachmentSaveFailed": "Dosya oraya kaydedilemedi. Başka bir klasör seçin.",
  "error.attachmentPreviewUnsupported": "Bu dosya için önizleme yok. Açmak için Farklı kaydet’i kullanın.",
  "error.attachmentPreviewTooLarge": "Bu görsel önizleme için çok büyük. Açmak için Farklı kaydet’i kullanın.",
  "error.clipboardUnavailable":
    "Mosaic panoyu kullanamadı. Panoyu tutuyor olabilecek uygulamaları kapatın, sonra yeniden deneyin.",
  "error.importFileUnreadable":
    "Mosaic bu dosyayı okuyamadı. Hâlâ var olduğunu ve başka bir uygulamada açık olmadığını kontrol edin, sonra yeniden deneyin.",
  "error.importDeleteNotAllowed":
    "Önce dosyayı içe aktarın. Mosaic bir dışa aktarmayı yalnızca öğeleri kasanıza girdikten sonra siler.",
  "error.importDeleteFailed":
    "Dosya silinemedi. Başka bir uygulamada açık veya salt okunur olabilir. Kendiniz silin, sonra geri dönüşüm kutusunu boşaltın.",
  "error.importFormatInvalid":
    "Bu dosya seçtiğiniz biçimde değil. Uygun biçimi seçin veya dosyayı yeniden dışa aktarın.",
  "error.importPreviewExpired": "Bu içe aktarma iptal edildi. Baştan başlamak için dosyayı yeniden seçin.",
  "error.sshRequestExpired":
    "Bu SSH isteği zaten sona erdi. Hâlâ gerekiyorsa komutu yeniden çalıştırın.",
  "error.sshAgentUnavailable":
    "SSH aracısı başlatılamadı. Başka bir SSH aracısının çalışmadığını kontrol edin, sonra yeniden açın.",
  "error.unlockDelayed": "Art arda çok fazla yanlış deneme. Birkaç saniye bekleyin, sonra yeniden deneyin.",
  "error.tooManyAttempts":
    "Çok fazla yanlış yanıt. Mosaic kilitlendi. Devam etmek için ana parolanızla kilidi açın.",
  "error.sshAllowTooSoon":
    "İsteği okumak için bu çok hızlıydı. Anahtarı ve programı kontrol edin, sonra yeniden İzin ver’i seçin.",
  "error.browserRequestExpired":
    "Bu tarayıcı isteği zaten sona erdi. Hâlâ gerekiyorsa uzantıdan yeniden isteyin.",
  "error.browserAllowTooSoon":
    "İsteği okumak için bu çok hızlıydı. Ayrıntıları kontrol edin, sonra yeniden seçin.",
  "error.browserPairingNotFound":
    "Bu tarayıcı artık bağlı değil. Güncel listeyi görmek için Ayarlar’ı yeniden açın.",
  "error.browserHostMissing":
    "Mosaic’in bu kopyasında yüklü bir tarayıcı bağlayıcısı yok, bu yüzden onarılacak bir şey yok. Bir tarayıcıyı bağlamak için Mosaic’i yükleyicisiyle yükleyin.",
  "error.agentOriginInvalid":
    "Aracılar bir anahtarı yalnızca https:// ile başlayan ve sadece siteyi belirten bir siteye, örneğin https://api.example.com adresine gönderebilir. Adresi kontrol edin.",
  "error.agentBindingInvalid":
    "Mosaic bu ayarı aracılar için kullanamıyor. Alanı, siteleri ve anahtarın nasıl gönderildiğini kontrol edin.",
  "error.agentValueUnusable":
    "Bu değer çok kısa veya içinde satır sonu var, bu yüzden aracılar onu kullanamaz. Bir anahtar tek satırda en az 12 karakter olmalıdır.",
  "error.agentVaultShared":
    "Aracılar yalnızca size ait olan ve paylaşılmayan bir kasadaki öğeleri kullanabilir. Önce öğeyi kendi kasalarınızdan birine taşıyın.",
  "error.agentAccessOff": "Aracı erişimi kapalı. Ayarlar’da Aracılar altından açın.",
  "error.agentRequestExpired":
    "Bu aracı isteği zaten sona erdi. Hâlâ gerekiyorsa aracı yeniden isteyebilir.",
  "error.agentAllowTooSoon":
    "İsteği okumak için bu çok hızlıydı. Ayrıntıları kontrol edin, sonra yeniden seçin.",
  "error.agentPairingNotFound":
    "Bu aracı artık bağlı değil. Güncel listeyi görmek için Ayarlar’ı yeniden açın.",
  "error.agentEndpointUnavailable":
    "Mosaic aracıların kullandığı bağlantıyı açamadı. Mosaic’in diğer kopyalarını kapatın ve yeniden deneyin.",
  "error.kitNotVerified":
    "Önce yeni Acil Durum Kitini kaydedin ve son grubunu onaylayın. Mosaic bundan önce hiçbir şeyi değiştirmez.",
  "error.kitGroupMismatch":
    "Bu, yeni Gizli Anahtarın son grubu değil. Yeni Acil Durum Kitini kontrol edin.",
  "error.rotationInProgress":
    "Başka bir anahtar değişikliği hâlâ açık. Anahtar yenilemeye devam et ile bitirin veya önce ondan vazgeçin.",
  "error.rotationNone": "Devam edilecek bir anahtar değişikliği yok.",
  "error.rotationNotFound":
    "Bu parola değişikliği artık açık değil. Başka bir aygıtta veya tarayıcıda kapatılmış olabilir. Durumunu görmek için öğeyi yeniden açın.",
  "error.rotationLimit":
    "Bu öğenin dört parola değişikliği zaten açık. Önce öğede birini bitirin veya kapatın.",
  "error.rotationOpen":
    "Bu girişin açık bir parola değişikliği var. Onu Çöp Kutusu’ndan geri yükleyin, sonra öğede Çalıştı veya Çalışmadı’yı seçin. Ondan sonra kalıcı olarak silebilirsiniz.",
  "error.rotationNotSupported":
    "Mosaic bu değişiklikte rehberlik edemiyor. https adresi olan bir giriş gerekir. Parolayı sitede kendiniz değiştirin.",
  "error.historyNotFound":
    "Bu önceki parola artık geçmişte değil. Güncel listeyi görmek için öğeyi yeniden açın.",
  "error.originNotRelated":
    "Bu sayfa bu girişin sitesine ait değil, bu yüzden Mosaic onu eklemedi. Olması gerekiyorsa öğenin adreslerini düzenleyin.",
  "error.breachConsentRequired":
    "Veri ihlali kontrolü kapalı. Önce açın ve gönderdiklerini kabul edin.",
  "error.breachRunning":
    "Bir veri ihlali kontrolü zaten çalışıyor. Bitmesini bekleyin veya önce durdurun.",
  "error.rotationTooFar":
    "Bu değişiklik hesabınıza zaten ulaştı, bu yüzden silinemez. Bitirmek için devam edin.",
  "error.rotationJumpTooLarge":
    "Bu aygıt diğer aygıtlara yetişemeyecek kadar geride. Yeniden kurun: hesabı bu aygıttan kaldırın ve Acil Durum Kitinizle yeniden katılın.",
  "error.rotationNotApplied":
    "Değişiklik hesabınıza hiç ulaşmadı, bu yüzden hiçbir şey değişmedi. Yeniden başlayın veya değişiklikten vazgeçin.",
  "error.secretKeyUnchanged":
    "Bu Gizli Anahtar artık çalışmıyor. Değişikliği yapan aygıtta kaydettiğiniz Acil Durum Kitindekini kullanın.",
  "error.passwordUnchanged": "Yeni ana parola mevcut olandan farklı olmalıdır.",
  "error.profileDowngrade":
    "Bu düzey hesabınızın şu an kullandığından daha güçlü değil. Daha yüksek birini seçin.",
  "error.profileTooSlow":
    "Bu düzeyle kilit açmak bu aygıtta iki saniyeden uzun sürerdi. Daha düşük bir düzey seçin veya yine de yükseltin.",
  "error.rekeyNeedsCleanSync":
    "Yeniden anahtarlama için eşitlemenin güncel ve uyarısız olması gerekir. Şimdi eşitleyin, uyarılara bakın, sonra yeniden deneyin.",
  "error.rekeyMemberKeyMismatch":
    "Paylaşılan bir kasadaki herkesin güvenlik kodunu henüz karşılaştırmadınız, bu yüzden o kasa yenilenemez. Kasanın paylaşım ayarlarında kodlarını karşılaştırın, sonra yeniden deneyin.",
  "error.rekeyBusy": "Mosaic çalışırken kasalarınız değişmeye devam etti. Biraz bekleyin ve yeniden deneyin.",
  "error.rekeyOutcomeUnknown":
    "Mosaic hesabınızın yeni anahtarları kabul edip etmediğini doğrulayamadı. Öğrenmek için Anahtar yenilemeye devam et’i seçin.",
  "error.rekeyNeedsServer":
    "Yeniden anahtarlama eşitleme sunucunuzu veya klasörünüzü gerektirir ve ona ulaşılamıyor. Bağlantıyı kontrol edin ve yeniden deneyin.",
  "error.rekeyNoAccountFile":
    "Eşitleme klasörünüzdeki hiçbir şey bu parola ve Gizli Anahtarla açılmıyor. Yeni Acil Durum Kitini kontrol edin.",
  "error.syncAccountRekeyed":
    "Hesabınızın anahtarları başka bir aygıtta yenilendi. Devam etmek için yeni Acil Durum Kitindeki Gizli Anahtarı kullanın.",
  "error.syncEpochRollback":
    "Eşitleme arka ucunuz bu aygıttakinden daha eski hesap anahtarları gösteriyor. Hiçbir şey değiştirilmedi.",
  "error.syncRekeyRefused":
    "Eşitleme arka ucunuzun sunduğu anahtarlar bu aygıttakilerin devamı değil. Hiçbir şey değiştirilmedi. Diğer aygıtlarınızı kontrol edin.",
  "error.pickerStale": "Bu liste güncel değil. Alana yeniden tıklayın.",
  "error.pickerTooSoon": "Biraz bekleyin, sonra yeniden onaylayın.",
  "error.programFillUnavailable": "Programlarda doldurma bu aygıtta kullanılamıyor.",
  "error.busy": "Mosaic hâlâ son adımı bitiriyor. Biraz bekleyin, sonra yeniden deneyin.",
  "error.internal":
    "Beklenmeyen bir şey bu işlemi durdurdu. Yeniden deneyin; tekrarlanırsa Mosaic’i yeniden başlatın.",
  "error.importTooLarge":
    "Bu dosya içe aktarmak için çok büyük veya çok karmaşık. Her seferinde daha az öğe dışa aktarın, sonra yeniden deneyin.",
  "error.importUnsupported":
    "Mosaic biçimin bu sürümünü okuyamıyor. Güncel bir biçimde yeniden dışa aktarın (KeePass için: AES veya ChaCha20 ile KDBX 4), sonra yeniden deneyin.",
  "error.importEncrypted":
    "Bu dışa aktarma parolayla korunuyor. Şifrelemesiz yeniden dışa aktarın, içe aktarın, sonra dosyayı silin.",
  "error.sshAgentEndpointInUse":
    "Başka bir SSH aracısı zaten çalışıyor (Windows’ta OpenSSH Authentication Agent hizmeti). Onu durdurun, sonra Mosaic’in SSH aracısını yeniden açın.",
  "error.cancelled": "İptal edildi. Hiçbir şey değişmedi.",
  "error.quickUnlockExpired":
    "Hızlı kilit açmanın süresi doldu. Ana parolanızla kilidi açın, sonra Ayarlar’dan yeniden açın.",
  "error.quickUnlockInvalidated":
    "Hızlı kilit açma bu aygıtta çalışmayı bıraktı ve kapatıldı. Ana parolanızla kilidi açın.",
  "error.passwordRequired":
    "Bu sefer ana parolanızı girin. Hızlı kilit açma bundan sonra yeniden çalışır.",
  "error.hardwareKeyUnsupported": "Bu güvenlik anahtarı veya sistem Mosaic ile kullanılamaz.",
  "error.hardwareKeyNotFound": "Güvenlik anahtarı bulunamadı. Takın ve yeniden deneyin.",
  "error.hardwareKeyTimeout": "Anahtara zamanında dokunulmadı. Yeniden deneyin.",
  "error.hardwareKeyPinRequired": "Güvenlik anahtarınızın PIN’ini girin.",
  "error.hardwareKeyPinInvalid": "Yanlış PIN. Birkaç yanlış denemeden sonra anahtar kendini kilitler.",
  "error.hardwareKeyPinBlocked":
    "Bu anahtar çok fazla yanlış PIN nedeniyle kilitlendi. Üreticisinin aracıyla sıfırlayın; kayıtları bu durumda kaybolur.",
  "error.hardwareKeyNoPin": "Önce bu anahtarda üreticisinin aracıyla bir PIN belirleyin.",
  "error.hardwareKeyNotRecognized":
    "Bu anahtar burada kayıtlı değil. Başka bir anahtar veya ana parolanızı deneyin.",
  "error.hardwareKeyLimit": "İzin verilen en fazla anahtarı (8) kaydettiniz. Önce birini kaldırın.",
  "error.hardwareKeyLast":
    "Bu, kasanın tek anahtarı. Kaldırmadan önce bir tane daha kaydedin.",
  "error.itemTypeNotAllowed": "Bu kasa yalnızca tek kullanımlık kodları tutar.",
  "error.otpScanUnsupported":
    "Ekran tarama bu sistemde kullanılamıyor. Bunun yerine bir ekran görüntüsü yapıştırın.",
  "error.otpQrNotFound": "QR kodu bulunamadı. Kodun tamamının görünür olduğundan emin olun, sonra yeniden deneyin.",
  "error.otpQrNotOtp":
    "Bu QR kodu bir iki adımlı doğrulama kurulum kodu değil. Kodu sitenin 2FA kurulum sayfasında arayın.",
  "error.otpQrMigration":
    "Bu, Mosaic’in henüz okuyamadığı bir Google Authenticator dışa aktarma kodu. Her hesabı kendi sitesinde yeniden kurun veya kurulum anahtarını girin.",
  "error.otpQrMultiple":
    "Birden fazla kurulum kodu görünüyor. Yalnızca istediğinizi gösterin, sonra yeniden deneyin.",
  "error.otpQrInvalid":
    "Bu kurulum kodu Mosaic’in desteklemediği ayarlar kullanıyor. Bunun yerine kurulum anahtarını girin.",
  "error.otpUriInvalid": "Bu geçerli bir otpauth:// bağlantısı değil. Bağlantının tamamını yeniden kopyalayın.",
  "error.otpKeyInvalid":
    "Bu anahtar geçerli değil. Yalnızca A’dan Z’ye harfleri ve 2’den 7’ye rakamları kullanır; yazım hatalarını kontrol edin.",
  "error.otpImageUnreadable": "Mosaic bu görseli okuyamıyor. Bir PNG veya JPEG dosyası kullanın.",
  "error.otpImageTooLarge": "Bu görsel çok büyük. Yalnızca QR kodunun ekran görüntüsünü kullanın.",
  "error.otpClipboardNoImage":
    "Panoda görsel yok. QR kodunun ekran görüntüsünü alın, sonra buraya yapıştırın.",
  "error.otpPendingExpired": "Taranan kod artık kullanılamıyor. Yeniden tarayın.",
  "error.syncNotConfigured": "Eşitleme bu aygıtta henüz kurulmadı. Önce Ayarlar’dan kurun.",
  "error.syncAlreadyConfigured":
    "Eşitleme zaten kurulu. Farklı bir sunucu veya klasör kullanmak için önce bağlantıyı kesin.",
  "error.syncServerUnreachable":
    "Mosaic sunucuya ulaşamadı. Adresi ve internet bağlantınızı kontrol edin, sonra yeniden deneyin.",
  "error.syncServerInvalid":
    "Bu bir Mosaic sunucusuna benzemiyor. Adresi kontrol edin; https://sync.example.com gibi görünmelidir.",
  "error.syncServerInsecure":
    "Mosaic sunuculara yalnızca https üzerinden bağlanır; böylece yoldaki kimse verilerinizi okuyamaz veya değiştiremez. https:// ile başlayan bir adres kullanın.",
  "error.syncServerMismatch": "Bu, hesabın eşitlendiği sunucu değil. Adresi kontrol edin.",
  "error.syncServerIncompatible":
    "Bu sunucu Mosaic’inizle uyuşmayan bir sürüm çalıştırıyor. Mosaic’i güncelleyin veya sunucunun sahibinden güncellemesini isteyin.",
  "error.syncRegistrationClosed":
    "Bu sunucu yeni eşitleme hesabı kabul etmiyor. Mevcut bir hesapta oturum açın veya sunucunun sahibine sorun.",
  "error.syncInviteInvalid":
    "Bu davet kodu geçerli değil veya zaten kullanılmış. Kontrol edin veya sunucunun sahibinden yenisini isteyin.",
  "error.syncAccountExists":
    "Bu sunucuda zaten bir eşitleme hesabınız var. Bunun yerine Oturum aç’ı seçin.",
  "error.syncAccountMismatch":
    "O sunucudaki eşitleme hesabı farklı bir Mosaic hesabına ait. Adresi ve e-postanızı kontrol edin.",
  "error.syncTwoFactorRequired":
    "Bu eşitleme hesabı kimlik doğrulayıcı uygulamanızdan bir kod gerektirir. 6 haneli kodu girin.",
  "error.syncTwoFactorInvalid":
    "Bu kod doğru değil veya süresi doldu. Kimlik doğrulayıcı uygulamanızdaki güncel 6 haneli kodu girin.",
  "error.syncSignedOut":
    "Sunucu bu aygıtın oturumunu kapattı. Eşitlemeye devam etmek için yeniden oturum açın. Değişiklikleriniz bu aygıtta güvende.",
  "error.syncRateLimited": "Çok fazla deneme. Birkaç dakika bekleyin, sonra yeniden deneyin.",
  "error.syncQuotaExceeded":
    "Sunucuda veya klasörde verileriniz için yer kalmadı. Yer açın veya sunucunun sahibinden daha fazlasını isteyin.",
  "error.syncFolderUnavailable":
    "Mosaic bu klasörü kullanamıyor. Var olduğunu, bulut hizmetinizin eşitlemeyi bitirdiğini ve Mosaic’in ona yazabildiğini kontrol edin.",
  "error.syncFolderNotEmpty":
    "Bu klasörde başka dosyalar var, bu yüzden Mosaic oraya yazmaz. Boş bir klasör seçin veya yeni bir tane oluşturun.",
  "error.syncFolderNoAccount":
    "Bu klasörde bu hesap için eşitleme verisi yok. Diğer aygıtınızın eşitlediği klasörü seçin.",
  "error.syncFolderDamaged":
    "Bu klasördeki bazı eşitleme dosyaları hasarlı. Bulut hizmetinizin eşitlemeyi bitirmesine izin verin veya klasörü bir yedekten geri yükleyin, sonra yeniden deneyin.",
  "error.syncPasswordOutcomeUnknown":
    "Sunucunun ana parolanızı değiştirip değiştirmediğini doğrulayamadık. Eşitleme hangisinin geçerli olduğunu doğrulayana kadar her iki parolayı da saklayın.",
  "error.syncWebauthnUnavailable":
    "Sunucunuz şu anda bunun için bir güvenlik anahtarı kullanamıyor veya bu hesapta yok. Bunun yerine kimlik doğrulayıcı uygulama kodu kullanın.",
  "error.syncWebauthnLimit":
    "Bu hesapta sunucunun izin verdiği en fazla güvenlik anahtarı zaten var. Artık kullanmadığınız birini kaldırın, sonra yeniden deneyin.",
  "error.syncHandoffExpired": "Tarayıcı adımının süresi doldu. Yeniden başlatın.",
  "error.syncHandoffFailed":
    "Tarayıcıda çok fazla yanlış kod veya başarısız denetim oldu. Adımı yeniden başlatın.",
  "error.syncPasswordNeedsServer":
    "Ana parolanızı değiştirmek eşitleme sunucunuzu gerektirir ve ona ulaşılamıyor. İnternet bağlantınızı kontrol edin ve yeniden deneyin.",
  "error.syncDeviceIsCurrent": "Kullandığınız aygıtı kaldıramazsınız. Bunun yerine Bağlantıyı kes’i kullanın.",
  "error.syncChoiceExpired": "Klasör seçiminin süresi doldu. Klasörü yeniden seçin.",
  "error.syncFreshLoginExpired": "Parola denetiminizin süresi doldu. Ana parolanızı yeniden girin.",
  "error.syncPasswordRollback": "Sunucu daha eski bir ana parola sundu. Sunucuyu kontrol edin.",
  "error.syncPasswordChanged":
    "Ana parolanız sunucuda veya klasörde değiştirildi. Yenisini aşağıya girin.",
  "error.sharingNotHosted":
    "Paylaşım bir Mosaic sunucusu gerektirir. Önce Eşitleme ayarlarından birini bağlayın.",
  "error.setupCodeNeedsServer": "Bir telefon yalnızca eşitlenen bir hesaba katılabilir.",
  "error.sharingContactUnverified":
    "Önce bu kişinin güvenlik kodunu karşılaştırın. Mosaic yalnızca doğruladığınız kişilerle paylaşır.",
  "error.sharingKeyChanged":
    "Bu kişinin anahtarı değişti veya kod eşleşmiyor. Devam etmeden önce onu yeniden arayın ve yeni kodu karşılaştırın.",
  "error.sharingRoleForbidden":
    "Bu kasadaki rolünüz buna izin vermiyor. Sahibine veya bir yöneticiye sorun.",
  "error.sharingInviteExists":
    "Bu kişi ve kasa için bir davet zaten bekliyor. Yenisini göndermek istiyorsanız önce onu geri çekin.",
  "error.sharingInviteStale":
    "Davet bu arada, örneğin bir anahtar değişikliğinden sonra, değişti. Yeniden açın.",
  "error.sharingInviteExpired":
    "Bu davetin süresi doldu veya geri çekildi. Gönderenden sizi yeniden davet etmesini isteyin.",
  "error.sharingInviteUnreadable":
    "Bu davet açılamıyor. Hasarlı olabilir veya bu hesap için olmayabilir. Gönderenden sizi yeniden davet etmesini isteyin.",
  "error.sharingInviteNotInRoster":
    "Kasanın üye listesi bu davetle eşleşmiyor, bu yüzden Mosaic onu açmadı. Gönderenden sizi yeniden davet etmesini isteyin.",
  "error.sharingOwnerCannotLeave":
    "Sahip bir kasadan ayrılamaz. Bunun yerine diğer kişileri kaldırın.",
  "error.sharingAlreadyMember": "Bu kişi zaten kasanın bir üyesi.",
  "error.sharingReinviteBlocked":
    "Bu kişiyi kısa süre önce kaldırdınız. Kasa anahtarı değiştikten sonra yeniden davet edebilirsiniz. Eşitleyin, sonra yeniden deneyin.",
  "error.sharingInviteLimit": "Çok fazla davet bekliyor. Yanıtları bekleyin veya bazılarını geri çekin.",
  "error.sharingVaultKnown":
    "Bu aygıtta o kasa zaten var, bu yüzden Mosaic onun için bir anahtar kabul etmedi. Gönderene hangi kasayı paylaştığını sorun.",
  "error.syncHasSharedVaults":
    "Bu aygıtta paylaşılan kasalar varken sunucu değiştiremezsiniz. Önce paylaşılan kasalardan ayrılın veya sahiplerinin sizi kaldırmasını sağlayın.",
  "error.emergencyWaiting":
    "Bir istek bekliyor, bu yüzden bu şimdi değiştirilemez. Onu reddedin veya bekleyin.",
  "error.emergencyCooldown":
    "Son isteğiniz reddedildi. Bir günlük bekleme süresinden sonra yeniden isteyebilirsiniz.",
  "error.emergencyNotGranted":
    "Talep edilecek bir şey yok: açık bir istek yok veya kişi size erişim vermedi.",
  "error.shortcutUnavailable":
    "Başka bir program bu kısayolu zaten kullanıyor veya Mosaic onu kaydedemiyor. Farklı bir birleşim seçin.",
  "error.shortcutUnsupported":
    "Sisteminiz uygulamaların genel bir kısayol ayarlamasına izin vermiyor. Bunun yerine Mosaic’i penceresinden açın.",
} satisfies Record<keyof typeof en, string>;
