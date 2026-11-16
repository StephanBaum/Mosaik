/**
 * Turkish UI strings: ext. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Girişleri Mosaic kasanızdan doldurun. Ana parolanız masaüstü uygulamasında kalır.",
  "ext.command.fill": "Girişi Mosaic ile doldur",
  "ext.command.generate": "Mosaic ile parola oluştur",
  "ext.save.badgeTitle": "Giriş kaydedilsin mi?",
  "ext.loading": "Yükleniyor…",
  "ext.site.secure": "Geçerli site, güvenli bağlantı (https): {host}",
  "ext.site.insecure": "Geçerli site, şifrelenmemiş (http): {host}",
  "ext.site.none": "Bu sekmede web sitesi yok",
  "ext.nav.label": "Görünümler",
  "ext.nav.logins": "Girişler ve arama",
  "ext.nav.generator": "Parola oluşturucu",
  "ext.footer.open": "Mosaic’i aç",
  "ext.footer.more": "Diğer seçenekler",
  "ext.footer.connected": "Mosaic’e bağlı",
  "ext.footer.locked": "Mosaic kilitli",
  "ext.footer.offline": "Bağlı değil",
  "ext.more.saveOffers": "Girişleri kaydetmeyi öner",
  "ext.more.saveOffersHint":
    "Uzantının gönderdiğiniz bir girişi fark edebilmesi için tüm web sitelerine erişim gerekir. Mosaic uygulamasında onaylamadıkça hiçbir şey kaydedilmez.",
  "ext.passkeys.toggle": "Geçiş anahtarları için Mosaic’i kullan",
  "ext.passkeys.hint":
    "Geçiş anahtarı isteklerini yanıtlamak için tüm web sitelerine erişim gerekir. Her birini Mosaic uygulamasında onaylarsınız.",
  "ext.passkeys.permissionDenied":
    "Mosaic’in geçiş anahtarı isteklerini yanıtlaması için web sitelerine erişim gerekir ve siz buna izin vermediniz.",
  "ext.more.disconnect": "Mosaic ile bağlantıyı kes",
  "ext.more.disconnectTitle": "Bu tarayıcının bağlantısı kesilsin mi?",
  "ext.more.disconnectBody":
    "Bu tarayıcı, yeniden bağlayana kadar girişleri dolduramaz. Yeni bağlantıyı Mosaic uygulamasında onaylarsınız.",
  "ext.action.cancel": "İptal",
  "ext.neverSave.menu": "Kaydetme önerilmeyen siteler…",
  "ext.neverSave.title": "Kaydetme asla önerilmeyenler",
  "ext.neverSave.intro":
    "Mosaic bu sitelerde girişleri kaydetmeyi önermez. Orada yeniden sorulmasını istiyorsanız siteyi kaldırın.",
  "ext.neverSave.empty":
    "Henüz site yok. Mosaic bir girişi kaydetmeyi önerdiğinde “Bu site için asla” ile bir tane ekleyebilirsiniz.",
  "ext.neverSave.remove": "Kaldır",
  "ext.neverSave.removeLabel": "{origin} sitesini listeden kaldır",
  "ext.neverSave.removed": "{origin} kaldırıldı. Mosaic orada girişleri kaydetmeyi yeniden önerebilir.",
  "ext.neverSave.close": "Kapat",
  "ext.connect.checking": "Mosaic’e bağlanılıyor…",
  "ext.connect.hostMissing.title": "Mosaic bu tarayıcı için kurulmadı",
  "ext.connect.hostMissing.body":
    "Mosaic uygulamasını bu bilgisayara yükleyin, sonra tarayıcıyı yeniden başlatın.",
  "ext.connect.hostMissing.dev": "Geliştiriciler: {command} çalıştırın, sonra tarayıcıyı yeniden başlatın.",
  "ext.connect.hostForbidden.title": "Bu tarayıcının bağlanmasına izin verilmiyor",
  "ext.connect.hostForbidden.body":
    "Mosaic yalnızca resmi Mosaic uzantısıyla konuşur. Onu tarayıcınızın mağazasından yükleyin.",
  "ext.connect.desktopNotRunning.title": "Mosaic’i bu bilgisayarda açın",
  "ext.connect.desktopNotRunning.body":
    "Uzantı Mosaic uygulaması üzerinden çalışır. Uygulamayı başlatın, sonra yeniden kontrol edin.",
  "ext.connect.check": "Yeniden kontrol et",
  "ext.connect.notPaired.title": "Mosaic’e bağlanın",
  "ext.connect.notPaired.body":
    "Bu tarayıcıyı bu bilgisayardaki Mosaic uygulamasına bağlayın. Uygulamada onaylarsınız.",
  "ext.connect.start": "Bağlan",
  "ext.connect.pairing.title": "Mosaic’te onaylayın",
  "ext.connect.pairing.body": "Mosaic’in aynı kodu gösterdiğini kontrol edin, sonra orada İzin ver’e tıklayın.",
  "ext.connect.pairing.code": "Kod",
  "ext.connect.pairing.waiting": "Mosaic uygulamasında yanıtınız bekleniyor…",
  "ext.connect.protocolMismatch.title": "Mosaic’i güncelleyin",
  "ext.connect.protocolMismatch.body":
    "Uzantı ve Mosaic uygulaması farklı sürümlerde. İkisini de en son sürüme güncelleyin.",
  "ext.connect.error.title": "Mosaic’e ulaşılamıyor",
  "ext.connect.error.body": "Mosaic uygulamasının çalıştığını kontrol edin, sonra yeniden deneyin.",
  "ext.locked.title": "Mosaic kilitli",
  "ext.locked.body": "{host} sitesinde doldurmak için Mosaic uygulamasında kilidi açın.",
  "ext.locked.bodyNoSite": "Burada kullanmak için Mosaic uygulamasında kilidi açın.",
  "ext.locked.open": "Mosaic’i aç",
  "ext.locked.note": "Ana parolanız yalnızca Mosaic uygulamasına yazılır.",
  "ext.logins.forSite": "{host} için",
  "ext.logins.count": "{count, plural, one {# giriş} other {# giriş}}",
  "ext.logins.sameSite": "aynı site",
  "ext.logins.none": "{host} için kayıtlı giriş yok.",
  "ext.logins.noneHint": "Mosaic tam adresi karşılaştırır, bu yüzden benzer görünen siteler asla eşleşmez.",
  "ext.logins.unsupported": "Mosaic bu sayfada dolduramıyor. Bir girişi kopyalamak için kasanızda arayın.",
  "ext.logins.shortcut": "{shortcut} ilk eşleşmeyi doldurur.",
  "ext.logins.frameNote":
    "Mosaic yalnızca {host} sitesinde doldurur; başka sitelerden gömülü çerçevelerin içinde asla.",
  "ext.item.untitled": "Başlıksız",
  "ext.item.noUsername": "Kullanıcı adı yok",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "Doldur",
  "ext.item.fillLabel": "{name} öğesini doldur",
  "ext.item.copyMenu": "{name} için kopyalama seçenekleri",
  "ext.item.copyUsername": "Kullanıcı adını kopyala",
  "ext.item.copyPassword": "Parolayı kopyala",
  "ext.item.copyCode": "Kodu kopyala",
  "ext.item.blocked": "Engellendi: bu sayfa şifrelenmemiş ve bu giriş buna izin vermiyor.",
  "ext.search.label": "Diğer öğeler",
  "ext.search.placeholder": "Kasanızda arayın",
  "ext.search.note":
    "Sonuçlar bu açılır pencerede kalır. Sayfa yalnızca doldurduğunuz tek girişi alır.",
  "ext.search.none": "Hiçbir şey bulunamadı.",
  "ext.search.copy": "Kopyala",
  "ext.search.copyOnly": "Yalnızca kopyala: bu site için kaydedilmemiş",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Ctrl+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Ctrl+Shift+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Ctrl+Alt+C}}",
  "ext.search.results": "{count, plural, one {# sonuç} other {# sonuç}}",
  "ext.frame.cross":
    "Odaktaki alan başka bir sitenin çerçevesinde. Mosaic orada doldurmaz.",
  "ext.frameConfirm.title": "Gömülü bir çerçevenin içinde doldurulsun mu?",
  "ext.frameConfirm.body":
    "Alan sayfanın kendisinde değil, {host} üzerindeki bir çerçevede. Yalnızca bu çerçeveye güveniyorsanız burada doldurun.",
  "ext.frameConfirm.confirm": "Çerçevede doldur",
  "ext.warning.idn.title": "Adresi kontrol edin",
  "ext.warning.idn.body":
    "Bu adres özel karakterler kullanıyor ve başka bir siteye benzeyebilir. Oturum açmadan önce adres çubuğunu kontrol edin.",
  "ext.warning.lookalike.body":
    "Bu site {saved} değil. Benzer görünüyor, bu yaygın bir kimlik avı hilesidir. Bu sekmeyi kapatın veya {saved} adresini kendiniz yazın.",
  "ext.warning.youAreOn": "Şu sitedesiniz",
  "ext.warning.writtenAs": "Şöyle yazılmış",
  "ext.warning.savedFor": "Şunun için kayıtlı",
  "ext.warning.insecure.title": "Bu sayfa şifrelenmemiş",
  "ext.warning.insecure.body":
    "Sayfa http kullanıyor. Ağdaki diğer kişiler parolanızı okuyabilir. Mosaic burada yalnızca buna izin verdiğiniz girişleri doldurur.",
  "ext.otp.title": "Tek kullanımlık kodunuzu doldurun",
  "ext.otp.next": "{name} için parola dolduruldu. Sıradaki: iki adımlı doğrulama kodu.",
  "ext.otp.for": "{name} için kod",
  "ext.otp.fill": "Kodu doldur",
  "ext.otp.copy": "Kodu kopyala",
  "ext.otp.note": "Kod Mosaic’ten doğrudan alana gider. Bu açılır pencere onu asla göstermez.",
  "ext.save.titleNew": "{host} için bu giriş kaydedilsin mi?",
  "ext.save.titleUpdate": "{host} için kayıtlı giriş güncellensin mi?",
  "ext.save.generated": "Mosaic’in {host} için oluşturduğu bir parola.",
  "ext.save.updates": "“{title}” öğesini günceller.",
  "ext.save.username": "Kullanıcı adı",
  "ext.save.noUsername": "Yok",
  "ext.save.password": "Parola",
  "ext.save.passwordHidden": "Gizli. Mosaic uygulamasında görürsünüz.",
  "ext.save.insecure": "Bu sayfa şifrelenmemiş (http).",
  "ext.save.action": "Mosaic’e kaydet…",
  "ext.save.notNow": "Şimdi değil",
  "ext.save.never": "Bu site için asla",
  "ext.save.waiting": "Mosaic uygulamasında onaylayın. Siz onaylayana kadar hiçbir şey kaydedilmez.",
  "ext.save.saved": "Mosaic’e kaydedildi.",
  "ext.save.updated": "Mosaic’te güncellendi.",
  "ext.save.cancelled": "Kaydedilmedi.",
  "ext.save.dismissed": "Bırakıldı. Parola unutuldu.",
  "ext.save.neverDone": "Mosaic {host} sitesinde bir daha kaydetmeyi önermeyecek.",
  "ext.save.permissionAsk":
    "Mosaic tarayıcınızdan web sitelerine erişim ister. Bunu açmak için izin verin.",
  "ext.generator.title": "{host} için parola oluştur",
  "ext.generator.titleGeneric": "Parola oluştur",
  "ext.generator.value": "Oluşturulan parola",
  "ext.generator.regenerate": "Başka bir parola oluştur",
  "ext.generator.type": "Parola türü",
  "ext.generator.random": "Rastgele",
  "ext.generator.passphrase": "Parola ifadesi",
  "ext.generator.include": "Dahil et",
  "ext.generator.rules.page": "Bu sayfanın parola kurallarına uyar",
  "ext.generator.rules.siteQuirks": "Bu site için bilinen kurallara uyar",
  "ext.generator.fill": "Doldur",
  "ext.generator.fillHint":
    "Doldurmak için sayfadaki yeni parola alanına tıklayın, sonra bu açılır pencereyi yeniden açın.",
  "ext.generator.copy": "Kopyala",
  "ext.generator.expired": "Bu parola artık tutulmuyordu, bu yüzden yenisi oluşturuldu.",
  "ext.generator.filled": "Dolduruldu. Mosaic onu kaydetmeyi önerecek.",
  "ext.copied": "Kopyalandı. {seconds, plural, one {# saniye} other {# saniye}} sonra temizlenecek.",
  "ext.copiedNoClear": "Kopyalandı.",
  "ext.filled": "Dolduruldu.",
  "ext.filledCode": "Kod dolduruldu.",
  "ext.error.invalidRequest":
    "Mosaic bu isteği anlayamadı. Uzantıyı ve Mosaic uygulamasını güncelleyin.",
  "ext.error.notPaired": "Bu tarayıcı Mosaic’e bağlı değil. Yeniden bağlayın.",
  "ext.error.protocolVersion":
    "Uzantı ve Mosaic uygulaması farklı sürümlerde. İkisini de güncelleyin.",
  "ext.error.busy": "Mosaic başka bir istekle meşgul. Biraz sonra yeniden deneyin.",
  "ext.error.locked": "Mosaic kilitli. Mosaic uygulamasında kilidini açın.",
  "ext.error.rateLimited":
    "Kısa sürede çok fazla istek yapıldı. Biraz bekleyin, sonra yeniden deneyin.",
  "ext.error.originMismatch":
    "Bu giriş bu web sitesi için kayıtlı değil, bu yüzden Mosaic burada doldurmaz.",
  "ext.error.insecureOrigin":
    "Bu sayfa şifrelenmemiş ve giriş bunda doldurulmasına izin vermiyor. Hiçbir şey doldurulmadı.",
  "ext.error.crossOriginFrame":
    "Bu alan başka bir sitenin çerçevesinde. Mosaic orada doldurmaz.",
  "ext.error.itemNotFound": "Bu öğe artık yok. Taşınmış veya silinmiş olabilir.",
  "ext.error.vaultUnavailable": "Bu kasa kapalı. Önce Mosaic uygulamasında açın.",
  "ext.error.noOtp": "Bu öğede tek kullanımlık kod yok.",
  "ext.error.otpNotFillable":
    "Bu kod her kullanımda artıyor (HOTP), bu yüzden buradan doldurulamaz veya kopyalanamaz. Mosaic uygulamasında kullanın.",
  "ext.error.fieldEmpty": "Bu alan boş, bu yüzden doldurulacak veya kopyalanacak bir şey yok.",
  "ext.error.clipboardUnavailable":
    "Mosaic panoyu kullanamadı. Yeniden deneyin veya Mosaic uygulamasında kopyalayın.",
  "ext.error.cancelled": "Mosaic uygulamasında iptal edildi. Hazır olduğunuzda yeniden deneyin.",
  "ext.error.passkeyNotFound": "Mosaic’te bu site için geçiş anahtarı yok.",
  "ext.error.passkeyExcluded": "Mosaic’te bu hesap için zaten bir geçiş anahtarınız var.",
  "ext.error.rpIdMismatch":
    "Bu site o alan adı için bir geçiş anahtarı kullanamaz, bu yüzden Mosaic yanıt vermedi.",
  "ext.error.passkeyUnsupported":
    "Mosaic, bu sitenin istediği seçeneklerle bir geçiş anahtarı oluşturamıyor.",
  "ext.error.pairingDenied":
    "Bağlantı Mosaic uygulamasında reddedildi. Yeniden denemek için Bağlan’ı, sonra İzin ver’i seçin.",
  "ext.error.pairingExpired":
    "Bağlantı isteğinin süresi doldu. Yeniden başlatmak için Bağlan’ı seçin.",
  "ext.error.pairingConflict": "Başka bir bağlantı isteği açıktı. Yeniden denemek için Bağlan’ı seçin.",
  "ext.error.pairingBlocked": "Çok fazla bağlantı reddedildi. On dakika bekleyin, sonra yeniden deneyin.",
  "ext.error.pairingLimit":
    "Bu bilgisayarda çok fazla bağlı tarayıcı var. Mosaic ayarlarında birinin bağlantısını kesin.",
  "ext.error.internal":
    "Mosaic beklenmeyen bir sorunla karşılaştı. Yeniden deneyin; tekrarlanırsa Mosaic uygulamasını yeniden başlatın.",
  "ext.error.desktopNotRunning": "Mosaic bu bilgisayarda çalışmıyor. Açın, sonra yeniden deneyin.",
  "ext.error.desktopNotAuthentic":
    "Yanıt veren uygulama, bu tarayıcının bağlı olduğu Mosaic uygulaması değil. Hiçbir şey gönderilmedi.",
  "ext.error.timeout":
    "Mosaic zamanında yanıt vermedi. Mosaic uygulamasında bir iletişim kutusu açıksa orada yanıtlayın.",
  "ext.error.frameTooLarge": "Bu, Mosaic’e göndermek için çok fazla veriydi.",
  "ext.error.hostMissing":
    "Mosaic yardımcısı bu tarayıcı için yüklü değil. Mosaic uygulamasını yükleyin, sonra tarayıcıyı yeniden başlatın.",
  "ext.error.hostForbidden": "Bu tarayıcının veya uzantının Mosaic ile konuşmasına izin verilmiyor.",
  "ext.error.hostExited": "Mosaic ile bağlantı kapandı. Yeniden deneyin.",
  "ext.error.unsupportedPage": "Mosaic bu tür bir sayfada dolduramaz.",
  "ext.error.noFields": "Burada giriş alanı bulunamadı. Bir alana tıklayın, sonra yeniden deneyin.",
  "ext.error.noMatch": "Mosaic’te bu site için kayıtlı giriş yok.",
  "ext.error.frameNeedsConfirmation":
    "Alan bir çerçevenin içinde. Doldurmak için Mosaic açılır penceresini açın ve çerçeveyi onaylayın.",
  "ext.error.frameOpaque": "Alan korumalı alandaki bir çerçevede. Mosaic orada doldurmaz.",
  "ext.error.originChanged":
    "Mosaic doldururken sayfa değişti. Hiçbir şey doldurulmadı. Yeniden deneyin.",
  "ext.error.pageHidden": "Sayfa görünür değil, bu yüzden Mosaic doldurmadı.",
  "ext.error.permissionDenied":
    "Mosaic’in girişleri fark etmesi için web sitelerine erişim gerekir ve siz buna izin vermediniz.",
  "ext.error.generatedExpired": "Mosaic artık bu parolayı tutmuyor. Yenisini oluşturun.",
  "ext.rotation.card.title": "Parola değişikliği",
  "ext.rotation.card.for": "{name} için",
  "ext.rotation.card.started": "Başlangıç: {when}",
  "ext.rotation.card.delivered": "Yeni parola zaten bir kez dolduruldu.",
  "ext.rotation.card.note":
    "Yeni parola yalnızca Mosaic uygulamasında gösterilir. Mosaic formu sizin yerinize asla göndermez.",
  "ext.rotation.card.list": "Devam eden parola değişiklikleri",
  "ext.rotation.change": "Parolayı değiştir…",
  "ext.rotation.changeLabel": "{name} için parolayı değiştir…",
  "ext.rotation.openPage": "Değiştirme sayfasını aç",
  "ext.rotation.fillChange": "Değiştirme formunu doldur",
  "ext.rotation.fillPending": "Oturum açmak için yeni parolayı doldur",
  "ext.rotation.worked": "Çalıştı",
  "ext.rotation.failed": "Çalışmadı",
  "ext.rotation.fresh": "Yeni bir parolayla baştan başla",
  "ext.rotation.waiting": "Mosaic uygulamasında sizi bekliyor…",
  "ext.rotation.started": "Başlatıldı. Mosaic uygulamasında Bu parolayı kullan’ı seçin.",
  "ext.rotation.startCancelled": "Hiçbir şey başlatılmadı.",
  "ext.rotation.filled": "Değiştirme formu dolduruldu. Kontrol edin, sonra kendiniz gönderin.",
  "ext.rotation.opened": "Değiştirme sayfası açıldı.",
  "ext.rotation.confirmed": "Bitti. Yeni parola artık Mosaic’e kaydedildi.",
  "ext.rotation.keptOld": "Mosaic eski parolayı korudu.",
  "ext.rotation.resultCancelled": "Hiçbir şey değişmedi. Değişiklik hâlâ açık.",
  "ext.rotation.offer.title": "İlgili bir adres",
  "ext.rotation.offer.body":
    "Bu sayfa {host} üzerinde ve bu adres {title} için kayıtlı değil. Mosaic’in orada doldurabilmesi için eklensin mi?",
  "ext.rotation.offer.add": "{host} ekle",
  "ext.rotation.offer.added": "Eklendi. Değişiklik artık burada gösteriliyor.",
  "ext.rotation.offer.cancelled": "Eklenmedi.",
  "ext.error.rotationNotFound":
    "Bu parola değişikliği artık açık değil. Girişten yenisini başlatın.",
  "ext.error.originNotRelated":
    "Bu adres bu girişle aynı sitede değil, bu yüzden Mosaic onu eklemedi.",
  "ext.error.rotationLimit":
    "Bu girişin zaten dört tamamlanmamış değişikliği var. Önce Mosaic uygulamasında birini bitirin veya iptal edin.",
  "ext.error.changeFormNotFound":
    "Mosaic burada bir parola değiştirme formu bulamadı. Hesabınızın parola sayfasını açın, sonra yeniden deneyin.",
  "ext.error.changeFormAmbiguous":
    "Mosaic hangi alanın hangisi olduğunu ayırt edemedi. Yeni parolayı Mosaic’ten kopyalayın ve kendiniz yapıştırın.",
} satisfies Record<keyof typeof en, string>;
