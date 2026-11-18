/**
 * Turkish UI strings: vault. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { vault as en } from "../en/vault.ts";

export const vault = {
  "vault.itemCount": "{count, plural, =0 {Öğe yok} one {# öğe} other {# öğe}}",
  "vault.deleteConfirm": "“{title}” silinsin mi? 30 gün boyunca Çöp Kutusu’ndan geri yükleyebilirsiniz.",
  "vault.defaultName": "Kişisel",
  "vault.unnamed": "Adsız kasa",
  "vault.vaults": "Kasalar",
  "vault.allItems": "Tüm öğeler",
  "vault.items": "Öğeler",
  "vault.search": "Kasanızda arayın",
  "vault.untitled": "Başlıksız",
  "vault.noResults":
    "“{query}” ile eşleşen bir şey yok. Yazımı kontrol edin veya bunun yerine bir web sitesi arayın.",
  "vault.new": "Yeni öğe",
  "vault.selectItem": "Ayrıntılarını görmek için bir öğe seçin.",
  "vault.trashEmpty": "Çöp Kutusu boş. Silinen öğeler burada 30 gün kalır.",
  "vault.favorite": "Favori",
  "vault.unfavorite": "Favorilerden kaldır",
  "vault.moveTo": "Kasaya taşı",
  "vault.allVaults": "Tüm kasalar",
  "vault.category.favorites": "Favoriler",
  "vault.category.logins": "Girişler",
  "vault.category.passkeys": "Geçiş anahtarları",
  "vault.category.codes": "Tek kullanımlık kodlar",
  "vault.category.cards": "Kartlar",
  "vault.category.notes": "Güvenli notlar",
  "vault.category.ssh": "SSH anahtarları",
  "vault.flag.otp": "2FA",
  "vault.moveToNamed": "{name} kasasına taşı",
  "vault.emptyTitle": "Kasanız boş",
  "vault.emptyBody":
    "Bir giriş, geçiş anahtarı, kart veya not ekleyin. Her şey eşitlenmeden önce bu aygıtta şifrelenir.",
  "vault.addFirst": "İlk öğenizi ekleyin",
  "vault.paletteTip.before": "Arama yapmak veya komut çalıştırmak için istediğiniz zaman",
  "vault.paletteTip.after": "tuşlarına basın.",
  "trash.retention": "Silinen öğeler 30 gün saklanır.",
  "trash.rowMeta":
    "Silinme: {deletedAt, date, medium} · {days, plural, one {# gün} other {# gün}} kaldı",
  "trash.detailMeta": "{kind} · {vault} · Silinme: {deletedAt, date, medium}",
  "trash.purgeConfirm.title": "\"{title}\" kalıcı olarak silinsin mi?",
  "trash.purgeConfirm.body": "Bu işlem geri alınamaz.",
  "vault.hardware.placeholderName": "Kodlar (güvenlik anahtarı)",
  "vault.hardware.title": "Bu kasa güvenlik anahtarınızı gerektirir",
  "vault.hardware.body":
    "Buradaki kodlar, biri ana parolanızı ve Gizli Anahtarınızı öğrense bile güvende kalır, ancak güvenlik anahtarınız olmadan onları göremezsiniz.",
  "vault.hardware.open": "Güvenlik anahtarını kullan",
  "vault.hardware.lockedNote": "Kilitli. Diğer kasalarınız açık kalır.",
  "vault.hardware.lockNow": "Bu kasayı kilitle",
  "vault.hardware.closesIn": "{time} sonra kilitlenir",
  "vault.hardware.backupTitle": "Yedek bir anahtar kaydedin",
  "vault.hardware.backupBody":
    "1 anahtar kayıtlı. İki anahtar öneririz: tek anahtarınızı kaybederseniz bu kasayı bir daha açamazsınız.",
  "vault.hardware.createOption": "Güvenlik anahtarıyla koru (yalnızca kodlar)",
  "vault.hardware.tradeoff":
    "Bu kasadaki kodlar her seferinde güvenlik anahtarınızı gerektirir; bu yüzden yalnızca çalınmış bir ana parola yetmez, ancak kayıtlı tüm anahtarları kaybetmek bu kodları kaybettirir.",
  "vault.hardware.removeNote":
    "Bir anahtarı kaldırmak bu kasanın anahtarını değiştirmez. Bir anahtar çalındıysa kodları yeni bir kasaya taşıyın.",
  "vault.hardware.moveNote":
    "Zaten kullandığınız bir kodu bu kasaya taşımak önceki kopyaları korumaz: eski öğeyi, geçmişini, yedekleri ve dışa aktarmaları. Mümkünse sağlayıcıda yeni bir gizli anahtar oluşturun ve onu ekleyin.",
  "vault.hardware.pinBody":
    "Varsa anahtarınızın PIN’ini girin, sonra anahtar yanıp sönünce dokunun.",
  "vault.hardware.keys": "Anahtarlar",
  "vault.hardware.keysLabel": "Bu kasanın anahtarları",
  "vault.hardware.eyebrow": "Donanımla korunan kodlar",
  "vault.hardware.selfLocked": "Bu kasa kendini kilitledi. Yeniden açmak için güvenlik anahtarınızı kullanın.",
  "vault.hardware.closingSoon":
    "Bu kasa 30 saniyeden kısa sürede kilitlenecek. Sonrasında yeniden açmak için güvenlik anahtarınızı kullanın.",
  "vault.hardware.keysTitle": "Bu kasanın anahtarları",
  "vault.hardware.codesOnly": "Yalnızca tek kullanımlık kodlar",
  "vault.create": "Yeni kasa",
  "vault.create.name": "Kasa adı",
  "vault.create.submit": "Kasa oluştur",
  "vault.menu": "Kasa işlemleri",
  "vaultKey.menu": "Kasa anahtarını yenile…",
  "vaultKey.title": "{vault} kasasının anahtarını yenile",
  "vaultKey.body":
    "Mosaic bu kasa için yeni bir anahtar üretir ve erişimi olan her aygıta ve kişiye iletir. Ellerindeki mevcut kopyalar onlar için okunabilir kalır; yeni anahtar bundan sonra değişenleri korur. Birinin erişimi kaybolduysa veya bir aygıt kaybolduysa bunu kullanın.",
  "vaultKey.attachments": "Eklerin anahtarlarını da yenile",
  "vaultKey.attachments.hint":
    "Bu kasadaki her dosyaya da yeni bir anahtar verir. Çok sayıda veya büyük dosya varsa daha uzun sürer.",
  "vaultKey.reauth": "Kasa anahtarını yenilemek ana parolanızı gerektirir.",
  "vaultKey.submit": "Anahtarı yenile",
  "vaultKey.working": "Anahtar yenileniyor…",
  "vaultKey.progress": "Ek anahtarları yenileniyor: {done} / {total}.",
  "vaultKey.done": "Kasa anahtarı yenilendi.",
  "vaultKey.doneAttachments":
    "Kasa anahtarı yenilendi ve ek anahtarları da yenilendi.",
  "vaultKey.unfinished":
    "Kasa anahtarı yenilendi. Ek anahtarlarının yenilenmesi daha sonra devam eder. Mosaic önce kapanırsa bir sonraki sefer Anahtar yenilemeye devam et’i seçin.",
  "vaultKey.unreachable":
    "{count, plural, one {# ek henüz bu aygıtta değil ve eski anahtarını koruyor.} other {# ek henüz bu aygıtta değil ve eski anahtarlarını koruyor.}}",
  "vaultKey.close": "Kapat",
  "vault.meta.title": "Kasa ayrıntıları kısaltıldı",
  "vault.meta.body":
    "İki aygıt “{vault}” kasasının {fields} bilgisini aynı anda değiştirdi ve birlikte saklanamayacak kadar uzun oldular. Mosaic bir sürümü korudu veya alanı boş bıraktı. Kontrol edin ve gerekirse yeniden yazın.",
  "vault.edit.title": "Kasayı düzenle",
  "vault.meta.edit": "Kasayı düzenle",
  "vault.meta.field.name": "adı",
  "vault.meta.field.icon": "simgesi",
  "vault.meta.field.description": "açıklaması",
  "vault.meta.dismiss": "Kapat",
} satisfies Record<keyof typeof en, string>;
