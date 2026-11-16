/**
 * Spanish UI strings: app. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { app as en } from "../en/app.ts";

export const app = {
  "app.name": "Mosaic",
  "app.preAudit":
    "Software previo a una auditoría: aún no lo ha revisado ningún auditor de seguridad externo.",
  "app.previewTag": "Versión preliminar",
  "app.previewBuild":
    "Compilación preliminar, no es una versión final. Sin firmar, previa a una auditoría, solo para probar.",
  "action.cancel": "Cancelar",
  "action.continue": "Continuar",
  "action.retry": "Reintentar",
  "action.copy": "Copiar",
  "action.reveal": "Mostrar",
  "action.hide": "Ocultar",
  "action.lock": "Bloquear",
  "action.details": "Detalles",
  "sync.offline":
    "No se pudo conectar con tu servidor de sincronización. Tus cambios están guardados en este dispositivo y se sincronizarán cuando vuelvas a tener conexión.",
  "app.loading": "Abriendo Mosaic…",
  "app.skipToContent": "Saltar al contenido",
  "action.save": "Guardar",
  "action.close": "Cerrar",
  "action.showPassword": "Mostrar contraseña",
  "action.hidePassword": "Ocultar contraseña",
  "action.back": "Atrás",
  "action.confirm": "Confirmar",
  "action.edit": "Editar",
  "action.delete": "Mover a la papelera",
  "action.restore": "Restaurar",
  "action.purge": "Eliminar definitivamente",
  "action.generate": "Generar",
  "action.useThis": "Usar esta",
  "action.done": "Listo",
  "nav.main": "Navegación principal",
  "nav.vault": "Caja fuerte",
  "nav.security": "Seguridad",
  "nav.generator": "Generador",
  "nav.settings": "Ajustes",
  "nav.trash": "Papelera",
  "nav.breadcrumb": "Ruta de navegación",
  "nav.securityCount":
    "{count, plural, one {# hallazgo por revisar} other {# hallazgos por revisar}}",
  "sync.status.local": "Guardado en este dispositivo",
  "sync.status.upToDate": "Sincronizado",
  "sync.status.attention": "Requiere atención",
  "sync.status.syncing": "Sincronizando…",
  "sync.status.offline": "Sin conexión",
  "sync.status.error": "Error de sincronización",
  "action.moreActions": "Más acciones",
  "action.regenerate": "Volver a generar",
} satisfies Record<keyof typeof en, string>;
