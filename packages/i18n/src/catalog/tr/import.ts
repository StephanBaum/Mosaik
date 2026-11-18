/**
 * Turkish UI strings: import. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { importMessages as en } from "../en/import.ts";

export const importMessages = {
  "import.format.enpass": "Enpass (.json)",
  "import.format.bitwarden": "Bitwarden (.json, şifrelenmemiş)",
  "import.format.keepass": "KeePass (.kdbx)",
  "import.format.onePassword": "1Password (.1pux)",
  "import.format.cxf": "FIDO Credential Exchange (.json): geçiş anahtarları ve parolalar",
  "import.format.dashlane": "Dashlane (.zip)",
  "import.format.keeper": "Keeper (.json)",
  "import.format.keeperCsv": "Keeper (.csv)",
  "import.format.protonPass": "Proton Pass (.zip veya .json, şifrelenmemiş)",
  "import.format.csv": "CSV dosyası (.csv)",
  "import.keepassPassword": "KeePass veritabanının parolası",
  "import.preview.ready":
    "{count, plural, one {# öğe içe aktarmaya hazır.} other {# öğe içe aktarmaya hazır.}}",
  "import.preview.attachments": "{count, plural, one {# ek} other {# ek}}",
  "import.preview.duplicate":
    "Buna benzer bir öğeniz zaten var. İşareti kaldırıldı, bu yüzden iki kez içe aktarılmayacak.",
  "import.preview.duplicateInFile": "Bu dosyada aynı öğe iki kez var. Kopyanın işareti kaldırıldı.",
  "import.warning.unmappedFields":
    "Bazı alanların Mosaic’te karşılığı yok. Özel alan olarak korundular.",
  "import.warning.attachmentsSkipped":
    "Bazı ekler 50 MiB’den büyük, 1 GiB’lik ek depolama alanınıza sığmıyor veya dışa aktarmada eksik. İçe aktarmadan sonra elle ekleyin.",
  "import.warning.entriesSkipped":
    "Bazı kayıtlar okunamadı ve dışarıda bırakılacak. Eski parola yöneticinizde kontrol edin.",
  "import.warning.otpUnreadable":
    "Bazı tek kullanımlık kod ayarları tanınmadı. Gizli alan olarak korundular; kodu bunlardan yeniden kurun.",
  "import.warning.otpWeak":
    "Bazı tek kullanımlık kodların gizli anahtarı 80 bitten kısa, bu yüzden kodların tahmin edilmesi daha kolay. Yine de içe aktarıldılar; mümkün olduğunda yeni bir gizli anahtarla yeniden kurun.",
  "import.warning.passkeysNotImported":
    "Bu dışa aktarma biçimi geçiş anahtarlarının özel anahtarlarını içermiyor, bu yüzden içe aktarılamazlar. Bu öğelerin geri kalanı içe aktarılır.",
  "import.warning.passkeyUnreadable": "Bazı geçiş anahtarları okunamadı ve dışarıda bırakıldı.",
  "import.warning.passkeyUnsupported":
    "Bazı geçiş anahtarları Mosaic’in desteklemediği bir anahtar türü kullanıyor (yalnızca ES256 çalışır) ve dışarıda bırakıldı.",
  "import.warning.passkeysSkipped":
    "Bazı geçiş anahtarları çok fazla oldukları için dışarıda bırakıldı (öğe başına en fazla 100, dosya başına 10.000).",
  "import.warning.passkeysKeepOldApp": "Geçiş anahtarlarını taşıyana kadar eski uygulamanızı saklayın.",
  "import.entry.unmappedFields":
    "Bazı alanların Mosaic’te karşılığı yok. Özel alan olarak korundular.",
  "import.entry.otpUnreadable":
    "Tek kullanımlık kod ayarı tanınmadı. Gizli alan olarak korundu; kodu ondan yeniden kurun.",
  "import.entry.otpWeak":
    "Tek kullanımlık kodun gizli anahtarı 80 bitten kısa, bu yüzden kodun tahmin edilmesi daha kolay. Yine de içe aktarıldı; mümkün olduğunda yeni bir gizli anahtarla yeniden kurun.",
  "import.entry.attachmentTooLarge":
    "Bir ek 50 MiB’den büyük ve dışarıda bırakıldı. İçe aktarmadan sonra elle ekleyin.",
  "import.entry.attachmentMissing":
    "Bir ek dışa aktarmada eksik. İçe aktarmadan sonra elle ekleyin.",
  "import.entry.attachmentQuota":
    "Bir ek 1 GiB’lik ek depolama alanınıza sığmıyor ve dışarıda bırakıldı.",
  "import.entry.passkeysNotImported":
    "Bu dışa aktarma biçimi bu öğenin geçiş anahtarının özel anahtarını içermiyor, bu yüzden içe aktarılamaz. Taşıyana kadar eski uygulamanızı saklayın.",
  "import.entry.passkeyUnreadable":
    "Bu öğenin bir geçiş anahtarı okunamadı ve dışarıda bırakıldı. Taşıyana kadar eski uygulamanızı saklayın.",
  "import.entry.passkeyUnsupported":
    "Bu öğenin bir geçiş anahtarı Mosaic’in desteklemediği bir anahtar türü kullanıyor (yalnızca ES256 çalışır) ve dışarıda bırakıldı. Taşıyana kadar eski uygulamanızı saklayın.",
  "import.entry.historyTruncated": "Yalnızca en son 32 eski parola korundu.",
  "import.entry.tagsDropped": "Boş veya aşırı uzun etiketler dışarıda bırakıldı.",
  "import.entry.archived": "Eski uygulamanızda arşivlenmişti. \"archived\" etiketini alır.",
  "import.entry.unknownCategory": "Bilinmeyen öğe türü. Güvenli not olarak içe aktarılır.",
  "import.entry.titleCleaned":
    "Başlıkta görünmez veya metni ters çeviren karakterler vardı. Kaldırıldılar.",
  "import.entry.passkeysLimit":
    "Bu öğede tek bir içe aktarmanın alabileceğinden fazla geçiş anahtarı var (öğe başına 100, dosya başına 10.000). Fazla geçiş anahtarları dışarıda bırakıldı. Taşıyana kadar eski uygulamanızı saklayın.",
  "import.entry.fieldsDropped":
    "Bu öğede Mosaic’in koruyabileceğinden fazla alan var. Fazla alanlar dışarıda bırakıldı.",
  "import.entry.notesTruncated":
    "Bu öğenin notları, bir öğenin alabileceğinden uzun. Sığmayan kısım dışarıda bırakıldı.",
  "import.entry.idnHost":
    "Bu adres alan adında özel karakterler kullanıyor ve başka bir siteye benzeyebilir. Güvenmeden önce kontrol edin.",
  "import.entry.urlKeptAsText": "Bu web adresi bağlantı olarak kullanılamadı. Metin olarak korundu.",
  "import.skip.malformed": "Bu kayıt hasarlı ve okunamıyor.",
  "import.skip.empty": "Bu kayıt boş.",
  "import.skip.tooLarge": "Bu kayıt, bir öğenin alabileceği en büyük boyut olan 1 MiB’den büyük.",
  "import.result":
    "{imported, plural, one {# öğe içe aktarıldı.} other {# öğe içe aktarıldı.}} {skipped, plural, =0 {} one {# dışarıda bırakıldı.} other {# dışarıda bırakıldı.}}",
  "import.flow.title": "İçe aktar",
  "import.flow.formatStep": "Nereden içe aktarıyorsunuz?",
  "import.flow.chooseFile": "Dışa aktarma dosyasını seçin",
  "import.flow.fileChosen": "Dosya: {name}",
  "import.flow.password": "KeePass veritabanının parolası",
  "import.flow.preview": "Önizleme",
  "import.flow.previewBusy": "Dosya okunuyor. Bu biraz sürebilir.",
  "import.flow.selectAll": "Tümünü seç",
  "import.flow.target": "Şuraya aktar",
  "import.flow.commit": "{count, plural, one {# öğeyi} other {# öğeyi}} içe aktar",
  "import.flow.entries": "Kayıtlar",
  "import.flow.skipped": "Dışarıda bırakılan",
  "import.flow.attachments": "{count, plural, one {# ek} other {# ek}}",
  "import.flow.cancel": "İçe aktarmayı iptal et",
  "import.flow.noneSelected": "İçe aktarmak için en az bir öğe seçin.",
  "import.flow.resultFailed":
    "{failed, plural, =0 {} one {# öğe kaydedilemedi.} other {# öğe kaydedilemedi.}}",
  "import.flow.deleteReminder":
    "Dışa aktarma dosyasını şimdi silin. Parolalarınızı düz metin olarak içerir.",
  "import.flow.dropHint":
    "Dışa aktarma dosyanızı seçin. Her şey bu aygıtta okunur ve şifrelenir.",
} satisfies Record<keyof typeof en, string>;
