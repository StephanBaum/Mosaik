/**
 * German UI strings: Passwort- und Schlüsseländerungen (M7). Informal "du", plain words. Same keys
 * as English (checked by the type). "Emergency Kit" and "Secret Key" stay as they are. No "Epoche":
 * the account's keys were "erneuert".
 */
import type { rotation as en } from "../en/rotation.ts";

export const rotation = {
  "rotation.section.title": "Passwort und Schlüssel",
  "rotation.section.body":
    "Ändere dein Master-Passwort, hole dir einen neuen Secret Key oder stelle dein ganzes Konto auf neue Schlüssel um. Bei jeder Änderung siehst du vorher, was passiert.",

  "rotation.action.password": "Master-Passwort ändern",
  "rotation.action.password.body":
    "Wähle ein neues Master-Passwort. Dein Secret Key und deine Tresore bleiben, wie sie sind.",
  "rotation.action.password.button": "Ändern …",
  "rotation.action.secretKey": "Secret Key erneuern",
  "rotation.action.secretKey.body":
    "Hol dir einen neuen Secret Key und ein neues Emergency Kit. Nutze das, wenn dein Kit verloren ging oder jemand es gesehen haben könnte.",
  "rotation.action.secretKey.button": "Erneuern …",
  "rotation.action.profile": "Schutz erhöhen",
  "rotation.action.profile.body":
    "Mach das Erraten deines Master-Passworts schwerer, indem jeder Versuch mehr Speicher braucht. Das Entsperren dauert etwas länger.",
  "rotation.action.profile.button": "Erhöhen …",
  "rotation.action.rekey": "Konto neu verschlüsseln",
  "rotation.action.rekey.body":
    "Erneuere alle Schlüssel deines Kontos: einen neuen Secret Key, neue Tresorschlüssel und einen neuen Sicherheitscode. Nutze das, wenn jemand deine Tresordaten zusammen mit deinem Passwort und Secret Key kopiert haben könnte.",
  "rotation.action.rekey.button": "Neu verschlüsseln …",
  "rotation.action.rekey.needsSync":
    "Beim Neuverschlüsseln wird die Kopie auf deinem Sync-Server oder in deinem Ordner erneuert, dafür brauchst du die Synchronisierung. Richte sie zuerst ein.",

  "rotation.title.password": "Master-Passwort ändern",
  "rotation.title.secretKey": "Secret Key erneuern",
  "rotation.title.profile": "Schutz erhöhen",
  "rotation.title.rekey": "Konto neu verschlüsseln",
  "rotation.step": "Schritt {current} von {total}",
  "rotation.back": "Zurück",
  "rotation.stop.title":
    "Änderung abbrechen?",
  "rotation.stop.body":
    "Das eben gespeicherte Emergency Kit funktioniert dann nicht. Behalte dein altes Kit.",
  "rotation.stop.confirm":
    "Änderung abbrechen",
  "rotation.stop.keep":
    "Weitermachen",
  "rotation.next": "Weiter",
  "rotation.close": "Schließen",

  "rotation.review.title.password": "Bevor du dein Passwort änderst",
  "rotation.review.title.secretKey": "Bevor du deinen Secret Key erneuerst",
  "rotation.review.title.profile": "Bevor du den Schutz erhöhst",
  "rotation.review.title.rekey": "Wurde etwas kompromittiert?",
  "rotation.review.changes": "Was sich ändert",
  "rotation.review.stays": "Was gleich bleibt",
  "rotation.review.password.changes":
    "Dein Master-Passwort. Die Schnellentsperrung auf diesem Gerät wird ausgeschaltet, du kannst sie danach wieder einrichten.",
  "rotation.review.password.stays":
    "Dein Secret Key, deine Tresore und Einträge, dein Sicherheitscode und deine Notfallkontakte.",
  "rotation.review.password.devices":
    "Deine anderen Geräte werden von der Synchronisierung abgemeldet. Gib auf jedem einmal das neue Master-Passwort ein, dann läuft sie weiter.",
  "rotation.review.secretKey.changes":
    "Dein Secret Key und dein Emergency Kit. Die Schnellentsperrung auf diesem Gerät wird ausgeschaltet, du kannst sie danach wieder einrichten.",
  "rotation.review.secretKey.stays":
    "Dein Master-Passwort, deine Tresore und Einträge, dein Sicherheitscode und deine Notfallkontakte. Registrierte Sicherheitsschlüssel funktionieren weiter.",
  "rotation.review.secretKey.devices":
    "Deine anderen Geräte werden von der Synchronisierung abgemeldet. Gib auf jedem einmal dein Passwort und den neuen Secret Key aus dem neuen Kit ein.",
  "rotation.review.secretKey.limit":
    "Damit kann man sich nicht mehr mit dem alten Secret Key anmelden. Wer deine Tresordaten schon mit deinem alten Passwort und Secret Key kopiert hat, kann sie weiterhin lesen. Verschlüssele dein Konto neu, um das zu stoppen.",
  "rotation.review.secretKey.compromised": "Es wurde etwas kompromittiert: stattdessen neu verschlüsseln",
  "rotation.review.profile.changes":
    "Wie viel Speicher jeder Entsperrversuch braucht, für dich und für jeden, der rät. Die Schnellentsperrung auf diesem Gerät wird ausgeschaltet, du kannst sie danach wieder einrichten.",
  "rotation.review.profile.stays":
    "Dein Master-Passwort, dein Secret Key, deine Tresore und dein Sicherheitscode. Registrierte Sicherheitsschlüssel funktionieren weiter.",
  "rotation.review.profile.devices":
    "Deine anderen Geräte werden von der Synchronisierung abgemeldet. Gib auf jedem einmal dein Master-Passwort ein, dann läuft sie weiter. Langsamere Geräte brauchen etwas länger zum Entsperren.",
  "rotation.review.rekey.intro":
    "Ein Neuverschlüsseln ist für den Fall gedacht, dass jemand deine Tresordaten und dein Passwort samt Secret Key haben könnte. Wenn du nur ein Gerät verloren hast, reicht es, den Secret Key zu erneuern oder dein Passwort zu ändern.",
  "rotation.review.rekey.yes": "Ja, mein Konto neu verschlüsseln",
  "rotation.review.rekey.no": "Nein, nur den Secret Key erneuern",
  "rotation.review.rekey.limit":
    "Ein Neuverschlüsseln holt nichts zurück, was schon kopiert wurde. Alte Kopien auf Servern und in Backups bleiben mit den alten Schlüsseln verschlüsselt, und die alten Schlüssel öffnen sie weiterhin.",

  "rotation.review.rekey.title": "Was das Neuverschlüsseln macht",
  "rotation.review.rekey.vaults":
    "{count, plural, one {# Tresor, den nur du nutzt, bekommt einen neuen Schlüssel.} other {# Tresore, die nur du nutzt, bekommen neue Schlüssel.}}",
  "rotation.review.rekey.keys":
    "Ein neuer Secret Key, ein neues Emergency Kit, ein neuer Sicherheitscode und ein neuer Kontoschlüssel.",
  "rotation.review.rekey.devices":
    "Deine anderen Geräte werden abgemeldet und können nicht synchronisieren, bis du auf jedem den neuen Secret Key und dein neues Master-Passwort eingibst.",
  "rotation.review.rekey.shared":
    "{count, plural, one {Du teilst # Tresor.} other {Du teilst # Tresore.}} Jeder bekommt einen neuen Schlüssel.",
  "rotation.review.rekey.sharedRow":
    "{name}, {count, plural, one {# Mitglied} other {# Mitglieder}}",
  "rotation.review.rekey.sharedMembers":
    "Bitte jede Person, mit der du teilst, deinen neuen Sicherheitscode zu vergleichen. Bis dahin bekommt sie keine neuen Schlüssel von dir und kann den erneuerten Tresor nicht öffnen.",
  "rotation.review.rekey.memberVaults":
    "{count, plural, one {# Tresor gehört jemand anderem.} other {# Tresore gehören anderen Personen.}} Ihre Besitzer werden gebeten, sie zu erneuern. Bis dahin können deine alten Schlüssel sie noch lesen.",
  "rotation.review.rekey.grants":
    "{count, plural, one {# Person hat Notfallzugriff auf deine Tresore.} other {# Personen haben Notfallzugriff auf deine Tresore.}} Ihr Zugriff wird für die neuen Schlüssel erneuert. Auch sie müssen deinen neuen Sicherheitscode vergleichen.",
  "rotation.review.rekey.quick":
    "Die Schnellentsperrung auf diesem Gerät wird ausgeschaltet. Du kannst sie danach wieder einrichten.",
  "rotation.review.rekey.hardwareKeys":
    "{count, plural, one {# Sicherheitsschlüssel, den du zum Entsperren registriert hast, wird ausgeschaltet.} other {# Sicherheitsschlüssel, die du zum Entsperren registriert hast, werden ausgeschaltet.}} Du kannst sie danach wieder registrieren.",
  "rotation.review.rekey.hardwareVaults":
    "{count, plural, one {# hardwaregeschützter Tresor ist nicht betroffen.} other {# hardwaregeschützte Tresore sind nicht betroffen.}}",
  "rotation.review.rekey.attachments":
    "{count, plural, one {# Anhang ist nicht auf diesem Gerät.} other {# Anhänge sind nicht auf diesem Gerät.}} Sie werden zuerst heruntergeladen, damit sie neu geschützt werden können. Was nicht erreichbar ist, behält den alten Schlüssel.",
  "rotation.review.rekey.folder":
    "Dein Sync-Ordner bekommt eine neue Kontodatei. Andere Geräte finden dort einen Hinweis, dass sich die Schlüssel geändert haben.",
  "rotation.review.rekey.unavailable":
    "Neuverschlüsseln ist nicht möglich: Die Synchronisierung ist nicht eingerichtet.",
  "rotation.review.loading":
    "Mosaic prüft, was ein Neuverschlüsseln betrifft …",
  "rotation.review.rekey.needsClean":
    "Fürs Neuverschlüsseln muss die Synchronisierung aktuell und ohne Warnungen sein. Synchronisiere jetzt, sieh dir Warnungen an und versuche es dann noch einmal.",

  "rotation.credentials.title":
    "Bestätige, dass du es bist",
  "rotation.credentials.password.title": "Deine Master-Passwörter",
  "rotation.credentials.current": "Aktuelles Master-Passwort",
  "rotation.credentials.new": "Neues Master-Passwort",
  "rotation.credentials.confirm": "Neues Master-Passwort noch einmal eingeben",
  "rotation.credentials.mismatch": "Die beiden Passwörter sind verschieden.",
  "rotation.credentials.same": "Das neue Master-Passwort muss sich vom aktuellen unterscheiden.",
  "rotation.credentials.hint":
    "Eine lange Passphrase ist besser als ein kurzes kompliziertes Passwort. Du kannst es nicht wiederherstellen: Mosaic kennt es nicht.",
  "rotation.credentials.code": "6-stelliger Code aus deiner Authenticator-App",
  "rotation.credentials.profile": "Schutzstufe",
  "rotation.credentials.profile.standard": "Standard: 256 MB Speicher pro Versuch",
  "rotation.credentials.profile.strong": "Stark: 512 MB Speicher pro Versuch",
  "rotation.credentials.profile.max": "Maximal: 1 GB Speicher pro Versuch",
  "rotation.credentials.profile.slow":
    "Mit dieser Stufe braucht dieses Gerät zum Entsperren mehr als zwei Sekunden. Ältere Geräte sind noch langsamer.",
  "rotation.credentials.profile.force": "Trotzdem erhöhen",

  "rotation.kit.title": "Speichere dein neues Emergency Kit",
  "rotation.kit.body":
    "Dein neuer Secret Key steht in diesem Kit. Speichere es zuerst: Mosaic ändert dein Konto erst, wenn du bestätigst, dass du es hast, denn ohne es kannst du dich auf einem neuen Gerät nicht anmelden.",
  "rotation.kit.save": "Neues Emergency Kit speichern",
  "rotation.kit.saved": "Gespeichert. Bewahre es sicher auf, getrennt von deinen Geräten.",
  "rotation.kit.cancelled": "Es wurde noch nichts gespeichert.",
  "rotation.kit.verify": "Gib die letzte Gruppe deines neuen Secret Keys aus dem gespeicherten Kit ein.",
  "rotation.kit.confirm": "Kit bestätigen",
  "rotation.kit.mistyped":
    "Das ist nicht das Ende des neuen Secret Keys. Sieh im gespeicherten Kit nach.",
  "rotation.kit.verified": "Kit bestätigt.",

  "rotation.confirm.title.password": "Dein Master-Passwort ändern",
  "rotation.confirm.title.secretKey": "Deinen Secret Key erneuern",
  "rotation.confirm.title.profile": "Den Schutz erhöhen",
  "rotation.confirm.title.rekey": "Dein Konto neu verschlüsseln",
  "rotation.confirm.body.rekey":
    "Dadurch werden deine anderen Geräte abgemeldet und alle Schlüssel erneuert. Lass Mosaic offen, bis es fertig ist.",
  "rotation.confirm.body.secretKey":
    "Dadurch werden deine anderen Geräte von der Synchronisierung abgemeldet. Lass Mosaic offen, bis es fertig ist.",
  "rotation.confirm.verb.password": "Passwort ändern",
  "rotation.confirm.verb.secretKey": "Secret Key erneuern",
  "rotation.confirm.verb.profile": "Schutz erhöhen",
  "rotation.confirm.verb.rekey": "Konto neu verschlüsseln",

  "rotation.run.title":
    "Wird ausgeführt …",
  "rotation.run.keepOpen":
    "Lass Mosaic offen. Mit vielen Anhängen kann das ein paar Minuten dauern.",
  "rotation.progress.label": "Fortschritt",
  "rotation.progress.count": "{done} von {total}",
  "rotation.phase.prepared": "Vorbereitung",
  "rotation.phase.backendStarted": "Dein Konto wird auf die neuen Schlüssel umgestellt",
  "rotation.phase.backendAccepted": "Dein Konto hat die neuen Schlüssel angenommen",
  "rotation.phase.folderWritten": "Der Sync-Ordner hat die neue Kontodatei",
  "rotation.phase.localCommitted": "Dieses Gerät nutzt jetzt die neuen Schlüssel",
  "rotation.phase.attachments": "Anhänge werden neu geschützt",
  "rotation.phase.uploading": "Die geschützten Daten werden gesendet",
  "rotation.phase.done": "Fertig",

  "rotation.done.title.password": "Master-Passwort geändert",
  "rotation.done.title.secretKey": "Secret Key erneuert",
  "rotation.done.title.profile": "Schutz erhöht",
  "rotation.done.title.rekey": "Konto neu verschlüsselt",
  "rotation.done.body.password":
    "Nutze ab jetzt das neue Master-Passwort. Deine anderen Geräte fragen einmal danach.",
  "rotation.done.body.secretKey":
    "Bewahre das neue Emergency Kit sicher auf und lösche das alte. Deine anderen Geräte fragen einmal nach dem neuen Secret Key.",
  "rotation.done.body.profile":
    "Das Entsperren braucht jetzt mehr Speicher. Deine anderen Geräte fragen einmal nach deinem Passwort.",
  "rotation.done.body.rekey":
    "Bewahre das neue Emergency Kit sicher auf und lösche das alte. Deine anderen Geräte fragen einmal nach dem neuen Secret Key und deinem neuen Master-Passwort.",
  "rotation.done.members":
    "Bitte alle, mit denen du einen Tresor teilst, deinen neuen Sicherheitscode zu vergleichen. Bis dahin bekommen sie keine neuen Schlüssel von dir.",
  "rotation.done.quick": "Schnellentsperrung wieder einrichten",
  "rotation.done.quick.body": "Die Schnellentsperrung wurde ausgeschaltet.",
  "rotation.done.hardware": "Sicherheitsschlüssel wieder registrieren",
  "rotation.done.hardware.body":
    "{count, plural, one {# Sicherheitsschlüssel wurde fürs Entsperren ausgeschaltet.} other {# Sicherheitsschlüssel wurden fürs Entsperren ausgeschaltet.}}",
  "rotation.done.unreachable.note":
    "Diese Dateien konnten nicht gelesen werden und wurden nicht neu verschlüsselt. Sie behalten ihren alten Dateischlüssel, bis du sie entfernst oder neu hinzufügst. Wer deine alten Schlüssel und eine Kopie der Datei hat, könnte sie also noch öffnen.",
  "rotation.done.unreachable.file": "{item}: {file}",
  "rotation.done.unreachable.unknownItem": "Unbekannter Eintrag",
  "rotation.done.unreachable.unknownFile": "unbekannte Datei",
  "rotation.done.unreachable.more": "{count, plural, one {und # weitere Datei} other {und # weitere Dateien}}",
  "rotation.done.unfinished":
    "Der letzte Teil läuft später weiter. Wenn Mosaic vorher schließt, wähle beim nächsten Mal „Schlüsseländerung fortsetzen“.",

  "rotation.resume.title": "Schlüsseländerung nicht abgeschlossen",
  "rotation.resume.body.password":
    "Wähle Fortsetzen, um deine Passwörter erneut einzugeben, oder verwirf sie.",
  "rotation.resume.body.secretKey":
    "Das Erneuern deines Secret Keys wurde unterbrochen. Nichts ist verloren. Setze es fort, um es abzuschließen.",
  "rotation.resume.body.rekey":
    "Das Neuverschlüsseln deines Kontos wurde unterbrochen. Nichts ist verloren. Setze es fort, um es abzuschließen.",
  "rotation.resume.body.profile":
    "Das Erhöhen des Schutzes wurde unterbrochen. Nichts ist verloren. Setze es fort, um es abzuschließen.",
  "rotation.resume.continue": "Schlüsseländerung fortsetzen",
  "rotation.resume.discard": "Verwerfen …",
  "rotation.resume.running":
    "Wird fortgesetzt …",
  "rotation.resume.progress":
    "{done} von {total} erledigt. Lass Mosaic offen, bis es fertig ist.",
  "rotation.resume.dialog.title": "Schlüsseländerung fortsetzen",
  "rotation.resume.dialog.password": "Master-Passwort, das du jetzt hast",
  "rotation.resume.dialog.group": "Letzte Gruppe des neuen Secret Keys",
  "rotation.resume.dialog.body":
    "Die Schlüssel deines Kontos haben sich vielleicht schon geändert. Damit sicher ist, dass du es bist, gib dein Master-Passwort und die letzte Gruppe des neuen Secret Keys aus dem neuen Emergency Kit ein.",
  "rotation.resume.dialog.submit": "Fortsetzen",
  "rotation.resume.discard.title": "Diese Schlüsseländerung verwerfen?",
  "rotation.resume.discard.body":
    "Der neue Secret Key und das neue Emergency Kit dieses Versuchs werden gelöscht. Dein Konto bleibt, wie es ist.",
  "rotation.resume.discard.confirm": "Schlüsseländerung verwerfen",
  "rotation.resume.abandon": "Den Rest aufgeben …",
  "rotation.resume.stalled":
    "Das Fortsetzen hat nicht geklappt. Wenn es immer wieder scheitert, kannst du hier aufhören.",
  "rotation.abandon.title":
    "Neuverschlüsseln der Dateien beenden?",
  "rotation.abandon.body":
    "Deine neuen Schlüssel sind schon im Einsatz. Mosaic hört auf, die noch wartenden Anhänge neu zu verschlüsseln. Sie behalten ihren alten Dateischlüssel. Wer deine alten Schlüssel und eine Kopie einer Datei hat, könnte sie also noch öffnen.",
  "rotation.abandon.stays":
    "Alles, was schon neu verschlüsselt ist, bleibt, wie es ist. Entferne die wartenden Dateien später oder füge sie neu hinzu, um sie mit dem neuen Schlüssel zu schützen.",
  "rotation.abandon.confirm":
    "Neuverschlüsseln beenden",
  "rotation.abandon.keep": "Weiter versuchen",
  "rotation.marker.title": "Nicht bestätigte Meldung zur Neuverschlüsselung",
  "rotation.marker.hint": "Eine Meldung in deinem Sync-Ordner sagt, dein Konto wurde neu verschlüsselt, aber Mosaic konnte das nicht bestätigen.",
  "rotation.marker.open":
    "Diese Meldung ignorieren …",
  "rotation.marker.open.label":
    "Diese Meldung ignorieren: {name}",
  "rotation.marker.confirm":
    "Mosaic hat eine Meldung zur Neuverschlüsselung gefunden, die es nicht bestätigen konnte. Wenn du dein Konto nicht auf einem anderen Gerät neu verschlüsselt hast, kannst du sie ignorieren.",
  "rotation.marker.ignore": "Meldung ignorieren",
  "rotation.marker.keep":
    "Meldung behalten",
  "rotation.ignored.title": "Ignorierte Meldungen zur Neuverschlüsselung",
  "rotation.ignored.body":
    "Meldungen, die du ignoriert hast. Stelle eine wieder her, wenn du dein Konto doch auf einem anderen Gerät neu verschlüsselt hast.",
  "rotation.ignored.row": "Meldung zur Neuverschlüsselung aus deinem Sync-Ordner",
  "rotation.ignored.restore": "Wiederherstellen",
  "rotation.ignored.restore.label": "Die Meldung {name} wiederherstellen",
  "rotation.resume.done": "Schlüsseländerung abgeschlossen.",

  "rotation.recovery.title": "Die Schlüssel deines Kontos wurden auf einem anderen Gerät erneuert",
  "rotation.recovery.body":
    "Ein Gerät von dir hat die Schlüssel dieses Kontos erneuert. Damit die Synchronisierung weiterläuft, gib dein Master-Passwort und den Secret Key aus dem neuen Emergency Kit ein.",
  "rotation.recovery.password":
    "Master-Passwort (das neue, falls du es dort geändert hast)",
  "rotation.recovery.secretKey": "Secret Key aus dem neuen Emergency Kit",
  "rotation.recovery.submit": "Neue Schlüssel verwenden",
  "rotation.recovery.warning":
    "Wenn du deine Schlüssel nicht auf einem anderen Gerät erneuert hast, ist diese Meldung vielleicht nicht echt. Du kannst sie ignorieren.",
  "rotation.recovery.notice":
    "Die Synchronisierung ist pausiert, bis du das auf diesem Gerät erledigt hast.",
  "rotation.recovery.open": "Neue Schlüssel verwenden …",
  "rotation.recovery.keyHint":
    "Wurden deine Schlüssel auf einem anderen Gerät erneuert, gib auch den neuen Secret Key aus dessen Emergency Kit ein.",
  "rotation.recovery.done": "Dieses Gerät nutzt jetzt die neuen Schlüssel.",
  "rotation.recovery.optional":
    "Secret Key aus dem neuesten Emergency Kit (nur wenn du ihn erneuert hast)",

  "rotation.file.unknown":
    "In deinem Sync-Ordner ist eine unbekannte Kontodatei aufgetaucht. Sie lässt sich mit deinen Schlüsseln nicht prüfen: Sie kann übrig geblieben sein, oder jemand hat deinen Ordner verändert. Deine Daten sind nicht betroffen.",
  "rotation.file.dismiss": "Ausblenden",
  "rotation.file.conflict.title": "Zwei Geräte haben gleichzeitig die Schlüssel erneuert",
  "rotation.file.conflict":
    "Ein anderes Gerät hat die Schlüssel deines Kontos im selben Moment erneuert wie dieses, deshalb sind sich die beiden nicht mehr einig. Verschlüssele von diesem Gerät aus noch einmal neu, um sie wieder zusammenzubringen: Dann gelten die Schlüssel dieses Geräts, und das andere fragt nach dem neuen Secret Key.",
  "rotation.file.conflict.rekey": "Von diesem Gerät noch einmal neu verschlüsseln",
  "rotation.file.conflict.keep":
    "Hinweis ausblenden",
} satisfies Record<keyof typeof en, string>;
