/**
 * Spanish UI strings: credentialRotation. Same keys as English (checked by the type).
 * ICU plural categories of Spanish: one, many, other (CLDR; `other` is required).
 */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "Vistas de seguridad",
  "credentialRotation.tab.overview": "Resumen",
  "credentialRotation.tab.rotate": "Cambiar ahora",
  "credentialRotation.tab.dueHidden": ", {count} por cambiar",
  "credentialRotation.queue.title": "Cambiar ahora",
  "credentialRotation.queue.intro":
    "Las contraseñas que más conviene cambiar primero. Mosaic te guía, pero cada contraseña la cambias tú mismo en el sitio.",
  "credentialRotation.queue.empty": "No hay nada que cambiar ahora mismo.",
  "credentialRotation.queue.count":
    "{count, plural, one {# contraseña por cambiar} other {# contraseñas por cambiar}}",
  "credentialRotation.queue.list": "Contraseñas por cambiar",
  "credentialRotation.queue.snoozedHeading": "Pospuestas",
  "credentialRotation.queue.snoozedUntil": "Pospuesta hasta el {date, date, medium}",
  "credentialRotation.queue.reason.weak": "Débil",
  "credentialRotation.queue.reason.reused": "Reutilizada",
  "credentialRotation.queue.reason.old": "Antigua",
  "credentialRotation.queue.reason.breached": "En una filtración",
  "credentialRotation.queue.reason.unfinished": "Cambio sin terminar",
  "credentialRotation.queue.change": "Cambiar contraseña",
  "credentialRotation.queue.continue": "Continuar el cambio",
  "credentialRotation.queue.editItem": "Editar elemento",
  "credentialRotation.queue.manualHint":
    "Mosaic no puede guiarte con esta. Abre el elemento y cámbiala tú mismo.",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "Posponer",
  "credentialRotation.queue.snoozeFor": "Posponer {title}",
  "credentialRotation.queue.snoozeDays": "{days, plural, one {# día} other {# días}}",
  "credentialRotation.queue.snoozed":
    "Pospuesta durante {days, plural, one {# día} other {# días}}.",
  "credentialRotation.queue.dismiss": "Descartar",
  "credentialRotation.queue.dismissFor": "Descartar {title}",
  "credentialRotation.queue.dismissed": "Descartada. Vuelve a aparecer cuando cambie la contraseña.",
  "credentialRotation.queue.undo": "Deshacer",
  "credentialRotation.panel.title": "Cambiar la contraseña de {title}",
  "credentialRotation.panel.step": "Paso {current} de {total}",
  "credentialRotation.panel.stays":
    "El cambio de {title} sigue abierto. La contraseña nueva se conserva en el elemento hasta que elijas Funcionó o No funcionó.",
  "credentialRotation.panel.cancel": "Cancelar",
  "credentialRotation.steps.new.title": "Elige una contraseña nueva",
  "credentialRotation.steps.new.intro":
    "Mosaic guarda la contraseña nueva en este elemento antes de que la uses en ningún sitio, para que no se pierda. La contraseña actual se mantiene hasta que lo confirmes.",
  "credentialRotation.steps.new.label": "Contraseña nueva",
  "credentialRotation.steps.new.use": "Usar esta contraseña",
  "credentialRotation.steps.new.saving": "Guardando…",
  "credentialRotation.steps.site.title": "Cámbiala en el sitio",
  "credentialRotation.steps.site.pageLabel": "Página de cambio",
  "credentialRotation.steps.site.body":
    "Abre la página de contraseña del sitio en tu navegador, haz clic en Mosaic en la barra de herramientas y elige Rellenar formulario de cambio. Mosaic nunca envía el formulario.",
  "credentialRotation.steps.site.copyLink": "Copiar enlace a la página de cambio",
  "credentialRotation.steps.site.linkCopied": "Enlace copiado.",
  "credentialRotation.steps.site.linkHint":
    "El enlace no es un secreto. Si el sitio no tiene una página en esa dirección, busca a mano sus ajustes de contraseña.",
  "credentialRotation.steps.site.noLink":
    "Este inicio de sesión no tiene una dirección https, así que Mosaic no puede sugerir una página de cambio. Cambia la contraseña en el sitio a mano.",
  "credentialRotation.steps.site.copyPassword": "Copiar contraseña nueva",
  "credentialRotation.steps.site.next": "Ya lo probé",
  "credentialRotation.steps.result.title": "¿El sitio aceptó la contraseña nueva?",
  "credentialRotation.steps.done.failed": "Contraseña actual conservada",
  "credentialRotation.steps.done.notSure": "Cambio sin cerrar",
  "credentialRotation.steps.result.body":
    "Mosaic no puede ver el sitio. Cuéntale qué pasó para que la contraseña correcta se quede en este elemento. En ningún caso se elimina nada.",
  "credentialRotation.steps.result.working": "Guardando…",
  "credentialRotation.result.worked": "Funcionó",
  "credentialRotation.result.failed": "No funcionó",
  "credentialRotation.result.notSure": "Aún no estoy seguro",
  "credentialRotation.result.workedDone":
    "Listo. La contraseña nueva es ahora la contraseña de este elemento. La anterior está en su historial.",
  "credentialRotation.result.failedDone":
    "Se mantiene la contraseña actual. La nueva se guarda en el historial como no confirmada, por si el sitio la aplicó de todos modos.",
  "credentialRotation.result.notSureDone":
    "El cambio sigue abierto en este elemento. Vuelve cuando lo sepas.",
  "credentialRotation.banner.title": "Cambio de contraseña iniciado el {date, date, medium}",
  "credentialRotation.banner.prepared":
    "La contraseña nueva se conserva aquí hasta que lo confirmes. Ninguna página la ha recibido aún.",
  "credentialRotation.banner.delivered":
    "La contraseña nueva se conserva aquí hasta que lo confirmes. La extensión del navegador la rellenó en el formulario del sitio.",
  "credentialRotation.banner.reveal": "Mostrar contraseña nueva",
  "credentialRotation.banner.hide": "Ocultar contraseña nueva",
  "credentialRotation.banner.copy": "Copiar contraseña nueva",
  "credentialRotation.banner.newPassword": "Contraseña nueva",
  "credentialRotation.banner.copyLink": "Copiar enlace a la página de cambio",
  "credentialRotation.history.title": "Historial de contraseñas",
  "credentialRotation.history.list": "Contraseñas anteriores",
  "credentialRotation.history.entry": "Reemplazada el {date, date, medium}",
  "credentialRotation.history.unconfirmed": "Sin confirmar",
  "credentialRotation.history.unconfirmedEntry": "Probada el {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "Puede que el sitio haya aplicado esta. Restáurala si falla el inicio de sesión con la contraseña actual.",
  "credentialRotation.history.reveal": "Mostrar la contraseña del {date, date, medium}",
  "credentialRotation.history.hide": "Ocultar la contraseña del {date, date, medium}",
  "credentialRotation.history.restore": "Restaurar",
  "credentialRotation.history.restoreFor": "Restaurar la contraseña del {date, date, medium}",
  "credentialRotation.history.restored": "Restaurada. La contraseña anterior está en el historial.",
  "credentialRotation.policy.label": "Recordarme cambiar esta contraseña",
  "credentialRotation.policy.none": "Nunca",
  "credentialRotation.policy.every": "Cada {days} días",
  "credentialRotation.policy.help":
    "Un recordatorio pone el elemento en la lista Cambiar ahora. Mosaic no envía ninguna notificación ni hace ninguna solicitud por ello.",
  "credentialRotation.url.help":
    "Opcional. La dirección https de la página de contraseña del sitio, dentro del mismo sitio que este inicio de sesión. Mosaic la abre solo cuando eliges Abrir página de cambio en la extensión del navegador.",
  "credentialRotation.url.invalid":
    "Mosaic no usará esta dirección. Debe empezar por https:// y estar en el mismo sitio que este inicio de sesión.",
  "credentialRotation.settings.title": "Recordatorios de contraseñas",
  "credentialRotation.settings.body":
    "Mosaic puede decirte, tras desbloquear, cuántas contraseñas conviene cambiar.",
  "credentialRotation.settings.reminders":
    "Mostrar cuántas contraseñas hay por cambiar tras desbloquear",
  "credentialRotation.settings.remindersHint":
    "Solo un mensaje dentro de la app. Sin notificación ni solicitud de red.",
  "credentialRotation.reminder.toast":
    "{count, plural, one {# contraseña por cambiar} other {# contraseñas por cambiar}}",
  "credentialRotation.reminder.show": "Mostrar",
  "credentialRotation.dialog.beginTitle": "¿Iniciar un cambio de contraseña?",
  "credentialRotation.dialog.beginBody":
    "{browser} quiere cambiar la contraseña de {title} en {host}. Mosaic guarda la contraseña nueva con este inicio de sesión y conserva la actual hasta que confirmes que el sitio la aceptó.",
  "credentialRotation.dialog.site": "Sitio web",
  "credentialRotation.dialog.login": "Inicio de sesión",
  "credentialRotation.dialog.password": "Contraseña nueva",
  "credentialRotation.dialog.hidden": "Oculta",
  "credentialRotation.dialog.show": "Mostrar",
  "credentialRotation.dialog.hide": "Ocultar",
  "credentialRotation.dialog.regenerate": "Generar otra",
  "credentialRotation.dialog.regenerated": "Se generó una contraseña nueva.",
  "credentialRotation.dialog.allow": "Usar esta contraseña",
  "credentialRotation.dialog.cancel": "Cancelar",
  "credentialRotation.dialog.decideLater": "Decidir más tarde",
  "credentialRotation.dialog.untitled": "este inicio de sesión",
  "credentialRotation.dialog.attempt":
    "Iniciado el {at, date, medium}, {at, time, short} · {stage}{n, plural, =0 {} one { · # otro cambio abierto} other { · # otros cambios abiertos}}",
  "credentialRotation.dialog.stagePrepared": "ninguna página tiene aún la contraseña nueva",
  "credentialRotation.dialog.stageDelivered": "la contraseña nueva se rellenó en una página",
  "credentialRotation.dialog.resultTitle": "¿{host} aceptó la contraseña nueva?",
  "credentialRotation.dialog.resultBody":
    "{browser} indica que el cambio funcionó. Si lo confirmas, la contraseña nueva pasa a ser la contraseña de {title} y la actual se mueve a su historial.",
  "credentialRotation.dialog.confirm": "Funcionó",
  "credentialRotation.dialog.keepOld": "Conservar la contraseña anterior",
  "credentialRotation.dialog.failedTitle": "¿Conservar la contraseña anterior?",
  "credentialRotation.dialog.failedBody":
    "{browser} indica que el cambio no funcionó. Mosaic conserva la contraseña actual de {title}. El valor nuevo se queda en su historial, por si el sitio lo cambió de todos modos.",
  "credentialRotation.addOrigin.title": "¿Añadir {host} a {title}?",
  "credentialRotation.addOrigin.body":
    "Mosaic también rellenará este inicio de sesión en {host}. Añádelo solo si {host} es realmente parte del mismo sitio.",
  "credentialRotation.addOrigin.idn":
    "Esta dirección usa caracteres especiales. Tu navegador puede mostrarla como {unicode}. Revísala con atención.",
  "credentialRotation.addOrigin.add": "Añadir dirección",
  "credentialRotation.addOrigin.cancel": "Ahora no",
} satisfies Record<keyof typeof en, string>;
