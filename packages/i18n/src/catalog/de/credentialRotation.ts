/** German UI strings: credential rotation (M7b). Informal "du". Same keys as English (checked by the type). */
import type { credentialRotation as en } from "../en/credentialRotation.ts";

export const credentialRotation = {
  "credentialRotation.tablist": "Ansichten der Sicherheit",
  "credentialRotation.tab.overview": "Übersicht",
  "credentialRotation.tab.rotate": "Jetzt ändern",
  "credentialRotation.tab.rotateCount": "Jetzt ändern, {count} zu ändern",

  "credentialRotation.queue.title": "Jetzt ändern",
  "credentialRotation.queue.intro":
    "Die Passwörter, die du zuerst ändern solltest. Mosaic führt dich, aber du änderst jedes Passwort selbst auf der Website.",
  "credentialRotation.queue.empty": "Im Moment gibt es nichts zu ändern.",
  "credentialRotation.queue.count":
    "{count, plural, one {# Passwort zu ändern} other {# Passwörter zu ändern}}",
  "credentialRotation.queue.list": "Passwörter, die du ändern solltest",
  "credentialRotation.queue.snoozedHeading": "Zurückgestellt",
  "credentialRotation.queue.snoozedUntil": "Zurückgestellt bis {date, date, medium}",
  "credentialRotation.queue.reason.weak": "Schwach",
  "credentialRotation.queue.reason.reused": "Mehrfach verwendet",
  "credentialRotation.queue.reason.old": "Alt",
  "credentialRotation.queue.reason.breached": "In einem Datenleck",
  "credentialRotation.queue.reason.unfinished": "Änderung nicht abgeschlossen",
  "credentialRotation.queue.change": "Passwort ändern",
  "credentialRotation.queue.continue": "Änderung fortsetzen",
  "credentialRotation.queue.editItem": "Eintrag bearbeiten",
  "credentialRotation.queue.manualHint":
    "Dieses kann Mosaic nicht begleiten. Öffne den Eintrag und ändere es selbst.",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "Zurückstellen",
  "credentialRotation.queue.snoozeFor": "{title} zurückstellen",
  "credentialRotation.queue.snoozeDays": "{days, plural, one {# Tag} other {# Tage}}",
  "credentialRotation.queue.snoozed": "Für {days, plural, one {# Tag} other {# Tage}} zurückgestellt.",
  "credentialRotation.queue.dismiss": "Ausblenden",
  "credentialRotation.queue.dismissFor": "{title} ausblenden",
  "credentialRotation.queue.dismissed": "Ausgeblendet. Es kommt zurück, wenn sich das Passwort ändert.",

  "credentialRotation.panel.title": "Passwort für {title} ändern",
  "credentialRotation.panel.step": "Schritt {current} von {total}",
  "credentialRotation.panel.cancel": "Abbrechen",

  "credentialRotation.steps.new.title": "Neues Passwort wählen",
  "credentialRotation.steps.new.intro":
    "Mosaic speichert das neue Passwort in diesem Eintrag, bevor du es irgendwo verwendest. So geht es nicht verloren. Das aktuelle Passwort bleibt, bis du bestätigst.",
  "credentialRotation.steps.new.label": "Neues Passwort",
  "credentialRotation.steps.new.use": "Dieses Passwort verwenden",
  "credentialRotation.steps.new.saving": "Wird gespeichert …",
  "credentialRotation.steps.site.title": "Auf der Website ändern",
  "credentialRotation.steps.site.body":
    "Öffne die Passwortseite der Website in deinem Browser, klicke in der Symbolleiste auf Mosaic und wähle „Änderungsformular ausfüllen“. Mosaic sendet das Formular nie ab.",
  "credentialRotation.steps.site.copyLink": "Link zur Änderungsseite kopieren",
  "credentialRotation.steps.site.linkCopied": "Link kopiert.",
  "credentialRotation.steps.site.linkHint":
    "Der Link ist kein Geheimnis. Wenn die Website unter dieser Adresse keine Seite hat, suche die Passworteinstellungen selbst.",
  "credentialRotation.steps.site.noLink":
    "Dieser Login hat keine https-Adresse, deshalb kann Mosaic keine Änderungsseite vorschlagen. Ändere das Passwort selbst auf der Website.",
  "credentialRotation.steps.site.copyPassword": "Neues Passwort kopieren",
  "credentialRotation.steps.site.next": "Ich habe es versucht",
  "credentialRotation.steps.result.title": "Hat die Website das neue Passwort akzeptiert?",
  "credentialRotation.steps.result.body":
    "Mosaic sieht die Website nicht. Sag Mosaic, was passiert ist, damit das richtige Passwort im Eintrag bleibt. Gelöscht wird in keinem Fall etwas.",
  "credentialRotation.steps.result.working": "Wird gespeichert …",

  "credentialRotation.result.worked": "Es hat geklappt",
  "credentialRotation.result.failed": "Es hat nicht geklappt",
  "credentialRotation.result.notSure": "Noch unklar",
  "credentialRotation.result.workedDone":
    "Fertig. Das neue Passwort ist jetzt das Passwort dieses Eintrags. Das alte steht im Verlauf.",
  "credentialRotation.result.failedDone":
    "Das aktuelle Passwort bleibt. Das neue liegt im Verlauf als nicht bestätigt, falls die Website es doch übernommen hat.",
  "credentialRotation.result.notSureDone":
    "Die Änderung bleibt an diesem Eintrag offen. Komm zurück, wenn du es weißt.",

  "credentialRotation.banner.title": "Passwortänderung am {date, date, medium} begonnen",
  "credentialRotation.banner.prepared":
    "Das neue Passwort liegt hier, bis du bestätigst. Noch keine Seite hat es erhalten.",
  "credentialRotation.banner.delivered":
    "Das neue Passwort liegt hier, bis du bestätigst. Die Browser-Erweiterung hat es ins Formular der Website eingetragen.",
  "credentialRotation.banner.reveal": "Neues Passwort anzeigen",
  "credentialRotation.banner.hide": "Neues Passwort verbergen",
  "credentialRotation.banner.copy": "Neues Passwort kopieren",
  "credentialRotation.banner.newPassword": "Neues Passwort",
  "credentialRotation.banner.copyLink": "Link zur Änderungsseite kopieren",

  "credentialRotation.history.title": "Passwortverlauf",
  "credentialRotation.history.list": "Frühere Passwörter",
  "credentialRotation.history.entry": "Ersetzt am {date, date, medium}",
  "credentialRotation.history.unconfirmed": "Nicht bestätigt",
  "credentialRotation.history.unconfirmedEntry": "Versucht am {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "Die Website hat dieses vielleicht übernommen. Stelle es wieder her, wenn die Anmeldung mit dem aktuellen Passwort fehlschlägt.",
  "credentialRotation.history.reveal": "Passwort vom {date, date, medium} anzeigen",
  "credentialRotation.history.hide": "Passwort vom {date, date, medium} verbergen",
  "credentialRotation.history.restore": "Wiederherstellen",
  "credentialRotation.history.restoreFor": "Passwort vom {date, date, medium} wiederherstellen",
  "credentialRotation.history.restored": "Wiederhergestellt. Das vorherige Passwort steht im Verlauf.",

  "credentialRotation.policy.label": "Erinnere mich, dieses Passwort zu ändern",
  "credentialRotation.policy.none": "Nie",
  "credentialRotation.policy.every": "Alle {days} Tage",
  "credentialRotation.policy.help":
    "Eine Erinnerung setzt den Eintrag auf die Liste „Jetzt ändern“. Mosaic sendet keine Benachrichtigung und stellt dafür keine Anfrage.",

  "credentialRotation.url.help":
    "Optional. Die https-Adresse der Passwortseite, auf derselben Website wie dieser Login. Mosaic öffnet sie nie für dich.",

  "credentialRotation.settings.title": "Passwort-Erinnerungen",
  "credentialRotation.settings.body":
    "Mosaic kann dir nach dem Entsperren sagen, wie viele Passwörter du ändern solltest.",
  "credentialRotation.settings.reminders": "Nach dem Entsperren anzeigen, wie viele Passwörter zu ändern sind",
  "credentialRotation.settings.remindersHint": "Nur eine Meldung in der App. Keine Benachrichtigung, keine Netzwerkanfrage.",

  "credentialRotation.reminder.toast":
    "{count, plural, one {# Passwort zu ändern} other {# Passwörter zu ändern}}",
  "credentialRotation.reminder.show": "Anzeigen",
} as const satisfies Record<keyof typeof en, string>;
