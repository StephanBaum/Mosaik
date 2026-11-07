# Landingpage-Texte (Deutsch)

> **Arbeitstitel.** {NAME} steht für den Produktnamen (Arbeitstitel „Mosaic"), {DOMAIN} für die Domain. Beides steht noch nicht fest. Die Lizenz ist offen, deshalb nennt der Text keine und vermeidet „Open Source". Mit **(kommt)** markierte Punkte sind geplant und behalten die Markierung, bis sie ausgeliefert sind. Stand: 07.11.2026. Preise sind Platzhalter. Anrede: du, wie in der App.

---

## 1. Hero

**Überschriften zur Auswahl**
1. Dein Tresor. Deine Cloud. Deine Schlüssel.
2. Der Passwortmanager, der offenlegt, wie er arbeitet.
3. Zwei Geheimnisse sperren deine Passwörter. Beide hast nur du.

**Unterzeile**
{NAME} ist ein Passwortmanager nach dem Zero-Knowledge-Prinzip, mit veröffentlichter Kryptospezifikation und veröffentlichtem Bedrohungsmodell. Synchronisiere über den Cloud-Ordner, den du schon nutzt, oder lass uns das hosten. Keine Telemetrie, kein Tracking.

**Hauptbutton:** {NAME} herunterladen (Desktop, Entwicklungsversion)
**Zweitbutton:** So funktioniert es

**Vertrauenszeile (unter den Buttons)**
Noch ohne Sicherheitsaudit. Kryptospezifikation öffentlich. Bedrohungsmodell öffentlich. Keine Analyse-Tools.

> Hinweis fürs Design: Die Zeile „Noch ohne Audit" gehört sichtbar in den Hero, nicht nur in den Footer.

---

## 2. So funktioniert es

**Zwei Geheimnisse öffnen deinen Tresor.**
Das eine ist dein Master-Passwort. Das andere ist ein Geheimer Schlüssel (Secret Key): 128 zufällige Bit, auf deinem Gerät erzeugt. Er verlässt deine Geräte nie. Du sicherst ihn einmal in deinem Notfall-Kit.

**Alles wird verschlüsselt, bevor es dein Gerät verlässt.**
Jeder Eintrag wird einzeln verschlüsselt. Bei der Synchronisierung wandern nur verschlüsselte Daten, egal ob über deinen Cloud-Ordner oder über unseren Server.

**Ein gestohlener Server gibt Angreifern nichts zum Knacken.**
Die Anmeldung läuft über OPAQUE. Der Server bekommt nie etwas, das aus deinem Passwort abgeleitet ist und sich offline durchprobieren ließe. Ohne deinen Geheimen Schlüssel reicht eine Kopie der Datenbank nicht.

**Absichtlich keine Hintertür.**
Wenn du Master-Passwort und Notfall-Kit verlierst, kann niemand deine Daten retten, wir auch nicht. Das ist der Preis von Zero Knowledge. Wir sagen es dir vorher.

*Link: Die Kryptospezifikation lesen.*

---

## 3. Funktionen

### Auf dem Desktop (in der Entwicklungsversion verfügbar)
Windows, macOS und Linux. Zugangsdaten, sichere Notizen, Karten, Identitäten, SSH-Schlüssel, API-Schlüssel, WLAN-Daten und Dateianhänge, mit eigenen Feldern und Tags. Befehlspalette, ganz ohne Maus bedienbar. Schnellentsperren mit Windows Hello oder Touch ID. Automatische Sperre und automatisches Leeren der Zwischenablage.

### Browser-Erweiterung (Entwicklungsversion, noch in keinem Store)
Für Chromium-Browser und Firefox. Füllt genau eine Anmeldung nach deinem Klick, nie beim Laden einer Seite. Abgleich mit der exakten Herkunft (Origin), kein Ausfüllen in fremden Frames, kein Ausfüllen über unverschlüsseltes HTTP. Die Erweiterung spricht über einen lokalen, authentifizierten Kanal mit der Desktop-App.

### Passkeys (in der Entwicklungsversion verfügbar)
Erstelle und nutze Passkeys im Browser. Jede Nutzung bestätigst du in der Desktop-App. Import aus Bitwarden, KeePassXC und dem FIDO-Austauschformat.

### Einmalcodes
TOTP, HOTP und Steam Guard. QR-Codes liest du vom Bildschirm oder aus einem Bild ein. Die Codes können in einem eigenen Tresor liegen, der einen Hardware-Schlüssel braucht.

### SSH-Agent
Stelle deine SSH-Schlüssel aus dem Tresor bereit. Jede Nutzung musst du freigeben.

### Import
Hol deine Daten aus Enpass, Bitwarden, KeePass (KDBX 4), 1Password (1PUX) und CSV. Vor dem Speichern siehst du eine Vorschau.

### Generator und Sicherheitsbericht
Zufällige Passwörter und Passphrasen aus einer kryptografischen Zufallsquelle, mit Live-Anzeige der Entropie. Ein Offline-Bericht zu schwachen, doppelt genutzten und alten Passwörtern. Nichts verlässt dein Gerät.

### Synchronisierung über deinen eigenen Cloud-Ordner **(kommt mit M6)**
Zeig {NAME} einen Ordner in Dropbox, iCloud, OneDrive oder Nextcloud. Die Dateien bleiben verschlüsselt. Dein Anbieter sieht nur, was in unserer Spezifikation aufgeführt ist.

### Gehostete Synchronisierung **(kommt mit M6)**
Du willst dich um nichts kümmern? Nutze die Synchronisierung über unseren Server, oder betreibe den Server selbst mit einem einzigen Compose-Befehl.

### Außerdem geplant
Passwort-Rotation und Prüfung auf Datenlecks (M7), ein Kommandozeilenwerkzeug (M8), Apps für iOS und Android (M10).

---

## 4. Sicherheit und Ehrlichkeit

**{NAME} hat noch kein Audit hinter sich.** Eine unabhängige Sicherheitsprüfung hat bisher nicht stattgefunden. Wir wollen vor Version 1.0 eine beauftragen. Bis dahin: Vertraue {NAME} keine Geheimnisse an, die du nicht verlieren kannst, und behalte deinen bisherigen Manager, solange du testest.

**Was wir veröffentlichen**
- Die Kryptospezifikation, mit jedem Verfahren und jeder Bibliothek beim Namen. Wir nutzen etablierte Bausteine (Argon2id, XChaCha20-Poly1305, HKDF, OPAQUE, HPKE) und erfinden keine eigenen.
- Das Bedrohungsmodell, samt Liste der Restrisiken, die wir bewusst in Kauf nehmen. Beispiele: Schadsoftware auf einem entsperrten Rechner kann mitlesen, was du siehst. Und wer bei einem gestohlenen Gerät an den Geheimen Schlüssel kommt, muss nur noch dein Master-Passwort knacken.
- Jede Entscheidung als schriftliches Protokoll.

**Was der Server sieht (gehostete Synchronisierung)**
Deine Konto-E-Mail, Geräte-IDs und -Namen, Tresor- und Eintrags-IDs, Größen und Upload-Zeiten sowie verschlüsselte Daten. Der Server sieht **keine** Titel, URLs, Benutzernamen, Tags, Tresornamen oder Dateinamen. IP-Adressen liegen nur im Arbeitsspeicher, für die Ratenbegrenzung, und werden nie auf die Platte geschrieben. Die vollständige Tabelle steht in Abschnitt 12 der Spezifikation.

**Was dein Cloud-Anbieter sieht (Ordner-Synchronisierung)**
Dateinamen, Größen, Anzahlen und Änderungszeiten, aber keine Inhalte. Wer Zugriff auf den Ordner hat, kann außerdem Dateien löschen oder zurückhalten. Siehe Abschnitt 12.1 der Spezifikation.

**Keine Telemetrie.** Keine Analyse-Tools, keine Skripte von Dritten, keine Absturzberichte, die irgendwohin gehen.

---

## 5. Kostenlos und bezahlt

| | **Kostenlos** | **Gehostete Synchronisierung** |
|---|---|---|
| Preis | 0 | {PRICE} pro {PERIOD} |
| Desktop-App und Browser-Erweiterung | ja | ja |
| Passkeys, Einmalcodes, SSH-Agent, Import | ja | ja |
| Sync über deinen eigenen Cloud-Ordner (Dropbox, iCloud, OneDrive, Nextcloud) **(kommt)** | ja | ja |
| Sync über unseren Server **(kommt)** | nein | ja |

Die App ist kostenlos, ebenso die Synchronisierung über einen Ordner, der dir gehört. Das Abo bezahlt den Betrieb des Sync-Servers. Die endgültigen Konditionen stehen zum Start fest.

---

## 6. Für Teams (kommt)

Kleines Team? Gemeinsame Tresore, sauberes Offboarding und Notfallzugriff sind für den Meilenstein M6b geplant. Es gibt sie **noch nicht.** Das Design erlaubt, dass ein Team seine Daten im eigenen Dropbox-Business-Teamordner oder auf einem selbst betriebenen Server hält. Wenn das für euch passt: Trag dich in die Liste ein und sag uns, was ihr braucht.

**Button:** Sag mir Bescheid, wenn Teams bereit sind

---

## 7. Downloads

**Prüfe, was du installierst. (kommt mit M9)**
Release-Versionen werden signiert ausgeliefert, mit Prüfsummen, einer Software-Stückliste (SBOM) und einer Anleitung, den Build nachzubauen. Bis M9 gibt es nur Entwicklungsversionen, und die sind nicht signiert.

| Plattform | Stand |
|---|---|
| Desktop für Windows, macOS, Linux | Entwicklungsversion |
| Erweiterung für Chrome, Edge, Firefox | Entwicklungsversion, Store-Einträge folgen |
| iOS, Android | geplant (M10) |
| Kommandozeile | geplant (M8) |

---

## 8. Häufige Fragen

**1. Ist {NAME} sicher?**
Es ist sorgfältig gebaut und darauf ausgelegt, dem Diebstahl eines Servers standzuhalten. Unabhängig geprüft wurde es aber noch nicht. Behandle es als Beta und bewahre von allem Wichtigen eine zweite Kopie auf.

**2. Wird es ein Audit geben?**
Wir wollen vor 1.0 ein unabhängiges Audit beauftragen und das Ergebnis veröffentlichen. *(Vom Inhaber zu bestätigen.)* Das Wort „geprüft" benutzen wir erst, wenn der Bericht vorliegt.

**3. Was, wenn ich mein Master-Passwort vergesse?**
Du brauchst dein Notfall-Kit mit dem Geheimen Schlüssel und ein Master-Passwort. Hast du beides verloren, kann niemand die Daten wiederherstellen.

**4. Kann ich von meinem jetzigen Manager umziehen?**
Ja, aus Enpass, Bitwarden, KeePass (KDBX 4), 1Password (1PUX) und CSV. Wer von LastPass kommt, exportiert als CSV und importiert die Datei. Passkeys lassen sich aus Bitwarden, KeePassXC und dem FIDO-Austauschformat übernehmen.

**5. Wie funktioniert die Synchronisierung?**
Sie ist noch nicht veröffentlicht. Sobald sie das ist, wählst du: ein Ordner in deinem eigenen Cloud-Speicher oder unser gehosteter Server. Beide transportieren nur verschlüsselte Daten.

**6. Was kostet die gehostete Synchronisierung?**
{PRICE}. Die App und die Ordner-Synchronisierung bleiben kostenlos.

**7. Sammelt ihr Nutzungsdaten?**
Nein. Keine Analyse-Tools, keine Telemetrie. Wir zählen Downloads und öffentliche GitHub-Aktivität, nichts über dich.

**8. Wo ist der Code?**
Spezifikation und Bedrohungsmodell sind öffentlich. Lizenz und Quellcode-Zugang: {LICENSE_STATEMENT} (Entscheidung steht noch aus).

**9. Gibt es eine mobile App oder ein Kommandozeilenwerkzeug?**
Noch nicht. Mobile ist für M10 geplant, die Kommandozeile für M8.

**10. Kann mein Team es nutzen?**
Noch nicht. Teamfunktionen sind für M6b geplant. Die erste Version richtet sich an Einzelpersonen.

---

## 9. Footer

{NAME} (Arbeitstitel). Software ohne Sicherheitsaudit.
Links: Kryptospezifikation, Bedrohungsmodell, Architekturentscheidungen, Sicherheitsrichtlinie (M9), Änderungsprotokoll, Datenschutz: wir erheben nichts, Impressum {IMPRINT}, Kontakt {CONTACT_CHANNEL}.
Sprachen: English, Deutsch.
