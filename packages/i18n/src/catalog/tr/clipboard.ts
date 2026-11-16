/**
 * Turkish UI strings: clipboard. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { clipboard as en } from "../en/clipboard.ts";

export const clipboard = {
  "clipboard.copied": "Kopyalandı. {seconds, plural, one {# saniye} other {# saniye}} sonra temizlenecek.",
  "clipboard.cleared": "Pano temizlendi.",
  "toast.copied": "Kopyalandı. {seconds, plural, one {# saniye} other {# saniye}} sonra temizlenecek.",
  "toast.copiedKept": "Kopyalandı. Mosaic panoyu temizlemeyecek.",
  "toast.clipboardCleared": "Pano temizlendi.",
  "toast.saved": "Kaydedildi.",
  "toast.trashed": "Çöp Kutusu’na taşındı.",
  "toast.restored": "Geri yüklendi.",
  "toast.purged": "Kalıcı olarak silindi.",
  "clipboard.clearNow": "Şimdi temizle",
  "toast.dismiss": "Kapat",
  "toast.region": "Bildirimler",
} satisfies Record<keyof typeof en, string>;
