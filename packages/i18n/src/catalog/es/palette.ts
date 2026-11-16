/**
 * Spanish UI strings: palette. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { palette as en } from "../en/palette.ts";

export const palette = {
  "palette.title": "Paleta de comandos",
  "palette.search": "Buscar elementos y comandos",
  "palette.items": "Elementos",
  "palette.commands": "Comandos",
  "palette.empty": "Sin resultados. Prueba con otra palabra.",
  "palette.cmd.newItem": "Nuevo elemento",
  "palette.cmd.generator": "Abrir el generador",
  "palette.cmd.security": "Abrir el informe de seguridad",
  "palette.cmd.settings": "Abrir los ajustes",
  "palette.cmd.import": "Importar desde otra app",
  "palette.cmd.share": "Compartir caja fuerte…",
  "palette.cmd.invites": "Invitaciones",
  "palette.cmd.lock": "Bloquear Mosaic",
  "palette.open": "Abrir la paleta de comandos",
  "palette.hint.move": "para moverte",
  "palette.hint.open": "para abrir",
  "palette.hint.close": "para cerrar",
} satisfies Record<keyof typeof en, string>;
