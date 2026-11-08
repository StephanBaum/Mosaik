/** German UI strings: sharing, safety codes, members and emergency access (M6b). Informal "du". Same keys as English (checked by the type). */
import type { sharing as en } from "../en/sharing.ts";

export const sharing = {
  "sharing.title": "Teilen",
  "sharing.body":
    "Teile Tresore mit Menschen, die du kennst. Alles bleibt verschlüsselt: Nur die Personen, die du einlädst, können einen geteilten Tresor lesen, und Mosaic teilt nur mit Personen, deren Sicherheitscode du verglichen hast.",
  "sharing.notHosted.none":
    "Zum Teilen brauchst du einen Mosaic-Server. Schalte zuerst in den Sync-Einstellungen die Synchronisierung mit einem Mosaic-Server ein.",
  "sharing.notHosted.folder":
    "Mit einem Sync-Ordner kannst du nicht teilen. Ein Ordner kann nicht festlegen, wer einen Tresor lesen darf, deshalb bleiben Tresore in einem Ordner nur für dich. Zum Teilen trennst du den Ordner und verbindest in den Sync-Einstellungen einen Mosaic-Server.",
  "sharing.notHosted":
    "Zum Teilen brauchst du einen Mosaic-Server. Verbinde zuerst in den Sync-Einstellungen einen.",

  "sharing.ownCode.title": "Dein Sicherheitscode",
  "sharing.ownCode.body":
    "Lies diesen Code der Person vor, mit der du teilst, persönlich oder am Telefon. Stimmt er mit dem überein, was sie für dich sieht, steht niemand zwischen euch.",
  "sharing.ownCode.pending":
    "Dein Sicherheitscode erscheint nach dem ersten Sync mit dem Server.",

  "sharing.contacts.title": "Personen, die du nachgeschlagen hast",
  "sharing.contacts.empty": "Du hast noch niemanden nachgeschlagen.",
  "sharing.contacts.verified": "Geprüft",
  "sharing.contacts.notVerified": "Nicht geprüft",
  "sharing.contacts.changed": "Muss neu geprüft werden",
  "sharing.contacts.verify": "Code vergleichen…",
  "sharing.contacts.review": "Änderung ansehen…",

  "sharing.share.open": "Teilen…",
  "sharing.share.title": "{vault} teilen",
  "sharing.share.dialog": "Teilen: {vault}",
  "sharing.lookup.email": "E-Mail-Adresse der Person",
  "sharing.lookup.find": "Nachschlagen",
  "sharing.lookup.searching": "Schlage nach…",
  "sharing.lookup.notFoundHint":
    "Mosaic kann dir nicht sagen, ob es zu einer Adresse ein Konto gibt. Stimmt der Code unten nicht mit dem überein, was die Person sieht, teile nichts.",

  "sharing.code.title": "Sicherheitscodes vergleichen",
  "sharing.code.compare":
    "Vergleiche diesen Code mit {email}, persönlich oder am Telefon. Nutze dafür weder E-Mail noch Chat.",
  "sharing.code.shown": "Code, den Mosaic für {email} zeigt",
  "sharing.code.typed": "Code, den die Person dir vorliest (optional)",
  "sharing.code.typedMatch": "Das ist der Code oben.",
  "sharing.code.matches": "Er stimmt überein",
  "sharing.code.mismatch":
    "Die Codes sind verschieden. Teile nichts. Jemand könnte sich zwischen dich und {email} geschaltet haben.",
  "sharing.code.mismatchHelp":
    "Prüfe die Adresse und bitte {email}, den Code noch einmal vorzulesen. Ist er immer noch anders, brich ab und sag es der Person auf anderem Weg.",
  "sharing.code.verified": "{email} ist geprüft.",
  "sharing.code.status.new": "Neuer Kontakt",
  "sharing.code.status.pinned": "Noch nicht geprüft",
  "sharing.code.status.verified": "Geprüft",
  "sharing.code.status.changed": "Schlüssel geändert",
  "sharing.code.status.conflicting": "Geräte sind uneins",
  "sharing.code.conflicting":
    "Deine Geräte haben für {email} verschiedene Schlüssel gesehen. Vergleiche den Code noch einmal mit der Person, bevor du teilst.",

  "sharing.role.label": "Rolle",
  "sharing.role.owner": "Eigentümer",
  "sharing.role.manage": "Verwalter",
  "sharing.role.write": "Bearbeiter",
  "sharing.role.read": "Leser",
  "sharing.role.desc.manage": "Kann Personen einladen und entfernen und Rollen ändern.",
  "sharing.role.desc.write": "Kann Einträge hinzufügen und ändern.",
  "sharing.role.desc.read": "Kann nur lesen.",
  "sharing.role.honesty":
    "Wer einen Eintrag lesen kann, kann ihn auch kopieren. Ein Leser kann den Tresor nicht ändern, aber nichts hindert ihn daran, das Gesehene zu behalten, und wenn du jemanden später entfernst, nimmt das nicht zurück, was schon heruntergeladen wurde.",

  "sharing.invite.send": "Einladung senden",
  "sharing.invite.sent":
    "Einladung an {email} gesendet. Die Person muss Mosaic öffnen und sie annehmen.",
  "sharing.invite.pending": "Wartet auf {email}",
  "sharing.invite.expires": "Läuft {when} ab",
  "sharing.invite.cancel": "Einladung zurückziehen",
  "sharing.invite.cancelled": "Einladung zurückgezogen.",

  "sharing.inbox.title": "Einladungen",
  "sharing.inbox.nav": "Einladungen",
  "sharing.inbox.navCount": "Einladungen, {count} offen",
  "sharing.inbox.intro":
    "Personen, die dich zu einem geteilten Tresor eingeladen haben. Mosaic prüft jede Einladung, bevor es sie dir zeigt.",
  "sharing.inbox.empty":
    "Gerade gibt es keine Einladungen. Lädt dich jemand zu einem geteilten Tresor ein, erscheint die Einladung hier.",
  "sharing.inbox.from": "Von {email}",
  "sharing.inbox.review": "Ansehen",
  "sharing.inbox.refresh": "Aktualisieren",
  "sharing.inbox.preview.title": "{vault} beitreten?",
  "sharing.inbox.preview.body": "{email} lädt dich als {role} ein.",
  "sharing.inbox.preview.unnamed": "einem geteilten Tresor",
  "sharing.inbox.preview.owner": "Der Tresor gehört {email}.",
  "sharing.inbox.verifySender":
    "Vergleiche vor dem Beitritt den Sicherheitscode von {email}.",
  "sharing.inbox.verifyOwner":
    "{sender} ist nicht der Eigentümer. Vergleiche vor dem Beitritt auch den Sicherheitscode des Eigentümers: Er ist der, der den Schlüssel des Tresors ändern darf.",
  "sharing.inbox.ownerCode": "Code des Eigentümers, {email}",
  "sharing.inbox.accept": "Tresor beitreten",
  "sharing.inbox.decline": "Ablehnen",
  "sharing.inbox.block": "Ablehnen und Absender blockieren",
  "sharing.inbox.unreadable":
    "Diese Einladung lässt sich nicht öffnen. Sie ist vielleicht beschädigt oder nicht für dich bestimmt. Bitte {email}, sie noch einmal zu senden.",
  "sharing.inbox.joined":
    "Du bist dem Tresor beigetreten. Er erscheint gleich in deiner Tresorliste.",
  "sharing.inbox.declined": "Einladung abgelehnt.",
  "sharing.inbox.outgoing": "Einladungen, die du gesendet hast",
  "sharing.inbox.outgoing.empty": "Keine deiner Einladungen wartet auf eine Antwort.",
  "sharing.inbox.unknownVault": "ein Tresor",

  "sharing.members.title": "Personen mit Zugriff",
  "sharing.members.you": "Du",
  "sharing.members.unverified": "Nicht geprüft",
  "sharing.members.unconfirmed": "Nicht auf der Mitgliederliste",
  "sharing.members.unconfirmed.help":
    "Der Server führt diese Person, die Mitgliederliste des Tresors aber nicht. Beim nächsten Schlüsselwechsel wird sie ausgesperrt. Kennst du sie nicht, entferne sie.",
  "sharing.members.dropped": "Nicht mehr auf dem Server",
  "sharing.members.dropped.help":
    "Diese Person steht auf der Mitgliederliste des Tresors, aber der Server führt sie nicht mehr. Vielleicht hat sie den Tresor verlassen. Prüfe, ob das so gewollt ist.",
  "sharing.members.invited": "Eingeladen",
  "sharing.members.removing": "Wird entfernt",
  "sharing.members.verified": "Geprüft",
  "sharing.members.verify": "Prüfen…",
  "sharing.members.remove": "Entfernen",
  "sharing.members.remove.title": "{email} entfernen?",
  "sharing.members.remove.confirm":
    "{email} behält, was schon heruntergeladen wurde. Mosaic wechselt jetzt den Tresorschlüssel, damit nichts Neues mehr lesbar ist.",
  "sharing.members.remove.confirmManager":
    "{email} behält, was schon heruntergeladen wurde. Der Server schickt ab sofort keine Updates mehr. Das Gerät des Tresor-Eigentümers wechselt den Tresorschlüssel beim nächsten Sync, und bis dahin hält nur der Server {email} draußen.",
  "sharing.members.removed": "Entfernt.",
  "sharing.members.leave": "Tresor verlassen",
  "sharing.members.leave.title": "Diesen Tresor verlassen?",
  "sharing.members.leave.confirm":
    "Die Kopie auf diesem Gerät bleibt lesbar, wird aber nicht mehr aktualisiert. Um wieder beizutreten, brauchst du eine neue Einladung.",
  "sharing.members.left": "Du hast den Tresor verlassen.",
  "sharing.members.roleOf": "Rolle von {email}",
  "sharing.members.invite": "Jemanden einladen…",
  "sharing.members.empty": "Bisher kannst nur du diesen Tresor öffnen.",
  "sharing.members.unknownEmail": "Unbekannte Adresse",
  "sharing.members.managerNote":
    "Du kannst Personen einladen und entfernen. Nur das Gerät des Eigentümers wechselt den Tresorschlüssel, eine Entfernung ist also erst abgeschlossen, wenn dieses Gerät synchronisiert hat.",
  "sharing.members.readerNote":
    "Du kannst diesen Tresor lesen, aber nicht ändern, wer Zugriff hat.",
  "sharing.rotation.running":
    "Eine Entfernung wartet darauf, dass das Gerät des Eigentümers den Tresorschlüssel wechselt. Bis es synchronisiert hat, hält nur der Server die entfernte Person draußen.",
  "sharing.rotation.unprotected":
    "{count, plural, one {# Datei behält} other {# Dateien behalten}} den alten Schutz, weil dieses Gerät sie nicht hat. Eine entfernte Person kann sie noch öffnen, wenn sie den alten Schlüssel behalten hat. Füge sie auf einem Gerät, das sie hat, erneut hinzu.",

  "sharing.keyChanged.title": "Der Sicherheitscode von {email} hat sich geändert",
  "sharing.keyChanged.body":
    "Der Server zeigt jetzt einen anderen Schlüssel für {email}. Das passiert, wenn die Person Mosaic neu einrichtet, und genau so würde auch ein Angriff auf den Server aussehen. Mit dem neuen Schlüssel wird nichts geteilt, bevor du die Codes persönlich oder am Telefon verglichen hast.",
  "sharing.keyChanged.old": "Code, den du vorher verglichen hast",
  "sharing.keyChanged.new": "Neuer Code",
  "sharing.keyChanged.accept": "Vergleichen und übernehmen",

  "sharing.finding.region": "Warnungen zum Teilen",
  "sharing.finding.unknownEmail": "eine unbekannte Person",
  "sharing.finding.verify": "Prüfen…",
  "sharing.finding.remove": "Entfernen",
  "sharing.finding.identityMismatch":
    "Der Server zeigt für dein Konto einen anderen Schlüssel zum Teilen als den, den dieses Gerät erzeugt hat, deshalb ist das Teilen pausiert. Teile nichts und prüfe deinen Server und deine anderen Geräte. Wähle dann „Ich habe es geprüft…“.",
  "sharing.finding.rosterInvalid":
    "Die Mitgliederliste von {vault} ist beschädigt, oder jemand ohne Berechtigung hat sie geändert. Füge niemanden hinzu, bevor du geprüft hast, wer Zugriff hat. Wähle dann „Ich habe es geprüft…“.",
  "sharing.finding.shareUntrusted":
    "Ein neuer Schlüssel für {vault} wurde abgelehnt: {sender} hat ihn geschickt, darf den Schlüssel dieses Tresors aber nicht ändern. Der Tresor bleibt beim alten Schlüssel. Frag den Eigentümer, ob er ihn geändert hat. Wähle dann „Ich habe es geprüft…“.",
  "sharing.finding.memberUnverified":
    "Du hast {email} in {vault} nicht geprüft. Mosaic schickt der Person keine neuen Schlüssel, und beim nächsten Schlüsselwechsel wird sie ausgesperrt. Vergleiche ihren Sicherheitscode oder entferne sie.",
  "sharing.finding.memberDropped":
    "{email} steht auf der Mitgliederliste von {vault}, aber der Server führt die Person nicht mehr. Vielleicht hat sie den Tresor verlassen, oder der Server hat sie entfernt. Prüfe, ob das so gewollt ist, und entferne sie, wenn nicht.",
  "sharing.finding.accessLost":
    "Du hast keinen Zugriff mehr auf {vault}. Die Kopie auf diesem Gerät bleibt lesbar, wird aber nicht mehr aktualisiert. Bitte den Eigentümer, dich erneut einzuladen.",
  "sharing.finding.rotationBlocked":
    "Der Schlüssel von {vault} konnte nach dem Entfernen einer Person nicht gewechselt werden, sie hat also noch den alten Schlüssel. Synchronisiere erneut. Bleibt es dabei, öffne die Mitgliederliste und prüfe, wer Zugriff hat.",

  "emergency.title": "Notfallzugriff",
  "emergency.body":
    "Lege Personen fest, die Zugriff auf deine Tresore anfragen können, falls dir etwas zustößt. Das ist keine Kontowiederherstellung: Sie bekommen nur Lesezugriff auf die Tresore, die du auswählst, erst nach einer Wartezeit, und du kannst in dieser Zeit ablehnen. Dein Emergency Kit ersetzt das nicht.",
  "emergency.folderUnsupported":
    "Notfallzugriff braucht den Mosaic-Server. Ein Sync-Ordner kann keine Wartezeit durchsetzen.",
  "emergency.granted.title": "Personen, die Zugriff anfragen können",
  "emergency.granted.empty": "Niemand kann Zugriff auf deine Tresore anfragen.",
  "emergency.granted.add": "Person hinzufügen…",
  "emergency.granted.scopeAll": "Alle deine Tresore",
  "emergency.granted.scopeSome": "{count, plural, one {# Tresor} other {# Tresore}}",
  "emergency.granted.wait": "Wartet {days, plural, one {# Tag} other {# Tage}}",
  "emergency.granted.waitingUntil":
    "Hat Zugriff angefragt. Er öffnet sich {when}, falls du nicht ablehnst.",
  "emergency.granted.approved": "Du hast die Anfrage genehmigt.",
  "emergency.granted.released": "Der Zugriff wurde freigegeben. Die Person kann die Tresore lesen.",
  "emergency.granted.idle": "Keine Anfrage",
  "emergency.granted.remove": "Zugriff entziehen",
  "emergency.granted.remove.title": "Notfallzugriff für {email} entziehen?",
  "emergency.granted.remove.body":
    "Die Person kann dann nichts mehr anfragen. Wurde ihr Zugriff schon freigegeben, wechselt Mosaic die Schlüssel der Tresore, die sie geöffnet hat; was sie schon heruntergeladen hat, behält sie.",
  "emergency.granted.removed": "Notfallzugriff entzogen.",
  "emergency.grant.title": "Notfallzugriff erlauben",
  "emergency.grant.lookup": "Wer soll anfragen können?",
  "emergency.grant.scope": "Was die Person öffnen kann",
  "emergency.grant.scope.all": "Alle meine Tresore, auch neue",
  "emergency.grant.scope.selected": "Nur die Tresore, die ich auswähle",
  "emergency.grant.scope.pick": "Wähle mindestens einen Tresor aus.",
  "emergency.grant.wait": "Wartezeit",
  "emergency.grant.wait.option": "{days, plural, one {# Tag} other {# Tage}}",
  "emergency.grant.waitHint":
    "Öffne Mosaic nach einer Anfrage innerhalb dieser Zeit mindestens einmal, damit du sie siehst und ablehnen kannst. Ist die Zeit um, bekommt die Person Lesezugriff, ohne dass du noch einmal gefragt wirst.",
  "emergency.grant.serverTrust":
    "Die Wartezeit wird vom Server eingehalten. Wer den Server kontrolliert, könnte sie überspringen.",
  "emergency.grant.vaultKeysOnly":
    "Das gilt nur für die Schlüssel deiner Tresore. Es gibt keinen Zugriff auf dein Konto, dein Master-Passwort oder deinen Secret Key.",
  "emergency.grant.submit": "Notfallzugriff erlauben",
  "emergency.grant.done": "{email} kann jetzt Notfallzugriff anfragen.",
  "emergency.grant.needVerify": "Vergleiche zuerst den Sicherheitscode von {email}.",
  "emergency.received.title": "Personen, die dich benannt haben",
  "emergency.received.empty": "Niemand hat dich für Notfallzugriff benannt.",
  "emergency.received.request": "Zugriff anfragen",
  "emergency.received.requested":
    "Anfrage gesendet. {email} sieht sie, sobald Mosaic das nächste Mal geöffnet wird.",
  "emergency.received.requestHint":
    "{email} sieht deine Anfrage in Mosaic. Nach {days, plural, one {# Tag} other {# Tagen}} kannst du die Tresore lesen, falls die Person nicht ablehnt.",
  "emergency.received.countdown": "Du kannst ihre Tresore {when} öffnen.",
  "emergency.received.claim": "Ihre Tresore öffnen",
  "emergency.received.claimed":
    "{count, plural, one {# Tresor wurde} other {# Tresore wurden}} hinzugefügt, nur lesbar.",
  "emergency.received.cooldown": "{email} hat abgelehnt. Du kannst {when} erneut anfragen.",
  "emergency.received.verifyFirst":
    "Vergleiche den Sicherheitscode von {email}, bevor du ihre Tresore öffnest. Mosaic nutzt nur einen Schlüssel, den du geprüft hast.",
  "emergency.received.verify": "Code vergleichen…",
  "emergency.received.approved": "{email} hat deine Anfrage genehmigt.",
  "emergency.received.waitDays": "Wartet {days, plural, one {# Tag} other {# Tage}}",
  "emergency.banner.region": "Anfrage für Notfallzugriff",
  "emergency.banner.body":
    "{email} hat Notfallzugriff auf deine Tresore angefragt. Er öffnet sich {when}, falls du nicht ablehnst.",
  "emergency.banner.refuse": "Ablehnen",
  "emergency.banner.approve": "Jetzt genehmigen…",
  "emergency.banner.approve.title": "Notfallzugriff jetzt genehmigen?",
  "emergency.banner.approve.body":
    "{email} bekommt sofort Lesezugriff auf deine Tresore. Gib zur Bestätigung dein Master-Passwort ein.",
  "emergency.banner.approve.confirm": "Genehmigen",
  "emergency.banner.refused": "Anfrage abgelehnt.",
  "emergency.banner.approved": "Zugriff genehmigt.",
} satisfies Record<keyof typeof en, string>;
