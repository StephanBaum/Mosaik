/** German UI strings: browser extension (manifest, menus, popup). Informal "du". Same keys as English (checked by the type). */
import type { ext as en } from "../en/ext.ts";

export const ext = {
  // Manifest and menu strings (`_locales`, built at build time).
  "ext.manifest.name": "Mosaic",
  "ext.manifest.description":
    "Füllt Logins aus deinem Mosaic-Tresor aus. Dein Master-Passwort bleibt in der Desktop-App.",
  "ext.command.fill": "Login mit Mosaic ausfüllen",
  "ext.command.generate": "Passwort mit Mosaic erzeugen",
  "ext.save.badgeTitle": "Login speichern?",

  // Shell
  "ext.loading": "Lädt …",
  "ext.site.secure": "Aktuelle Website, sichere Verbindung (https): {host}",
  "ext.site.insecure": "Aktuelle Website, nicht verschlüsselt (http): {host}",
  "ext.site.none": "Dieser Tab zeigt keine Website",
  "ext.nav.label": "Ansichten",
  "ext.nav.logins": "Logins und Suche",
  "ext.nav.generator": "Passwort-Generator",
  "ext.footer.open": "Mosaic öffnen",
  "ext.footer.more": "Weitere Optionen",
  "ext.footer.connected": "Mit Mosaic verbunden",
  "ext.footer.locked": "Mosaic ist gesperrt",
  "ext.footer.offline": "Nicht verbunden",
  "ext.more.saveOffers": "Speichern von Logins anbieten",
  "ext.more.saveOffersHint":
    "Braucht Zugriff auf alle Websites, damit die Erweiterung ein abgeschicktes Login bemerkt. Gespeichert wird erst, wenn du es in der Mosaic-App bestätigst.",
  "ext.passkeys.toggle": "Mosaic für Passkeys verwenden",
  "ext.passkeys.hint":
    "Braucht Zugriff auf alle Websites, um Passkey-Anfragen zu beantworten. Jede bestätigst du in der Mosaic-App.",
  "ext.passkeys.permissionDenied":
    "Mosaic braucht Zugriff auf Websites, um Passkey-Anfragen zu beantworten, und du hast ihn nicht erlaubt.",
  "ext.more.disconnect": "Von Mosaic trennen",
  "ext.more.disconnectTitle": "Diesen Browser trennen?",
  "ext.more.disconnectBody":
    "Dieser Browser kann erst wieder Logins ausfüllen, wenn du ihn neu verbindest. Die neue Verbindung bestätigst du in der Mosaic-App.",
  "ext.action.cancel": "Abbrechen",

  // Connecting
  "ext.connect.checking": "Verbinde mit Mosaic …",
  "ext.connect.hostMissing.title": "Mosaic ist für diesen Browser nicht eingerichtet",
  "ext.connect.hostMissing.body":
    "Installiere die Mosaic-App auf diesem Computer und starte dann den Browser neu.",
  "ext.connect.hostMissing.dev": "Entwickler: Führe {command} aus und starte dann den Browser neu.",
  "ext.connect.hostForbidden.title": "Dieser Browser darf sich nicht verbinden",
  "ext.connect.hostForbidden.body":
    "Mosaic spricht nur mit der offiziellen Mosaic-Erweiterung. Installiere sie aus dem Store deines Browsers.",
  "ext.connect.desktopNotRunning.title": "Öffne Mosaic auf diesem Computer",
  "ext.connect.desktopNotRunning.body":
    "Die Erweiterung arbeitet über die Mosaic-App. Starte die App und prüfe dann erneut.",
  "ext.connect.check": "Erneut prüfen",
  "ext.connect.notPaired.title": "Mit Mosaic verbinden",
  "ext.connect.notPaired.body":
    "Verbinde diesen Browser mit der Mosaic-App auf diesem Computer. Du bestätigst das in der App.",
  "ext.connect.start": "Verbinden",
  "ext.connect.pairing.title": "In Mosaic bestätigen",
  "ext.connect.pairing.body":
    "Prüfe, ob Mosaic denselben Code zeigt, und klicke dort auf „Erlauben“.",
  "ext.connect.pairing.code": "Code",
  "ext.connect.pairing.waiting": "Warte auf deine Antwort in der Mosaic-App …",
  "ext.connect.protocolMismatch.title": "Mosaic aktualisieren",
  "ext.connect.protocolMismatch.body":
    "Erweiterung und Mosaic-App haben unterschiedliche Versionen. Aktualisiere beide auf die neueste Version.",
  "ext.connect.error.title": "Mosaic ist nicht erreichbar",
  "ext.connect.error.body": "Prüfe, ob die Mosaic-App läuft, und versuche es erneut.",

  // Locked
  "ext.locked.title": "Mosaic ist gesperrt",
  "ext.locked.body": "Entsperre es in der Mosaic-App, um auf {host} auszufüllen.",
  "ext.locked.bodyNoSite": "Entsperre es in der Mosaic-App, um es hier zu nutzen.",
  "ext.locked.open": "Mosaic öffnen",
  "ext.locked.note": "Dein Master-Passwort gibst du nur in der Mosaic-App ein.",

  // Logins
  "ext.logins.forSite": "Für {host}",
  "ext.logins.count": "{count, plural, one {# Login} other {# Logins}}",
  "ext.logins.sameSite": "gleiche Website",
  "ext.logins.none": "Kein gespeichertes Login für {host}.",
  "ext.logins.noneHint":
    "Mosaic vergleicht die vollständige Adresse, deshalb bekommen ähnlich aussehende Seiten nie einen Treffer.",
  "ext.logins.unsupported":
    "Mosaic kann auf dieser Seite nicht ausfüllen. Durchsuche deinen Tresor, um ein Login zu kopieren.",
  "ext.logins.shortcut": "{shortcut} füllt den ersten Treffer aus.",
  "ext.logins.frameNote":
    "Mosaic füllt nur auf {host} aus, nie in eingebetteten Frames anderer Websites.",
  "ext.item.untitled": "Ohne Titel",
  "ext.item.noUsername": "Kein Benutzername",
  "ext.item.twoFactor": "2FA",
  "ext.item.fill": "Ausfüllen",
  "ext.item.fillLabel": "{name} ausfüllen",
  "ext.item.copyMenu": "Kopier-Optionen für {name}",
  "ext.item.copyUsername": "Benutzernamen kopieren",
  "ext.item.copyPassword": "Passwort kopieren",
  "ext.item.copyCode": "Code kopieren",
  "ext.item.blocked":
    "Gesperrt: Diese Seite ist nicht verschlüsselt und dieses Login erlaubt das nicht.",
  "ext.search.label": "Andere Einträge",
  "ext.search.placeholder": "Tresor durchsuchen",
  "ext.search.note":
    "Ergebnisse bleiben in diesem Popup. Die Seite bekommt immer nur das eine Login, das du ausfüllst.",
  "ext.search.none": "Nichts gefunden.",
  "ext.search.copy": "Kopieren",
  "ext.search.copyOnly": "Nur kopieren: nicht für diese Website gespeichert",
  "ext.shortcut.copyUser": "{platform, select, mac {⌘C} other {Strg+C}}",
  "ext.shortcut.copyPassword": "{platform, select, mac {⇧⌘C} other {Strg+Umschalt+C}}",
  "ext.shortcut.copyCode": "{platform, select, mac {⌥⌘C} other {Strg+Alt+C}}",
  "ext.search.results": "{count, plural, one {# Ergebnis} other {# Ergebnisse}}",

  // Frames
  "ext.frame.cross":
    "Das Feld im Fokus liegt in einem Frame einer anderen Website. Mosaic füllt dort nicht aus.",
  "ext.frameConfirm.title": "In einem eingebetteten Frame ausfüllen?",
  "ext.frameConfirm.body":
    "Das Feld liegt in einem Frame auf {host}, nicht in der Seite selbst. Fülle nur aus, wenn du diesem Frame vertraust.",
  "ext.frameConfirm.confirm": "Im Frame ausfüllen",

  // Address warnings
  "ext.warning.idn.title": "Prüfe die Adresse",
  "ext.warning.idn.body":
    "Diese Adresse enthält Sonderzeichen und kann wie eine andere Website aussehen. Prüfe die Adressleiste, bevor du dich anmeldest.",
  "ext.warning.lookalike.body":
    "Diese Seite ist nicht {saved}. Sie sieht ähnlich aus, ein häufiger Phishing-Trick. Schließe den Tab oder gib {saved} selbst ein.",
  "ext.warning.youAreOn": "Du bist auf",
  "ext.warning.writtenAs": "Geschrieben als",
  "ext.warning.savedFor": "Gespeichert für",
  "ext.warning.insecure.title": "Diese Seite ist nicht verschlüsselt",
  "ext.warning.insecure.body":
    "Die Seite nutzt http. Andere im Netzwerk könnten dein Passwort mitlesen. Mosaic füllt hier nur Logins aus, bei denen du es erlaubt hast.",

  // One-time codes
  "ext.otp.title": "Einmalcode ausfüllen",
  "ext.otp.next": "Passwort für {name} ausgefüllt. Als Nächstes: der Zwei-Faktor-Code.",
  "ext.otp.for": "Code für {name}",
  "ext.otp.fill": "Code ausfüllen",
  "ext.otp.copy": "Code kopieren",
  "ext.otp.note":
    "Der Code geht von Mosaic direkt ins Feld. Dieses Popup zeigt ihn nie an.",

  // Saving
  "ext.save.titleNew": "Dieses Login für {host} speichern?",
  "ext.save.titleUpdate": "Gespeichertes Login für {host} aktualisieren?",
  "ext.save.generated": "Ein Passwort, das Mosaic für {host} erzeugt hat.",
  "ext.save.updates": "Aktualisiert „{title}“.",
  "ext.save.username": "Benutzername",
  "ext.save.noUsername": "Keiner",
  "ext.save.password": "Passwort",
  "ext.save.passwordHidden": "Verborgen. Du siehst es in der Mosaic-App.",
  "ext.save.insecure": "Diese Seite ist nicht verschlüsselt (http).",
  "ext.save.action": "In Mosaic speichern …",
  "ext.save.notNow": "Jetzt nicht",
  "ext.save.never": "Nie für diese Website",
  "ext.save.waiting": "Bestätige in der Mosaic-App. Bis dahin wird nichts gespeichert.",
  "ext.save.saved": "In Mosaic gespeichert.",
  "ext.save.updated": "In Mosaic aktualisiert.",
  "ext.save.cancelled": "Nicht gespeichert.",
  "ext.save.dismissed": "Verworfen. Das Passwort ist vergessen.",
  "ext.save.neverDone": "Mosaic bietet auf {host} nicht mehr an zu speichern.",
  "ext.save.permissionAsk":
    "Mosaic bittet deinen Browser um Zugriff auf Websites. Erlaube ihn, um das einzuschalten.",

  // Generator
  "ext.generator.title": "Passwort für {host} erzeugen",
  "ext.generator.titleGeneric": "Passwort erzeugen",
  "ext.generator.value": "Erzeugtes Passwort",
  "ext.generator.regenerate": "Weiteres Passwort erzeugen",
  "ext.generator.type": "Passwort-Art",
  "ext.generator.random": "Zufällig",
  "ext.generator.passphrase": "Passphrase",
  "ext.generator.include": "Enthalten",
  "ext.generator.rules.page": "Folgt den Passwort-Regeln dieser Seite",
  "ext.generator.rules.siteQuirks": "Folgt den bekannten Regeln dieser Website",
  "ext.generator.fill": "Ausfüllen",
  "ext.generator.fillHint":
    "Zum Ausfüllen klickst du auf der Seite in ein Feld für ein neues Passwort und öffnest dieses Popup erneut.",
  "ext.generator.copy": "Kopieren",
  "ext.generator.expired":
    "Dieses Passwort wurde nicht mehr gehalten, deshalb wurde ein neues erzeugt.",
  "ext.generator.filled": "Ausgefüllt. Mosaic bietet an, es zu speichern.",

  // Announcements
  "ext.copied":
    "Kopiert. Wird in {seconds, plural, one {# Sekunde} other {# Sekunden}} gelöscht.",
  "ext.copiedNoClear": "Kopiert.",
  "ext.filled": "Ausgefüllt.",
  "ext.filledCode": "Code ausgefüllt.",

  // Errors: every code the extension can show (`ErrorCode`), what happened and what to do.
  "ext.error.invalidRequest":
    "Mosaic hat die Anfrage nicht verstanden. Aktualisiere die Erweiterung und die Mosaic-App.",
  "ext.error.notPaired": "Dieser Browser ist nicht mit Mosaic verbunden. Verbinde ihn neu.",
  "ext.error.protocolVersion":
    "Erweiterung und Mosaic-App haben unterschiedliche Versionen. Aktualisiere beide.",
  "ext.error.busy": "Mosaic bearbeitet gerade eine andere Anfrage. Versuche es gleich noch einmal.",
  "ext.error.locked": "Mosaic ist gesperrt. Entsperre es in der Mosaic-App.",
  "ext.error.rateLimited":
    "Das waren zu viele Anfragen in kurzer Zeit. Warte einen Moment und versuche es dann erneut.",
  "ext.error.originMismatch":
    "Dieses Login ist nicht für diese Website gespeichert, deshalb füllt Mosaic es hier nicht aus.",
  "ext.error.insecureOrigin":
    "Diese Seite ist nicht verschlüsselt und das Login erlaubt dort kein Ausfüllen. Es wurde nichts ausgefüllt.",
  "ext.error.crossOriginFrame":
    "Das Feld liegt in einem Frame einer anderen Website. Mosaic füllt dort nicht aus.",
  "ext.error.itemNotFound":
    "Dieser Eintrag existiert nicht mehr. Er wurde vielleicht verschoben oder gelöscht.",
  "ext.error.vaultUnavailable": "Dieser Tresor ist geschlossen. Öffne ihn zuerst in der Mosaic-App.",
  "ext.error.noOtp": "Dieser Eintrag hat keinen Einmalcode.",
  "ext.error.otpNotFillable":
    "Dieser Code zählt bei jeder Nutzung hoch (HOTP), deshalb kann er von hier nicht ausgefüllt oder kopiert werden. Nutze ihn in der Mosaic-App.",
  "ext.error.fieldEmpty": "Dieses Feld ist leer, es gibt also nichts auszufüllen oder zu kopieren.",
  "ext.error.clipboardUnavailable":
    "Mosaic konnte die Zwischenablage nicht nutzen. Versuche es erneut oder kopiere in der Mosaic-App.",
  "ext.error.cancelled": "In der Mosaic-App abgebrochen. Versuche es erneut, wenn du so weit bist.",
  "ext.error.passkeyNotFound": "Mosaic hat keinen Passkey für diese Website.",
  "ext.error.passkeyExcluded": "Du hast für dieses Konto bereits einen Passkey in Mosaic.",
  "ext.error.rpIdMismatch":
    "Diese Website darf keinen Passkey für diese Domain nutzen, deshalb hat Mosaic nicht geantwortet.",
  "ext.error.passkeyUnsupported":
    "Mosaic kann mit den Optionen dieser Website keinen Passkey erstellen.",
  "ext.error.pairingDenied":
    "Die Verbindung wurde in der Mosaic-App abgelehnt. Wähle „Verbinden“, um es erneut zu versuchen, und dann „Erlauben“.",
  "ext.error.pairingExpired":
    "Die Verbindungsanfrage ist abgelaufen. Wähle „Verbinden“, um neu zu beginnen.",
  "ext.error.pairingConflict":
    "Eine andere Verbindungsanfrage war offen. Wähle „Verbinden“, um es erneut zu versuchen.",
  "ext.error.pairingBlocked":
    "Zu viele Verbindungen wurden abgelehnt. Warte zehn Minuten und versuche es dann erneut.",
  "ext.error.pairingLimit":
    "Mit diesem Computer sind zu viele Browser verbunden. Trenne einen in den Mosaic-Einstellungen.",
  "ext.error.internal":
    "In Mosaic ist ein unerwartetes Problem aufgetreten. Versuche es erneut. Wenn es bleibt, starte die Mosaic-App neu.",
  "ext.error.desktopNotRunning":
    "Mosaic läuft auf diesem Computer nicht. Öffne es und versuche es dann erneut.",
  "ext.error.desktopNotAuthentic":
    "Die App, die geantwortet hat, ist nicht die Mosaic-App, mit der dieser Browser verbunden ist. Es wurde nichts gesendet.",
  "ext.error.timeout":
    "Mosaic hat nicht rechtzeitig geantwortet. Ist in der Mosaic-App ein Dialog offen, beantworte ihn dort.",
  "ext.error.frameTooLarge": "Das waren zu viele Daten, um sie an Mosaic zu senden.",
  "ext.error.hostMissing":
    "Der Mosaic-Helfer ist für diesen Browser nicht installiert. Installiere die Mosaic-App und starte den Browser neu.",
  "ext.error.hostForbidden":
    "Dieser Browser oder diese Erweiterung darf nicht mit Mosaic sprechen.",
  "ext.error.hostExited": "Die Verbindung zu Mosaic wurde geschlossen. Versuche es erneut.",
  "ext.error.unsupportedPage": "Mosaic kann auf dieser Art von Seite nicht ausfüllen.",
  "ext.error.noFields":
    "Hier wurden keine Login-Felder gefunden. Klicke in ein Feld und versuche es erneut.",
  "ext.error.noMatch": "Mosaic hat kein gespeichertes Login für diese Website.",
  "ext.error.frameNeedsConfirmation":
    "Das Feld liegt in einem Frame. Öffne das Mosaic-Popup und bestätige den Frame, um es auszufüllen.",
  "ext.error.frameOpaque":
    "Das Feld liegt in einem Frame mit Sandbox. Mosaic füllt dort nicht aus.",
  "ext.error.originChanged":
    "Die Seite hat sich während des Ausfüllens geändert. Es wurde nichts ausgefüllt. Versuche es erneut.",
  "ext.error.pageHidden": "Die Seite ist nicht sichtbar, deshalb hat Mosaic nicht ausgefüllt.",
  "ext.error.permissionDenied":
    "Mosaic braucht Zugriff auf Websites, um Logins zu bemerken, und du hast ihn nicht erlaubt.",
  "ext.error.generatedExpired":
    "Mosaic hält dieses Passwort nicht mehr. Erzeuge ein neues.",

  // Passwortänderung auf der Website (M7b)
  "ext.rotation.card.title": "Passwortänderung",
  "ext.rotation.card.for": "Für {name}",
  "ext.rotation.card.started":
    "Gestartet am {when}",
  "ext.rotation.card.delivered": "Das neue Passwort wurde schon einmal eingetragen.",
  "ext.rotation.card.note":
    "Das neue Passwort siehst du nur in der Mosaic-App. Mosaic sendet das Formular nie für dich ab.",
  "ext.rotation.card.list": "Laufende Passwortänderungen",
  "ext.rotation.change": "Passwort ändern…",
  "ext.rotation.changeLabel": "Passwort ändern… für {name}",
  "ext.rotation.openPage": "Änderungsseite öffnen",
  "ext.rotation.fillChange": "Änderungsformular ausfüllen",
  "ext.rotation.fillPending": "Neues Passwort zum Anmelden eintragen",
  "ext.rotation.worked": "Hat geklappt",
  "ext.rotation.failed": "Hat nicht geklappt",
  "ext.rotation.fresh": "Mit neuem Passwort neu starten",
  "ext.rotation.waiting": "Mosaic wartet in der App auf dich…",
  "ext.rotation.started": "Gestartet. Wähle in der Mosaic-App „Dieses Passwort verwenden“.",
  "ext.rotation.startCancelled": "Es wurde nichts gestartet.",
  "ext.rotation.filled": "Änderungsformular ausgefüllt. Prüfe es und sende es selbst ab.",
  "ext.rotation.opened": "Änderungsseite geöffnet.",
  "ext.rotation.confirmed": "Fertig. Das neue Passwort ist jetzt in Mosaic gespeichert.",
  "ext.rotation.keptOld": "Mosaic hat das alte Passwort behalten.",
  "ext.rotation.resultCancelled": "Nichts geändert. Die Änderung ist weiterhin offen.",
  "ext.rotation.offer.title": "Eine verwandte Adresse",
  "ext.rotation.offer.body":
    "Diese Seite liegt auf {host}, das nicht für {title} gespeichert ist. Soll Mosaic die Adresse hinzufügen, damit es dort ausfüllen kann?",
  "ext.rotation.offer.add": "{host} hinzufügen",
  "ext.rotation.offer.added": "Hinzugefügt. Die Änderung wird jetzt hier angezeigt.",
  "ext.rotation.offer.cancelled": "Nicht hinzugefügt.",

  "ext.error.rotationNotFound":
    "Diese Passwortänderung ist nicht mehr offen. Starte beim Login eine neue.",
  "ext.error.originNotRelated":
    "Diese Adresse liegt nicht auf derselben Website wie dieser Login, deshalb hat Mosaic sie nicht hinzugefügt.",
  "ext.error.rotationLimit":
    "Dieser Login hat schon vier unfertige Änderungen. Schließe in der Mosaic-App zuerst eine ab oder brich sie ab.",
  "ext.error.changeFormNotFound":
    "Mosaic hat hier kein Formular zum Ändern des Passworts gefunden. Öffne die Passwortseite deines Kontos und versuche es erneut.",
  "ext.error.changeFormAmbiguous":
    "Mosaic konnte nicht erkennen, welches Feld welches ist. Kopiere das neue Passwort aus Mosaic und füge es selbst ein.",
} satisfies Record<keyof typeof en, string>;
