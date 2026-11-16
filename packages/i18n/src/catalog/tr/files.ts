/**
 * Turkish UI strings: files. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { files as en } from "../en/files.ts";

export const files = {
  "files.title": "Dosyalar",
  "files.add": "Dosya ekle…",
  "files.empty": "Henüz dosya yok.",
  "files.previewLoading": "Önizleme hazırlanıyor…",
  "files.size.b": "{value} B",
  "files.size.kb": "{value} KB",
  "files.size.mb": "{value} MB",
  "files.addedAt": "Eklenme: {at, date, medium}",
  "files.preview": "{name} önizlemesi",
  "files.saveAs": "{name} dosyasını farklı kaydet…",
  "files.saveAsShort": "Farklı kaydet…",
  "files.remove": "{name} dosyasını kaldır",
  "files.close": "Kapat",
  "files.previewAlt": "{name} önizlemesi",
  "files.previewOnlyImages":
    "Önizleme PNG ve JPEG görselleri için çalışır. Diğer dosyaları açmak için Farklı kaydet’i kullanın.",
  "files.notDownloaded": "Bu dosya henüz bu aygıtta değil.",
  "files.editorNote": "Dosyalar, ekler veya kaldırmanızla birlikte kaydedilir.",
  "files.removeConfirm.title": "{name} dosyası kaldırılsın mı?",
  "files.removeConfirm.body":
    "Dosya, bu öğeyi eşitleyen tüm aygıtlardan silinir. Bu işlem geri alınamaz.",
  "files.removeConfirm.confirm": "Dosyayı kaldır",
  "files.removeConfirm.keep": "Dosyayı tut",
  "toast.fileAdded": "Dosya eklendi.",
  "toast.fileSaved": "Dosya kaydedildi.",
  "toast.fileRemoved": "Dosya kaldırıldı.",
} satisfies Record<keyof typeof en, string>;
