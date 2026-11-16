/**
 * Brazilian Portuguese UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Brazilian Portuguese: one, many, other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "Paleta de comandos",
  "palette.search": "Buscar itens e comandos",
  "palette.items": "Itens",
  "palette.commands": "Comandos",
  "palette.empty": "Nada corresponde. Tente outra palavra.",
  "palette.cmd.newItem": "Novo item",
  "palette.cmd.generator": "Abrir o gerador",
  "palette.cmd.security": "Abrir o relatório de segurança",
  "palette.cmd.settings": "Abrir configurações",
  "palette.cmd.import": "Importar de outro app",
  "palette.cmd.share": "Compartilhar cofre…",
  "palette.cmd.invites": "Convites",
  "palette.cmd.lock": "Bloquear o Mosaic",
  "palette.open": "Abrir paleta de comandos",
  "palette.hint.move": "para mover",
  "palette.hint.open": "para abrir",
  "palette.hint.close": "para fechar",
} satisfies Record<keyof typeof en, string>;
