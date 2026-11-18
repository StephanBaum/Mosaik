/**
 * Turkish UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Turkish: one, other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "Komut paleti",
  "palette.search": "Öğeleri ve komutları ara",
  "palette.items": "Öğeler",
  "palette.commands": "Komutlar",
  "palette.empty": "Eşleşen bir şey yok. Başka bir sözcük deneyin.",
  "palette.cmd.newItem": "Yeni öğe",
  "palette.cmd.generator": "Oluşturucuyu aç",
  "palette.cmd.security": "Güvenlik raporunu aç",
  "palette.cmd.settings": "Ayarları aç",
  "palette.cmd.import": "Başka bir uygulamadan içe aktar",
  "palette.cmd.share": "Kasayı paylaş…",
  "palette.cmd.invites": "Davetler",
  "palette.cmd.lock": "Mosaic’i kilitle",
  "palette.open": "Komut paletini aç",
  "palette.hint.move": "gezinmek için",
  "palette.hint.open": "açmak için",
  "palette.hint.close": "kapatmak için",
} satisfies Record<keyof typeof en, string>;
