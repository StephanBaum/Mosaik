/**
 * Turkish UI strings: rotation. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "Parola ve anahtarlar",
  "rotation.section.body":
    "Ana parolanızı değiştirin, yeni bir Gizli Anahtar alın veya tüm hesabınızı yeni anahtarlara taşıyın. Her değişiklik, herhangi bir şeye dokunulmadan önce ne olacağını size gösterir.",
  "rotation.action.password": "Ana parolayı değiştir",
  "rotation.action.password.body":
    "Yeni bir ana parola seçin. Gizli Anahtarınız ve kasalarınız olduğu gibi kalır.",
  "rotation.action.password.button": "Değiştir…",
  "rotation.action.secretKey": "Gizli Anahtarı yenile",
  "rotation.action.secretKey.body":
    "Yeni bir Gizli Anahtar ve yeni bir Acil Durum Kiti alın. Kitiniz kaybolduysa veya biri görmüş olabilirse bunu kullanın.",
  "rotation.action.secretKey.button": "Yenile…",
  "rotation.action.profile": "Korumayı yükselt",
  "rotation.action.profile.body":
    "Her deneme için daha fazla bellek kullanarak ana parolanızı tahmin etmeyi zorlaştırın. Kilit açma biraz daha uzun sürer.",
  "rotation.action.profile.button": "Yükselt…",
  "rotation.action.rekey": "Hesabı yeniden anahtarla",
  "rotation.action.rekey.body":
    "Hesabınızın tüm anahtarlarını yenileyin: yeni bir Gizli Anahtar, yeni kasa anahtarları ve yeni bir güvenlik kodu. Biri kasa verilerinizi parolanız ve Gizli Anahtarınızla birlikte kopyalamış olabilirse bunu kullanın.",
  "rotation.action.rekey.button": "Yeniden anahtarla…",
  "rotation.action.rekey.needsSync":
    "Yeniden anahtarlama, eşitleme sunucunuzdaki veya klasörünüzdeki kopyayı yeniler, bu yüzden eşitleme gerekir. Önce eşitlemeyi kurun.",
  "rotation.title.password": "Ana parolayı değiştir",
  "rotation.title.secretKey": "Gizli Anahtarı yenile",
  "rotation.title.profile": "Korumayı yükselt",
  "rotation.title.rekey": "Hesabı yeniden anahtarla",
  "rotation.step": "Adım {current} / {total}",
  "rotation.back": "Geri",
  "rotation.stop.title": "Bu değişiklik durdurulsun mu?",
  "rotation.stop.body": "Az önce kaydettiğiniz Acil Durum Kiti çalışmaz. Eski kitinizi saklayın.",
  "rotation.stop.confirm": "Değişikliği durdur",
  "rotation.stop.keep": "Devam et",
  "rotation.next": "Devam",
  "rotation.close": "Kapat",
  "rotation.review.title.password": "Parolanızı değiştirmeden önce",
  "rotation.review.title.secretKey": "Gizli Anahtarınızı yenilemeden önce",
  "rotation.review.title.profile": "Korumayı yükseltmeden önce",
  "rotation.review.title.rekey": "Bir şey ele mi geçirildi?",
  "rotation.review.changes": "Neler değişir",
  "rotation.review.stays": "Neler aynı kalır",
  "rotation.review.password.changes":
    "Ana parolanız. Bu aygıttaki hızlı kilit açma kapatılır ve sonradan yeniden kurabilirsiniz.",
  "rotation.review.password.stays":
    "Gizli Anahtarınız, kasalarınız ve öğeleriniz, güvenlik kodunuz ve acil durum kişileriniz.",
  "rotation.review.password.devices":
    "Diğer aygıtlarınızın eşitleme oturumu kapatılır. Her birinde yeni ana parolayı bir kez yazın, eşitleme devam eder.",
  "rotation.review.secretKey.changes":
    "Gizli Anahtarınız ve Acil Durum Kitiniz. Bu aygıttaki hızlı kilit açma kapatılır ve sonradan yeniden kurabilirsiniz.",
  "rotation.review.secretKey.stays":
    "Ana parolanız, kasalarınız ve öğeleriniz, güvenlik kodunuz ve acil durum kişileriniz. Kaydettiğiniz güvenlik anahtarları çalışmaya devam eder.",
  "rotation.review.secretKey.devices":
    "Diğer aygıtlarınızın eşitleme oturumu kapatılır. Her birinde parolanızı ve yeni kitteki yeni Gizli Anahtarı bir kez yazın.",
  "rotation.review.secretKey.limit":
    "Bu, eski Gizli Anahtarla oturum açmayı durdurur. Kasa verilerinizi eski parolanız ve Gizli Anahtarınızla zaten kopyalamış olan herkes onu hâlâ okuyabilir. Bunu durdurmak için hesabı yeniden anahtarlayın.",
  "rotation.review.secretKey.compromised": "Bir şey ele geçirildi: bunun yerine yeniden anahtarla",
  "rotation.review.profile.changes":
    "Her kilit açma denemesinin, sizin ve tahmin eden herkesin ne kadar bellek gerektirdiği. Bu aygıttaki hızlı kilit açma kapatılır ve sonradan yeniden kurabilirsiniz.",
  "rotation.review.profile.stays":
    "Ana parolanız, Gizli Anahtarınız, kasalarınız ve güvenlik kodunuz. Kaydettiğiniz güvenlik anahtarları çalışmaya devam eder.",
  "rotation.review.profile.devices":
    "Diğer aygıtlarınızın eşitleme oturumu kapatılır. Her birinde ana parolanızı bir kez yazın, eşitleme devam eder. Daha yavaş aygıtların kilidi biraz daha uzun sürede açılır.",
  "rotation.review.rekey.intro":
    "Yeniden anahtarlama, birinin kasa verilerinize ve parolanıza ile Gizli Anahtarınıza sahip olabileceği durum içindir. Yalnızca bir aygıtı kaybettiyseniz Gizli Anahtarı yenilemek veya parolanızı değiştirmek yeterlidir.",
  "rotation.review.rekey.yes": "Evet, hesabımı yeniden anahtarla",
  "rotation.review.rekey.no": "Hayır, bunun yerine yalnızca Gizli Anahtarı yenile",
  "rotation.review.rekey.limit":
    "Yeniden anahtarlama, birinin zaten kopyaladığını geri almaz. Sunuculardaki ve yedeklerdeki eski kopyalar eski anahtarlarla şifreli kalır ve eski anahtarlar onları hâlâ açar.",
  "rotation.review.rekey.title": "Yeniden anahtarlama ne yapar",
  "rotation.review.rekey.vaults":
    "{count, plural, one {Yalnızca sizin kullandığınız # kasa yeni bir anahtar alır.} other {Yalnızca sizin kullandığınız # kasa yeni anahtarlar alır.}}",
  "rotation.review.rekey.keys":
    "Yeni bir Gizli Anahtar, yeni bir Acil Durum Kiti, yeni bir güvenlik kodu ve yeni bir hesap anahtarı.",
  "rotation.review.rekey.devices":
    "Diğer aygıtlarınızın oturumu kapatılır ve her birinde yeni Gizli Anahtarı ile yeni ana parolanızı yazana kadar eşitleyemez.",
  "rotation.review.rekey.shared":
    "{count, plural, one {# kasa paylaşıyorsunuz.} other {# kasa paylaşıyorsunuz.}} Her biri yeni bir anahtar alır.",
  "rotation.review.rekey.sharedRow": "{name}, {count, plural, one {# üye} other {# üye}}",
  "rotation.review.rekey.sharedMembers":
    "Paylaştığınız her kişiden yeni güvenlik kodunuzu karşılaştırmasını isteyin. Bunu yapana kadar sizden yeni anahtar almazlar ve yenilenen kasayı açamazlar.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, one {# kasa başkasına ait.} other {# kasa başkalarına ait.}} Sahiplerinden yenilemeleri istenir. Bunu yapana kadar eski anahtarlarınız onları hâlâ okuyabilir.",
  "rotation.review.rekey.grants":
    "{count, plural, one {# kişinin kasalarınıza acil durum erişimi var.} other {# kişinin kasalarınıza acil durum erişimi var.}} Erişimleri yeni anahtarlar için yenilenir. Her birinin de yeni güvenlik kodunuzu karşılaştırması gerekir.",
  "rotation.review.rekey.quick":
    "Bu aygıttaki hızlı kilit açma kapatılır. Sonradan yeniden kurabilirsiniz.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, one {Kilit açmak için kaydettiğiniz # güvenlik anahtarı kapatılır.} other {Kilit açmak için kaydettiğiniz # güvenlik anahtarı kapatılır.}} Sonradan yeniden kaydedebilirsiniz.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, one {Donanımla korunan # kasa etkilenmez.} other {Donanımla korunan # kasa etkilenmez.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, one {# ek bu aygıtta değil.} other {# ek bu aygıtta değil.}} Yeniden korunabilmeleri için önce indirilirler. Ulaşılamayanlar eski anahtarı korur.",
  "rotation.review.rekey.folder":
    "Eşitleme klasörünüz yeni bir hesap dosyası alır. Diğer aygıtlar orada anahtarların değiştiğine dair bir not bulur.",
  "rotation.review.rekey.unavailable": "Yeniden anahtarlama kullanılamıyor: eşitleme kurulmadı.",
  "rotation.review.loading": "Yeniden anahtarlamanın nelere dokunduğu kontrol ediliyor…",
  "rotation.review.rekey.needsClean":
    "Yeniden anahtarlama için eşitlemenin güncel ve uyarısız olması gerekir. Şimdi eşitleyin, uyarılara bakın, sonra yeniden deneyin.",
  "rotation.credentials.title": "Siz olduğunuzu onaylayın",
  "rotation.credentials.password.title": "Ana parolalarınız",
  "rotation.credentials.current": "Mevcut ana parola",
  "rotation.credentials.new": "Yeni ana parola",
  "rotation.credentials.confirm": "Yeni ana parolayı yeniden yazın",
  "rotation.credentials.mismatch": "İki parola farklı.",
  "rotation.credentials.same": "Yeni ana parola mevcut olandan farklı olmalıdır.",
  "rotation.credentials.hint":
    "Uzun bir parola ifadesi, kısa ve karmaşık bir paroladan daha iyidir. Onu kurtaramazsınız: Mosaic onu bilmez.",
  "rotation.credentials.code": "Kimlik doğrulayıcı uygulamanızdaki 6 haneli kod",
  "rotation.credentials.profile": "Koruma düzeyi",
  "rotation.credentials.profile.standard": "Standart: deneme başına 256 MB bellek",
  "rotation.credentials.profile.strong": "Güçlü: deneme başına 512 MB bellek",
  "rotation.credentials.profile.max": "En yüksek: deneme başına 1 GB bellek",
  "rotation.credentials.profile.slow":
    "Bu düzeyle kilit açma bu aygıtta iki saniyeden uzun sürer. Diğer, daha eski aygıtlar daha da yavaş olur.",
  "rotation.credentials.profile.force": "Yine de yükselt",
  "rotation.kit.title": "Yeni Acil Durum Kitinizi kaydedin",
  "rotation.kit.body":
    "Yeni Gizli Anahtarınız bu kitte. Önce kaydedin: Mosaic hesabınızı yalnızca kiti aldığınızı onayladıktan sonra değiştirir, çünkü onsuz yeni bir aygıtta oturum açamazsınız.",
  "rotation.kit.save": "Yeni Acil Durum Kitini kaydet",
  "rotation.kit.saved": "Kaydedildi. Aygıtlarınızdan ayrı, güvenli bir yerde saklayın.",
  "rotation.kit.cancelled": "Henüz hiçbir şey kaydedilmedi.",
  "rotation.kit.verify": "Kaydedilen kitten yeni Gizli Anahtarınızın son grubunu yazın.",
  "rotation.kit.confirm": "Kiti onayla",
  "rotation.kit.mistyped": "Bu, yeni Gizli Anahtarın sonu değil. Kaydedilen kiti kontrol edin.",
  "rotation.kit.verified": "Kit onaylandı.",
  "rotation.confirm.title.password": "Ana parolanızı değiştirin",
  "rotation.confirm.title.secretKey": "Gizli Anahtarınızı yenileyin",
  "rotation.confirm.title.profile": "Korumayı yükseltin",
  "rotation.confirm.title.rekey": "Hesabınızı yeniden anahtarlayın",
  "rotation.confirm.body.rekey":
    "Bu, diğer aygıtlarınızın oturumunu kapatır ve tüm anahtarları yeniler. Bitene kadar Mosaic’i açık tutun.",
  "rotation.confirm.body.secretKey":
    "Bu, diğer aygıtlarınızın eşitleme oturumunu kapatır. Bitene kadar Mosaic’i açık tutun.",
  "rotation.confirm.verb.password": "Parolayı değiştir",
  "rotation.confirm.verb.secretKey": "Gizli Anahtarı yenile",
  "rotation.confirm.verb.profile": "Korumayı yükselt",
  "rotation.confirm.verb.rekey": "Hesabı yeniden anahtarla",
  "rotation.run.title": "Anahtarlarınız yenileniyor…",
  "rotation.run.keepOpen": "Mosaic’i açık tutun. Çok sayıda ek varsa bu birkaç dakika sürebilir.",
  "rotation.progress.label": "İlerleme",
  "rotation.progress.count": "{done} / {total}",
  "rotation.progress.done": "bitti",
  "rotation.phase.prepared": "Hazırlanıyor",
  "rotation.phase.backendStarted": "Hesabınızdan yeni anahtarlara geçmesi isteniyor",
  "rotation.phase.backendAccepted": "Hesabınız yeni anahtarları kabul etti",
  "rotation.phase.folderWritten": "Eşitleme klasöründe yeni hesap dosyası var",
  "rotation.phase.localCommitted": "Bu aygıt artık yeni anahtarları kullanıyor",
  "rotation.phase.attachments": "Ekler yeniden korunuyor",
  "rotation.phase.uploading": "Korunan veriler gönderiliyor",
  "rotation.phase.done": "Bitti",
  "rotation.done.title.password": "Ana parola değiştirildi",
  "rotation.done.title.secretKey": "Gizli Anahtar yenilendi",
  "rotation.done.title.profile": "Koruma yükseltildi",
  "rotation.done.title.rekey": "Hesap yeniden anahtarlandı",
  "rotation.done.body.password":
    "Bundan sonra yeni ana parolayı kullanın. Diğer aygıtlarınız onu bir kez ister.",
  "rotation.done.body.secretKey":
    "Yeni Acil Durum Kitini güvende tutun ve eskisini silin. Diğer aygıtlarınız yeni Gizli Anahtarı bir kez ister.",
  "rotation.done.body.profile":
    "Kilit açma artık daha fazla bellek gerektiriyor. Diğer aygıtlarınız parolanızı bir kez ister.",
  "rotation.done.body.rekey":
    "Yeni Acil Durum Kitini güvende tutun ve eskisini silin. Diğer aygıtlarınız yeni Gizli Anahtarı ve yeni ana parolanızı bir kez ister.",
  "rotation.done.members":
    "Kasa paylaştığınız herkesten yeni güvenlik kodunuzu karşılaştırmasını isteyin. Bunu yapana kadar sizden yeni anahtar almazlar.",
  "rotation.done.quick": "Hızlı kilit açmayı yeniden kurun",
  "rotation.done.quick.body": "Hızlı kilit açma kapatıldı.",
  "rotation.done.hardware": "Güvenlik anahtarlarını yeniden kaydedin",
  "rotation.done.hardware.body":
    "{count, plural, one {# güvenlik anahtarı kilit açma için kapatıldı.} other {# güvenlik anahtarı kilit açma için kapatıldı.}}",
  "rotation.done.unreachable.note":
    "Bu dosyalar okunamadı ve yeniden şifrelenmedi. Siz kaldırana veya yeniden ekleyene kadar eski dosya anahtarlarını korurlar; bu yüzden eski anahtarlarınıza ve bir dosyanın kopyasına sahip olan herkes onu yine açabilir.",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "Bilinmeyen öğe",
  "rotation.done.unreachable.unknownFile": "bilinmeyen dosya",
  "rotation.done.unreachable.more":
    "{count, plural, one {ve # dosya daha} other {ve # dosya daha}}",
  "rotation.done.unfinished":
    "Son bölüm daha sonra devam eder. Mosaic bitmeden kapanırsa bir sonraki sefer Anahtar yenilemeye devam et’i seçin.",
  "rotation.resume.title": "Anahtar değişikliği tamamlanmadı",
  "rotation.resume.body.password":
    "Bir anahtar değişikliği yarıda kesildi. Hiçbir şey kaybolmadı. Parolalarınızı yeniden yazmak için Devam’ı seçin veya silin.",
  "rotation.resume.body.secretKey":
    "Gizli Anahtarınızın yenilenmesi yarıda kesildi. Hiçbir şey kaybolmadı. Bitirmek için devam edin.",
  "rotation.resume.body.rekey":
    "Hesabınızın yeniden anahtarlanması yarıda kesildi. Hiçbir şey kaybolmadı. Bitirmek için devam edin.",
  "rotation.resume.body.profile":
    "Korumanın yükseltilmesi yarıda kesildi. Hiçbir şey kaybolmadı. Bitirmek için devam edin.",
  "rotation.resume.continue": "Anahtar yenilemeye devam et",
  "rotation.resume.discard": "Sil…",
  "rotation.resume.running": "Devam ediliyor…",
  "rotation.resume.progress": "{done} / {total} bitti. Bitene kadar Mosaic’i açık tutun.",
  "rotation.resume.dialog.title": "Anahtar yenilemeye devam et",
  "rotation.resume.dialog.password": "Şu anki ana parolanız",
  "rotation.resume.dialog.group": "Yeni Gizli Anahtarın son grubu",
  "rotation.resume.dialog.body":
    "Hesabınızın anahtarları zaten değişmiş olabilir. Sizin olduğunuzdan emin olmak için ana parolanızı ve yeni Acil Durum Kitindeki yeni Gizli Anahtarın son grubunu yazın.",
  "rotation.resume.dialog.submit": "Devam",
  "rotation.resume.discard.title": "Bu anahtar değişikliği silinsin mi?",
  "rotation.resume.discard.body":
    "Bu denemedeki yeni Gizli Anahtar ve yeni Acil Durum Kiti silinir. Hesabınız olduğu gibi kalır.",
  "rotation.resume.discard.confirm": "Anahtar değişikliğini sil",
  "rotation.resume.abandon": "Geri kalanından vazgeç…",
  "rotation.resume.stalled": "Devam etme tamamlanmadı. Hep başarısız olursa burada durabilirsiniz.",
  "rotation.abandon.title": "Dosyalarınızı yeniden şifrelemek durdurulsun mu?",
  "rotation.abandon.body":
    "Yeni anahtarlarınız zaten kullanımda. Mosaic hâlâ bekleyen eklerin yeniden şifrelenmesini durdurur. Bunlar eski dosya anahtarlarını korur; bu yüzden eski anahtarlarınıza ve bir dosyanın kopyasına sahip olan herkes onu yine açabilir.",
  "rotation.abandon.stays":
    "Zaten yeniden şifrelenen her şey olduğu gibi kalır. Bekleyen dosyaları yeni anahtarla korumak için daha sonra kaldırın veya yeniden ekleyin.",
  "rotation.abandon.confirm": "Yeniden şifrelemeyi durdur",
  "rotation.abandon.keep": "Denemeye devam et",
  "rotation.marker.title": "Onaylanmamış yeniden anahtarlama bildirimi",
  "rotation.marker.hint":
    "Anahtarlarınızı başka bir aygıtta yenilemediyseniz bu bildirim gerçek olmayabilir. Yoksayabilirsiniz.",
  "rotation.marker.open": "Bu bildirimi yoksay…",
  "rotation.marker.open.label": "Bu bildirimi yoksay: {name}",
  "rotation.marker.confirm":
    "Mosaic doğrulayamadığı bir yeniden anahtarlama bildirimi buldu. Hesabınızı başka bir aygıtta yeniden anahtarlamadıysanız yoksayabilirsiniz.",
  "rotation.marker.ignore": "Bildirimi yoksay",
  "rotation.marker.keep": "Bildirimi tut",
  "rotation.ignored.title": "Yoksayılan yeniden anahtarlama bildirimleri",
  "rotation.ignored.body":
    "Yoksaymayı seçtiğiniz bildirimler. Hesabınızı başka bir aygıtta gerçekten yeniden anahtarladıysanız birini geri yükleyin.",
  "rotation.ignored.row": "Eşitleme klasörünüzden yeniden anahtarlama bildirimi",
  "rotation.ignored.restore": "Geri yükle",
  "rotation.ignored.restore.label": "{name} bildirimini geri yükle",
  "rotation.resume.done": "Anahtar değişikliği bitti.",
  "rotation.recovery.title": "Hesabınızın anahtarları başka bir aygıtta yenilendi",
  "rotation.recovery.body":
    "Aygıtlarınızdan biri bu hesabın anahtarlarını yeniledi. Eşitlemeye devam etmek için ana parolanızı ve yeni Acil Durum Kitindeki Gizli Anahtarı yazın.",
  "rotation.recovery.password": "Ana parola (orada değiştirdiyseniz yenisi)",
  "rotation.recovery.secretKey": "Yeni Acil Durum Kitindeki Gizli Anahtar",
  "rotation.recovery.submit": "Yeni anahtarları kullan",
  "rotation.recovery.warning":
    "Anahtarlarınızı kendiniz yenilemediniz mi? O zaman eski parolanıza ve Gizli Anahtarınıza sahip biri yenilemiş olabilir. Buraya hiçbir şey girmeyin; önce diğer aygıtlarınızı kontrol edin.",
  "rotation.recovery.notice": "Bunu bu aygıtta yapana kadar eşitleme duraklatıldı.",
  "rotation.recovery.open": "Yeni anahtarları kullan…",
  "rotation.recovery.keyHint":
    "Anahtarlarınız başka bir aygıtta yenilendiyse, onun Acil Durum Kitindeki yeni Gizli Anahtarı da girin.",
  "rotation.recovery.done": "Bu aygıt artık yeni anahtarları kullanıyor.",
  "rotation.recovery.optional": "En yeni Acil Durum Kitindeki Gizli Anahtar (yalnızca yenilediyseniz)",
  "rotation.file.unknown":
    "Eşitleme klasörünüzde bilinmeyen bir hesap dosyası belirdi. Anahtarlarınızla denetlenemiyor: artık bir kalıntı olabilir veya biri klasörünüzü değiştirmiş olabilir. Verileriniz etkilenmedi.",
  "rotation.file.dismiss": "Kapat",
  "rotation.file.conflict.title": "İki aygıt anahtarları aynı anda yeniledi",
  "rotation.file.conflict":
    "Başka bir aygıt hesabınızın anahtarlarını bununla aynı anda yeniledi, bu yüzden ikisi artık uyuşmuyor. Birleştirmek için bu aygıttan yeniden anahtarlayın: sonra bu aygıtın anahtarları geçerli olur ve diğeri yeni Gizli Anahtarı ister.",
  "rotation.file.conflict.rekey": "Bu aygıttan yeniden anahtarla",
  "rotation.file.conflict.keep": "Bildirimi gizle",
  "rotation.resume.body.vault":
    "Bir kasanın eklerinin yeniden şifrelenmesi yarıda kesildi. Hiçbir şey kaybolmadı. Bitirmek için devam edin.",
} satisfies Record<keyof typeof en, string>;
