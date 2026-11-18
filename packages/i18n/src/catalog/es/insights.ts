/**
 * Spanish UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "Seguridad",
  "insights.intro":
    "Las comprobaciones se hacen en este dispositivo. Última comprobación: {checkedAt, date, medium} a las {checkedAt, time, short}, sobre {count, plural, one {# elemento} other {# elementos}}.",
  "insights.checkAgain": "Comprobar de nuevo",
  "insights.summary": "Resumen",
  "insights.fixFirst": "Corrige esto primero",
  "insights.showAll": "Mostrar {count, plural, one {# hallazgo} other {# hallazgos}}",
  "insights.empty": "No se encontraron problemas. Mosaic vuelve a comprobar cada vez que abres esta página.",
  "insights.kind.weak": "Débiles",
  "insights.kind.weak.description": "Fáciles de adivinar",
  "insights.kind.reused": "Reutilizadas",
  "insights.kind.reused.description": "{groups, plural, one {# grupo} other {# grupos}}",
  "insights.kind.old": "Antiguas (más de 1 año)",
  "insights.kind.old.description": "Sin cambiar desde hace más de un año",
  "insights.kind.missingTotp": "Sin 2FA",
  "insights.kind.missingTotp.description": "El sitio lo admite, pero no lo has configurado",
  "insights.kind.insecureUrl": "HTTP sin protección",
  "insights.kind.insecureUrl.description": "Envía tu contraseña sin cifrar",
  "insights.kind.breached": "En una filtración",
  "insights.kind.breached.description": "Detectada por la comprobación de filtraciones opcional",
  "insights.finding.breached":
    "Esta contraseña aparece en una filtración de datos conocida. Cámbiala ahora, en todos los sitios donde la uses.",
  "insights.finding.weak": "Es fácil de adivinar. Cámbiala por una contraseña generada.",
  "insights.finding.reused":
    "También la usan {count, plural, one {# elemento más} other {# elementos más}}. Usa una contraseña distinta para cada sitio.",
  "insights.finding.old":
    "No se ha cambiado desde hace más de un año. Cámbiala si sigues usando esta cuenta.",
  "insights.finding.missingTotp":
    "{site} ofrece códigos de verificación en dos pasos. Actívalos allí y guarda el código en Mosaic.",
  "insights.finding.insecureUrl":
    "Usa HTTP sin cifrar: {url}. Cambia a https:// si el sitio lo admite.",
  "insights.severity.high": "Alta",
  "insights.severity.medium": "Media",
  "insights.severity.low": "Baja",
  "insights.action.changePassword": "Cambiar contraseña",
  "insights.action.setUpTotp": "Configurar 2FA",
  "insights.action.reviewUrl": "Revisar",
  "insights.attribution.twoFactor": "Datos de 2FA: 2FA Directory (2fa.directory), licencia MIT.",
  "insights.counts": "{count, plural, =0 {Sin problemas} one {# problema} other {# problemas}}",
  "insights.documentation": "Cómo activarlo: {url}",
  "insights.actionFor": "{action}: {title}",
} satisfies Record<keyof typeof en, string>;
