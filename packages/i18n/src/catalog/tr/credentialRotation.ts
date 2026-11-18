/**
 * Turkish UI strings: credentialRotation. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "Güvenlik görünümleri",
  "credentialRotation.tab.overview": "Genel bakış",
  "credentialRotation.tab.rotate": "Şimdi değiştir",
  "credentialRotation.tab.dueHidden": ", {count} değiştirilecek",
  "credentialRotation.queue.title": "Şimdi değiştir",
  "credentialRotation.queue.intro":
    "Önce değiştirmeye değer parolalar. Mosaic size rehberlik eder ama her parolayı siteden kendiniz değiştirirsiniz.",
  "credentialRotation.queue.empty": "Şu anda değiştirilecek bir şey yok.",
  "credentialRotation.queue.count":
    "{count, plural, one {Değiştirilecek # parola} other {Değiştirilecek # parola}}",
  "credentialRotation.queue.list": "Değiştirilecek parolalar",
  "credentialRotation.queue.snoozedHeading": "Ertelenenler",
  "credentialRotation.queue.snoozedUntil": "{date, date, medium} tarihine kadar ertelendi",
  "credentialRotation.queue.reason.weak": "Zayıf",
  "credentialRotation.queue.reason.reused": "Yeniden kullanılmış",
  "credentialRotation.queue.reason.old": "Eski",
  "credentialRotation.queue.reason.breached": "Veri ihlalinde",
  "credentialRotation.queue.reason.unfinished": "Değişiklik tamamlanmadı",
  "credentialRotation.queue.change": "Parolayı değiştir",
  "credentialRotation.queue.continue": "Değişikliğe devam et",
  "credentialRotation.queue.editItem": "Öğeyi düzenle",
  "credentialRotation.queue.manualHint":
    "Mosaic bunda rehberlik edemiyor. Öğeyi açın ve kendiniz değiştirin.",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "Ertele",
  "credentialRotation.queue.snoozeFor": "{title} öğesini ertele",
  "credentialRotation.queue.snoozeDays": "{days, plural, one {# gün} other {# gün}}",
  "credentialRotation.queue.snoozed": "{days, plural, one {# gün} other {# gün}} ertelendi.",
  "credentialRotation.queue.dismiss": "Yoksay",
  "credentialRotation.queue.dismissFor": "{title} öğesini yoksay",
  "credentialRotation.queue.dismissed": "Yoksayıldı. Parola değişince geri gelir.",
  "credentialRotation.queue.undo": "Geri al",
  "credentialRotation.panel.title": "{title} için parolayı değiştir",
  "credentialRotation.panel.step": "Adım {current} / {total}",
  "credentialRotation.panel.stays":
    "{title} için değişiklik açık kalır. Yeni parola, Çalıştı veya Çalışmadı’yı seçene kadar öğede saklanır.",
  "credentialRotation.panel.cancel": "İptal",
  "credentialRotation.steps.new.title": "Yeni bir parola seçin",
  "credentialRotation.steps.new.intro":
    "Mosaic yeni parolayı bir yerde kullanmadan önce bu öğeye kaydeder; böylece kaybolamaz. Siz onaylayana kadar mevcut parola yerinde kalır.",
  "credentialRotation.steps.new.label": "Yeni parola",
  "credentialRotation.steps.new.use": "Bu parolayı kullan",
  "credentialRotation.steps.new.saving": "Kaydediliyor…",
  "credentialRotation.steps.site.title": "Sitede değiştirin",
  "credentialRotation.steps.site.pageLabel": "Değiştirme sayfası",
  "credentialRotation.steps.site.body":
    "Sitenin parola sayfasını tarayıcınızda açın, araç çubuğundaki Mosaic’e tıklayın ve Değiştirme formunu doldur’u seçin. Mosaic formu asla göndermez.",
  "credentialRotation.steps.site.copyLink": "Değiştirme sayfasının bağlantısını kopyala",
  "credentialRotation.steps.site.linkCopied": "Bağlantı kopyalandı.",
  "credentialRotation.steps.site.linkHint":
    "Bağlantı gizli değildir. Sitede bu adreste bir sayfa yoksa parola ayarlarını elle bulun.",
  "credentialRotation.steps.site.noLink":
    "Bu girişin https adresi yok, bu yüzden Mosaic’in önereceği bir değiştirme sayfası yok. Parolayı sitede elle değiştirin.",
  "credentialRotation.steps.site.copyPassword": "Yeni parolayı kopyala",
  "credentialRotation.steps.site.next": "Denedim",
  "credentialRotation.steps.result.title": "Site yeni parolayı kabul etti mi?",
  "credentialRotation.steps.done.failed": "Mevcut parola korundu",
  "credentialRotation.steps.done.notSure": "Değişiklik açık bırakıldı",
  "credentialRotation.steps.result.body":
    "Mosaic siteyi göremez. Ne olduğunu söyleyin ki doğru parola bu öğede kalsın. Her iki durumda da hiçbir şey silinmez.",
  "credentialRotation.steps.result.working": "Kaydediliyor…",
  "credentialRotation.result.worked": "Çalıştı",
  "credentialRotation.result.failed": "Çalışmadı",
  "credentialRotation.result.notSure": "Henüz emin değilim",
  "credentialRotation.result.workedDone":
    "Bitti. Yeni parola artık bu öğenin parolası. Eskisi geçmişinde.",
  "credentialRotation.result.failedDone":
    "Mevcut parola kalır. Yenisi, site yine de uygulamış olabilir diye, onaylanmamış olarak geçmişte saklanır.",
  "credentialRotation.result.notSureDone":
    "Değişiklik bu öğede açık kalır. Öğrendiğinizde geri dönün.",
  "credentialRotation.banner.title": "Parola değişikliği {date, date, medium} tarihinde başlatıldı",
  "credentialRotation.banner.prepared":
    "Yeni parola siz onaylayana kadar burada saklanır. Henüz hiçbir sayfa onu almadı.",
  "credentialRotation.banner.delivered":
    "Yeni parola siz onaylayana kadar burada saklanır. Tarayıcı uzantısı onu sitenin formuna doldurdu.",
  "credentialRotation.banner.reveal": "Yeni parolayı göster",
  "credentialRotation.banner.hide": "Yeni parolayı gizle",
  "credentialRotation.banner.copy": "Yeni parolayı kopyala",
  "credentialRotation.banner.newPassword": "Yeni parola",
  "credentialRotation.banner.copyLink": "Değiştirme sayfasının bağlantısını kopyala",
  "credentialRotation.history.title": "Parola geçmişi",
  "credentialRotation.history.list": "Önceki parolalar",
  "credentialRotation.history.entry": "Değiştirilme: {date, date, medium}",
  "credentialRotation.history.unconfirmed": "Onaylanmadı",
  "credentialRotation.history.unconfirmedEntry": "Denenme: {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "Site bunu uygulamış olabilir. Mevcut parolayla oturum açma başarısız olursa geri yükleyin.",
  "credentialRotation.history.reveal": "{date, date, medium} tarihli parolayı göster",
  "credentialRotation.history.hide": "{date, date, medium} tarihli parolayı gizle",
  "credentialRotation.history.restore": "Geri yükle",
  "credentialRotation.history.restoreFor": "{date, date, medium} tarihli parolayı geri yükle",
  "credentialRotation.history.restored": "Geri yüklendi. Önceki parola geçmişte.",
  "credentialRotation.policy.label": "Bu parolayı değiştirmem için hatırlat",
  "credentialRotation.policy.none": "Asla",
  "credentialRotation.policy.every": "Her {days} günde bir",
  "credentialRotation.policy.help":
    "Bir hatırlatıcı öğeyi Şimdi değiştir listesine koyar. Mosaic bunun için bildirim göndermez ve istek yapmaz.",
  "credentialRotation.url.help":
    "İsteğe bağlı. Sitenin parola sayfasının https adresi, bu girişle aynı sitede. Mosaic onu yalnızca tarayıcı uzantısında Değiştirme sayfasını aç’ı seçtiğinizde açar.",
  "credentialRotation.url.invalid":
    "Mosaic bu adresi kullanmayacak. https:// ile başlamalı ve bu girişle aynı sitede olmalı.",
  "credentialRotation.settings.title": "Parola hatırlatıcıları",
  "credentialRotation.settings.body":
    "Mosaic, kilidi açtıktan sonra kaç parolanın değiştirilmeye değer olduğunu size söyleyebilir.",
  "credentialRotation.settings.reminders": "Kilit açıldıktan sonra kaç parolanın değiştirileceğini göster",
  "credentialRotation.settings.remindersHint":
    "Yalnızca uygulama içinde bir mesaj. Bildirim yok, ağ isteği yok.",
  "credentialRotation.reminder.toast":
    "{count, plural, one {Değiştirilecek # parola} other {Değiştirilecek # parola}}",
  "credentialRotation.reminder.show": "Göster",
  "credentialRotation.dialog.beginTitle": "Parola değişikliği başlatılsın mı?",
  "credentialRotation.dialog.beginBody":
    "{browser}, {host} sitesindeki {title} parolasını değiştirmek istiyor. Mosaic yeni parolayı bu girişle birlikte saklar ve sitenin kabul ettiğini onaylayana kadar mevcut parolayı korur.",
  "credentialRotation.dialog.site": "Web sitesi",
  "credentialRotation.dialog.login": "Giriş",
  "credentialRotation.dialog.password": "Yeni parola",
  "credentialRotation.dialog.hidden": "Gizli",
  "credentialRotation.dialog.show": "Göster",
  "credentialRotation.dialog.hide": "Gizle",
  "credentialRotation.dialog.regenerate": "Başka bir tane oluştur",
  "credentialRotation.dialog.regenerated": "Yeni bir parola oluşturuldu.",
  "credentialRotation.dialog.allow": "Bu parolayı kullan",
  "credentialRotation.dialog.cancel": "İptal",
  "credentialRotation.dialog.decideLater": "Sonra karar ver",
  "credentialRotation.dialog.untitled": "bu giriş",
  "credentialRotation.dialog.attempt":
    "Başlangıç: {at, date, medium}, {at, time, short} · {stage}{n, plural, =0 {} one { · # başka değişiklik açık} other { · # başka değişiklik açık}}",
  "credentialRotation.dialog.stagePrepared": "henüz hiçbir sayfada yeni parola yok",
  "credentialRotation.dialog.stageDelivered": "yeni parola bir sayfaya dolduruldu",
  "credentialRotation.dialog.resultTitle": "{host} yeni parolayı kabul etti mi?",
  "credentialRotation.dialog.resultBody":
    "{browser} değişikliğin çalıştığını söylüyor. Onaylarsanız yeni parola {title} için parola olur ve mevcut parola geçmişine taşınır.",
  "credentialRotation.dialog.confirm": "Çalıştı",
  "credentialRotation.dialog.keepOld": "Eski parolayı koru",
  "credentialRotation.dialog.failedTitle": "Eski parola korunsun mu?",
  "credentialRotation.dialog.failedBody":
    "{browser} değişikliğin çalışmadığını söylüyor. Mosaic {title} için mevcut parolayı korur. Site yine de değiştirdiyse diye yeni değer geçmişinde kalır.",
  "credentialRotation.addOrigin.title": "{host}, {title} öğesine eklensin mi?",
  "credentialRotation.addOrigin.body":
    "Mosaic bu girişi {host} sitesinde de dolduracak. Yalnızca {host} gerçekten aynı sitenin parçasıysa ekleyin.",
  "credentialRotation.addOrigin.idn":
    "Bu adres özel karakterler kullanıyor. Tarayıcınız onu {unicode} olarak gösterebilir. Dikkatlice kontrol edin.",
  "credentialRotation.addOrigin.add": "Adresi ekle",
  "credentialRotation.addOrigin.cancel": "Şimdi değil",
} satisfies Record<keyof typeof en, string>;
