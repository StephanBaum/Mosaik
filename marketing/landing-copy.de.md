# Landingpage-Texte (Deutsch)

> **Arbeitstitel.** {NAME} steht für den Produktnamen (Arbeitstitel „Mosaic“), {DOMAIN} für die Domain. Beides steht noch nicht fest. Die Lizenz ist offen, deshalb nennt der Text keine und sagt nie „Open Source“.
>
> **Rahmen.** Geschrieben für den Launch-Tag des fertigen Produkts: Alle Meilensteine bis M10 sind ausgeliefert (Vorgabe des Owners, 07.11.2026). Vor der Veröffentlichung jede Zeile gegen `docs/PROGRESS.md` prüfen und streichen oder kennzeichnen, was noch nicht ausgeliefert ist. Aussagen folgen `positioning.md` §5 bis §9; „nur {NAME}“ bezieht sich auf die Passwortmanager der Vergleichsseite.
>
> **Reihenfolge** wie im Designkonzept (`docs/superpowers/specs/2026-11-07-landing-page-design.md`, Branch `feat/web`): Hero → verschlüsselt, bevor es geht → was ein gestohlener Server enthält → zwei Geheimnisse → keine Hintertür → Funktionen → Sicherheit → Preise und Teams → Download → FAQ. Zwei Kapitel sind **NEU**: „Beim Lügen erwischt“ und „Nur {NAME}“. **Kinetische** Zeilen sind bewusst kurz; **Labels** sind 12 px Mono in Versalien, höchstens 28 Zeichen. Anrede: du, wie in der App. Produktbegriffe wie in der App: Secret Key, Emergency Kit, Passkey, Login.

---

## 1. Hero (M1, lebendes Mosaik)

**Label:** ENDE-ZU-ENDE-VERSCHLÜSSELT

### Überschriften zur Auswahl

| # | Überschrift | Unterzeile | Hauptbutton |
|---|---|---|---|
| **1 (Empfehlung)** | **Vertrau keinem Server.<br>Nicht mal unserem.** | Jeder Eintrag wird auf deinem Gerät verschlüsselt, bevor er es verlässt. Synchronisiere über deinen eigenen Cloud-Ordner oder unseren Server. Und spielt ein Server dir alte Daten vor, hält Änderungen zurück oder zeigt deinen Geräten verschiedene Stände, merkt {NAME} das. | {NAME} kostenlos holen |
| 2 | **Deine Cloud oder unsere.<br>Lesen kannst nur du.** | Dropbox, iCloud, OneDrive, Nextcloud, der Ordner deines Teams oder unser Server. Überall dieselbe Ende-zu-Ende-Verschlüsselung, und eine App, die merkt, wenn der Speicher lügt. | {NAME} kostenlos holen |
| 3 | **Der Server kann lügen.<br>Dein Tresor merkt es.** | Andere Passwortmanager glauben ihrem Server aufs Wort. {NAME} prüft nach: Jedes Gerät erkennt zurückgespielte, eingefrorene und gespaltene Daten. | Kostenlos starten |
| 4 | **Überall gespeichert.<br>Nur bei dir lesbar.** | Passwörter, Passkeys, Einmalcodes und SSH-Schlüssel, einzeln auf deinem Gerät versiegelt und über den Speicher synchronisiert, den du wählst. | {NAME} kostenlos holen |
| 5 | **Jedes Geheimnis. Jedes Gerät.<br>Kein blindes Vertrauen.** | Desktop, Browser, iPhone und Android, ein verschlüsselter Tresor, synchronisiert über deine Cloud oder unsere. Nichts musst du uns glauben: Die Spezifikation ist öffentlich. | {NAME} herunterladen |

**Warum 1:** Es sagt das, was kein anderer großer Passwortmanager in unserem Vergleich tut (er rechnet mit einem feindlichen Server und prüft nach), es stimmt für beide Arten der Synchronisierung, es passt in zwei kinetische Zeilen, und „unserem“ ist der selbstbewusste Dreh. Kinetischer Moment: „unserem“ wechselt von Strichstärke 400 auf 720, wenn das Mosaik landet (statt „beide“ im Designkonzept). Bei 360 px prüfen: „Vertrau keinem Server.“ muss auf zwei Zeilen umbrechen dürfen.

**Zweitbutton:** Sieh, was der Server sieht
**Vertrauenszeile:** Kryptospezifikation öffentlich · Bedrohungsmodell öffentlich · Null Telemetrie · Noch kein unabhängiges Audit

> Fürs Design: Die Audit-Zeile bleibt im Hero. Erst wenn ein veröffentlichter Auditbericht existiert, wird daraus „Unabhängig geprüft von {AUDITOR}. Zum Bericht.“, keinen Tag früher.

---

## 2. Verschlüsselt, bevor es geht (M2, fixiert)

**Label (links):** DIESES GERÄT
**Label (rechts):** SERVER

**Kinetische Überschrift:** Verschlüsselt, bevor es dein Gerät verlässt.

**Text:**
Jedes Login, jede Karte, jede Notiz, jeder Passkey und jede Datei wird einzeln auf deinem Gerät verschlüsselt, mit XChaCha20-Poly1305. Bei der Synchronisierung wandert nichts anderes. Nicht in deinen Cloud-Ordner, nicht auf unseren Server.

**Bildunterschriften (in Scroll-Reihenfolge):**
1. Deine Daten. Hier lesbar.
2. Eintrag für Eintrag versiegelt.
3. Unterwegs nur als Chiffretext.
4. Mehr hätte ein gestohlener Server nicht.

**Metadaten-Labels:** DATENSATZ-ID · GRÖSSE · HOCHGELADEN · GERÄT

**Hinweis zur Grafik:** Illustration. Die Bytes sind zufällig und in deinem Browser erzeugt. Das echte Datenformat steht in der Kryptospezifikation.

---

## 3. Was ein gestohlener Server enthält (M3, Glaslinse)

**Kinetische Überschrift:** Ein gestohlener Server enthält nur Rauschen.

**Text:**
Wer unsere Datenbank stiehlt, bekommt Chiffretext, IDs, Größen und Zeitstempel. Keine Titel, keine URLs, keine Benutzernamen, keine Tresornamen, keine Dateinamen. Die Anmeldung läuft über OPAQUE, deshalb liegt auf dem Server nicht einmal ein Passwort-Hash, den man durchprobieren könnte.

**Umschalter:** Sicht des Servers | Deine Sicht
**Live-Region:** Das speichert der Server. Das hier kann er daraus nicht machen.

**Tag-Spalten**
- **DAS SIEHT DER SERVER:** E-Mail · Geräte-IDs · Datensatz-IDs · Größen · Uploadzeiten · Chiffretext
- **DAS SIEHT ER NIE:** Titel · URLs · Benutzernamen · Passwörter · Notizen · Tresornamen · Dateinamen · dein Master-Passwort · deinen Secret Key

**Absatz zur Ordner-Synchronisierung:**
Du synchronisierst über deinen eigenen Cloud-Ordner? Dein Anbieter sieht verschlüsselte Dateien, ihre Namen, Größen und Änderungszeiten. Was drinsteht, sieht er nicht. *Link: §12.1 der Spezifikation*

---

## 3b. Beim Lügen erwischt **NEU** (Vorschlag für einen Signature-Moment)

> Fürs Design: ein kurzes, ruhiges Kapitel. Bildidee: Zwei Gerätekarten veröffentlichen signierte Prüfpunkte, der Server zeigt einer davon einen älteren oder anderen Stand, die Prüfsummen passen nicht zusammen, ein ruhiger Warnhinweis im Stil der App erscheint. Daten erfunden und als „Beispiel“ gekennzeichnet.

**Label:** LÜGEN-ERKENNUNG

**Kinetische Überschrift:** Der Server kann lügen. Deine Geräte merken es.

**Text:**
Die meisten Passwortmanager glauben, was ihr Server ihnen zeigt. {NAME} nicht. Jedes Gerät signiert einen Prüfpunkt dessen, was es gesehen hat. Spielt dir ein Server oder ein Cloud-Ordner einen alten Stand vor, hält er still und leise Änderungen zurück oder zeigt er zwei deiner Geräte verschiedene Fassungen deines Tresors, schlägt {NAME} Alarm. Und zwar so lange, bis du nachgesehen hast.

**Drei Schritte (Label + eine Zeile):**
- **ZURÜCKGESPIELT** Alte Daten als neu verkauft? Abgelehnt.
- **EINGEFROREN** Änderungen zurückgehalten? Du wirst gewarnt.
- **GESPALTEN** Deine Geräte sehen verschiedene Tresore? „Server inkonsistent“.

**Warnhinweis (Beispiel-UI):** Server inkonsistent. Zwei deiner Geräte haben verschiedene Fassungen dieses Tresors gesehen. Nichts ist verloren. Jetzt ansehen.

**Kleingedrucktes:** Erkennung, keine Zauberei: Ein Server kann Daten weiterhin zurückhalten, nur nicht unbemerkt. Für die Spaltung vergleichen sich deine Geräte, sobald sie auf demselben Stand sind. So funktioniert es: Spezifikation §10.

---

## 4. Zwei Geheimnisse (M4, fixiert)

**Kinetische Überschrift:** Eins merkst du dir. Eins behält dein Gerät.

**Text:**
Dein Tresor öffnet sich nur mit zwei Geheimnissen: deinem Master-Passwort und einem Secret Key, 128 zufälligen Bit, die auf deinem Gerät entstehen und nie verschickt werden. Selbst wer unsere ganze Datenbank hätte und dein Passwort erriete, stünde noch 128 Bit vor der Tür.

**Labels der Schlüsselkacheln**
- **MASTER-PASSWORT** Nur in deinem Kopf.
- **SECRET KEY** Nur auf deinen Geräten. 128 zufällige Bit.

**Verbindungslabel:** Argon2id + HKDF → Kontoschlüssel
**Unterschrift nach der Welle:** Wieder lesbar. Nur hier.
**Hinweis zum Beispielschlüssel:** Ein Beispiel, in deinem Browser erzeugt. Nutze nie einen Schlüssel von einer Website: Die App erzeugt deinen auf deinem Gerät und verschickt ihn nie.
**Button:** Noch einen erzeugen

---

## 5. Keine Hintertür (statisch, Strichstärke pro Wort)

**Kinetische Überschrift:** Keine Hintertür. Mit Absicht.

**Großer Satz:** Vergisst du dein Master-Passwort, holt niemand deine Daten zurück. Auch wir nicht.

**Text:**
Kein Generalschlüssel, kein Support-Zugang, keine „Kontowiederherstellung“, die in Wahrheit eine Hintertür ist. Dein Emergency Kit bringt deinen Secret Key auf ein neues Gerät, und über den Notfallzugriff kommt eine Person deines Vertrauens an deine Daten, wenn du das vorher einrichtest. So sieht Zero Knowledge aus, und wir sagen es dir vor der Anmeldung, nicht danach.

---

## 6. Was {NAME} kann (Bento)

**Kinetische Überschrift:** Jedes Geheimnis. Jedes Gerät.

**Einleitung:** Ein Tresor für alles, was du schützt, auf allem, was du nutzt.

| Kachel | Überschrift | Text |
|---|---|---|
| Desktop (6×2, Mini-Tresor) | **Schnell auf jedem Desktop.** | Windows, macOS und Linux. Tastenkürzel, tippen, fertig: Jede Aktion ist einen Tastendruck entfernt. Entsperren mit Windows Hello, Touch ID oder einem Sicherheitsschlüssel. |
| Browser (3×2, Klick → Ausfüllen) | **Kein Klick, kein Ausfüllen.** | Chrome, Edge und Firefox. Füllt ein Login erst nach deinem Klick aus, nur auf genau der Seite, zu der es gehört, nie in einem versteckten Frame, nie über unverschlüsseltes HTTP. Nachgemachte Domains bekommen eine Warnung, nicht dein Passwort. |
| Passkeys (3×1) | **Passkeys, überall.** | Erstelle und nutze Passkeys im Browser und auf dem Handy, synchronisiert und Ende-zu-Ende-verschlüsselt. |
| Einmalcodes (3×1, TOTP-Ring) | **Codes, die sich selbst eintragen.** | TOTP, HOTP und Steam Guard. QR-Code vom Bildschirm scannen, und der Code landet direkt nach dem Passwort im Feld. Noch sicherer: in einem Tresor, den nur dein Hardware-Schlüssel öffnet. |
| SSH-Agent | **Deine SSH-Schlüssel an der kurzen Leine.** | Ein eingebauter SSH-Agent liefert Schlüssel aus deinem Tresor. Vor jeder Nutzung fragt er dich. |
| Import | **In Minuten umgezogen.** | Enpass, Bitwarden, KeePass, 1Password und CSV, mit Vorschau und Dublettenprüfung. Passkeys ziehen über das FIDO-Austauschformat mit um. |
| Synchronisierung | **Deine Cloud oder unsere.** | Dropbox, iCloud Drive, OneDrive, Nextcloud, ein NAS oder ein Team-Ordner. Oder unser Server. Oder dein eigener, mit einem einzigen Docker-Compose-Befehl. Funktioniert offline, führt sauber zusammen und verliert nie stillschweigend eine Änderung. |
| Teilen | **Teilen, ohne dem Mittelsmann zu trauen.** | Geteilte Tresore, für jede Person einzeln versiegelt. Einmal den Sicherheitscode vergleichen, und kein untergeschobener Schlüssel kommt an dir vorbei. Entfernst du jemanden, wird der Tresor neu verschlüsselt. |
| Notfallzugriff | **Ein Weg rein für die Menschen, denen du vertraust.** | Benenne jemanden. Bist du nicht erreichbar, bekommt die Person nach deiner Wartezeit Zugriff, außer du lehnst ab. |
| Rotation | **Wechseln, bevor es wehtut.** | Eine Datenleck-Prüfung, die nur fünf Zeichen eines Hashs verschickt. Eine „Jetzt ändern“-Liste, geführte Passwortwechsel auf jeder Website und Konnektoren, die SSH-Schlüssel, PostgreSQL- und MySQL-Passwörter und Webhooks nach Zeitplan wechseln. |
| Sicherheitsbericht | **Wisse, wo du verwundbar bist.** | Schwache, mehrfach genutzte, von Lecks betroffene und alte Logins, Seiten ohne HTTPS und fehlende Zwei-Faktor-Anmeldung, jeweils mit Lösung per Klick. |
| Generator (live) | **Passwörter, die Websites auch nehmen.** | Zufällige Passwörter und EFF-Passphrasen, mit Stärkeanzeige in Echtzeit und passend zu den Passwortregeln jeder Seite. |
| Kommandozeile | **Geheimnisse im Terminal.** | Suchen, abrufen, erzeugen und Einmalcodes holen, direkt aus der Shell. Geheimnisse landen direkt in der Umgebung eines Befehls. Ausgabe als JSON. |
| Mobil | **Dein Handy, vollwertig.** | iOS und Android mit System-Autofill und Passkeys, auf demselben Rust-Kern wie der Desktop. |
| Dateien und Felder | **Alles, was du aufheben musst.** | Logins, Karten, Identitäten, Notizen, API-Schlüssel, WLAN, verschlüsselte Dateien und eigene Felder. |

**Chip-Labels:** DESKTOP · BROWSER · iOS · ANDROID · KOMMANDOZEILE

---

## 6b. Nur {NAME} **NEU** (Abgrenzung)

> Fürs Design: keine Logos oder Namen von Mitbewerbern (die stehen auf der Vergleichsseite). Drei große Aussagen, jede mit einem Mono-Link auf den Beleg in der Spezifikation. Die Regel „keine Vergleichstabelle“ aus dem Designkonzept gilt weiter.

**Kinetische Überschrift:** Was andere nicht können.

**Einleitung:** Wir haben {NAME} mit den größten Namen unter den Passwortmanagern verglichen. Das hier kann keiner von ihnen.

1. **Dein Ordner oder unser Server. Du entscheidest, die Verschlüsselung bleibt.**
   Synchronisiere über deinen eigenen Cloud-Ordner, den Dropbox-Ordner deines Teams, unseren Server oder einen eigenen, alles mit einem Ende-zu-Ende-Protokoll. Wechseln kannst du jederzeit. *Beleg: Spezifikation §10.1*

2. **Es erwischt einen lügenden Server.**
   Jedes Gerät erkennt zurückgespielte, eingefrorene und gespaltene Daten und warnt dich deutlich: „Server inkonsistent“. Andere verlassen sich darauf, was ihr Server ihnen zeigt. *Beleg: Spezifikation §10*

3. **OPAQUE-Anmeldung über zwei Geheimnissen.**
   Der Server hat nie etwas aus deinem Passwort, das sich offline durchprobieren ließe, und einer gestohlenen Datenbank fehlen ohne deinen Secret Key 128 Bit. *Beleg: Spezifikation §5, §9*

**Und anders als die meisten Passwortmanager**
- Ausfüllen nur bei exakt passender Herkunft. `login.bank.example` wird nie auf `boese.bank.example` ausgefüllt.
- Kein automatisches Ausfüllen beim Laden einer Seite. Nicht mal als Einstellung.
- Null Telemetrie. Keine Analyse, keine Absturzberichte, keine Aufrufe an Dritte.
- Jedes Feld, das unser Server sieht, steht in der Spezifikation.
- Ein Bedrohungsmodell, das auch die Risiken nennt, die wir bewusst tragen.

*Link: {NAME} im Vergleich mit 1Password, Bitwarden, Proton Pass, KeePassXC und Enpass →*

---

## 7. Sicherheit, offen gelegt

**Kinetische Überschrift:** Nachlesen statt glauben.

**Audit-Hinweis (immer sichtbar, h3 mit Warnlinie):**
**Noch kein unabhängiges Audit.** Alles, was ein Prüfer braucht, ist schon öffentlich: Spezifikation, Bedrohungsmodell, jede Designentscheidung. *(Owner: „Ein unabhängiges Audit ist für {AUDIT_DATE} beauftragt“ erst ergänzen, wenn es beauftragt ist.)*

**Papierkarten**
- **Kryptospezifikation** · Jedes Verfahren und jede Bibliothek mit Namen. Argon2id, XChaCha20-Poly1305, HKDF, OPAQUE, HPKE. Wir setzen zusammen, wir erfinden nichts. *§1 bis §16*
- **Bedrohungsmodell** · Angreifer, Gegenmaßnahmen und eine Liste der Risiken, die wir bewusst tragen. *ab R1*
- **Entscheidungsprotokoll** · Jede Sicherheitsentscheidung, samt der Alternativen, die wir verworfen haben. *ADR-Log*
- **Meldeverfahren** · Wie du eine Schwachstelle meldest und was wir dir zusagen. *SECURITY.md*

**Gebaut, als käme es darauf an**
Speichersicherer Rust-Kern ohne unsicheren Code. Geheimnisse werden nach Gebrauch aus dem Speicher gelöscht. Jeder Parser wird gefuzzt. Eine feindliche Test-Website greift die Browser-Erweiterung bei jedem Build an. Releases sind signiert, mit SBOM und einer Anleitung zum Nachbauen.

**Zeile zur Telemetrie:** Keine Analyse. Keine Absturzberichte. Kein Tracking. Der einzige Aufruf nach draußen ist die Datenleck-Prüfung, und die verschickt fünf Zeichen eines Hashs.

---

## 8. Preise und Teams

**Kinetische Überschrift:** Kostenlos, wo es zählt.

| | **Kostenlos** | **Gehostet** | **Teams** |
|---|---|---|---|
| Preis | 0 | {PRICE} pro {PERIOD} | {TEAM_PRICE} pro Person und {PERIOD} |
| Apps für Desktop, Browser, iOS, Android und Kommandozeile | ✓ | ✓ | ✓ |
| Passkeys, Einmalcodes, SSH-Agent, Import, Rotation | ✓ | ✓ | ✓ |
| Synchronisierung über deinen eigenen Cloud-Ordner | ✓ | ✓ | ✓ |
| Synchronisierung über unseren Server, mit Zwei-Faktor-Anmeldung | | ✓ | ✓ |
| Notfallzugriff | {TBD} | ✓ | ✓ |
| Geteilte Team-Tresore, im Team-Ordner oder auf unserem Server | | | ✓ |
| Server selbst betreiben | ✓ | | {TBD} |

**Zeile unter der Tabelle:** Die App ist kostenlos, auch die Synchronisierung über Speicher, den du schon hast. Du zahlst nur, wenn wir den Server für dich betreiben sollen.

**Team-Band:** **Die Geheimnisse deines Teams in der Cloud deines Teams.** Geteilte Tresore in einem Dropbox-Business-Team-Ordner, auf Nextcloud oder auf unserem Server. Wer das Team verlässt, verliert den Zugriff: Der Tresor wird neu verschlüsselt. Notfallzugriff inklusive. **Button:** Team starten

> Owner: Die Tarifmatrix ist ein Vorschlag; `{TBD}`-Felder und Preise sind offene Entscheidungen (README).

---

## 9. Download und Prüfung

**Kinetische Überschrift:** Prüf, was du installierst.

**Text:** Jedes Release ist signiert und kommt mit Prüfsummen, einer SBOM und einer Anleitung, es selbst aus dem Quellcode zu bauen.

**Plattform-Karten:** Windows · macOS · Linux · Chrome · Edge · Firefox · iOS · Android · Kommandozeile

**Prüfschritte**
1. **Herunterladen:** Installer und Signatur.
2. **Prüfen:** Prüfsumme und Signatur mit einem Befehl.
3. **Nachbauen,** wenn du ganz sicher sein willst. *Link: Anleitung zum reproduzierbaren Build*

---

## 10. FAQ

**1. Ist {NAME} sicher?**
{NAME} ist so gebaut, dass ein gestohlener oder feindlicher Server nichts Brauchbares bekommt und deine Geräte merken, wenn der Server lügt. Ein unabhängiges Audit gab es noch nicht, und wir behaupten es erst, wenn ein Bericht veröffentlicht ist. Alles, was du brauchst, um selbst zu urteilen, ist öffentlich.

**2. Was passiert, wenn euer Server gehackt wird?**
Der Angreifer bekommt Chiffretext und die Metadaten aus §12 der Spezifikation. Ohne dein Master-Passwort und deinen Secret Key gibt es nichts zu entschlüsseln, und dank OPAQUE gibt es keinen Passwort-Hash, den man offline knacken könnte.

**3. Und wenn sich der Server selbst gegen mich wendet?**
Genau dafür ist {NAME} gebaut. Er kann deine Daten nicht fälschen und dir keinen alten Stand unbemerkt unterschieben. Hält er Änderungen zurück oder zeigt er deinen Geräten verschiedene Stände, wirst du gewarnt.

**4. Geht Synchronisierung auch ohne euren Server?**
Ja, kostenlos. Richte {NAME} auf einen Ordner in Dropbox, iCloud Drive, OneDrive, Nextcloud, auf einem NAS oder in einem Dropbox-Business-Team-Ordner ein. Dein Anbieter sieht immer nur verschlüsselte Dateien.

**5. Was, wenn ich mein Master-Passwort vergesse?**
Dann kann niemand deine Daten wiederherstellen. Wir nicht, und auch nicht, wer deinen Server betreibt. Dein Emergency Kit enthält den Secret Key für neue Geräte, und über den Notfallzugriff kommt eine Person deines Vertrauens nach einer Wartezeit an deine Daten, wenn du das vorher einrichtest. Einen dritten Weg gibt es nicht, mit Absicht.

**6. Kann ich aus meinem bisherigen Passwortmanager importieren?**
Ja: Enpass, Bitwarden, KeePass (KDBX 4), 1Password (1PUX) und CSV, LastPass und andere also über CSV. Passkeys kommen aus Bitwarden, KeePassXC und dem FIDO-Austauschformat mit.

**7. Läuft es auf meinem Handy?**
Ja. iOS und Android, mit System-Autofill und Passkeys.

**8. Sammelt ihr Nutzungsdaten?**
Nein. Keine Analyse, keine Telemetrie, keine Absturzberichte. Es gibt auch keinen Schalter dafür, weil nichts gesammelt wird.

**9. Kann mein Team {NAME} nutzen?**
Ja. Geteilte Tresore, sauberes Entziehen von Zugriffen und Notfallzugriff, mit den Daten in eurem eigenen Team-Ordner oder auf unserem Server.

**10. Wo ist der Code?**
Kryptospezifikation, Bedrohungsmodell und Entscheidungsprotokoll sind öffentlich. {LICENSE_STATEMENT}

**11. Was kostet es?**
Die Apps und die Synchronisierung über deinen Ordner sind kostenlos. Gehostete Synchronisierung kostet {PRICE} pro {PERIOD}, Teams {TEAM_PRICE} pro Person.

---

## 11. Footer

{NAME} · {AUDIT_FOOTER, Standard: „Noch kein unabhängiges Audit.“}
Links: Kryptospezifikation · Bedrohungsmodell · Entscheidungsprotokoll · Sicherheitsrichtlinie · Vergleich · Änderungen · Datenschutz: Wir sammeln nichts · Impressum {IMPRINT} · Kontakt {CONTACT_CHANNEL}
Keine Cookies, also kein Banner. · English | Deutsch

---

## 12. 404

**Überschrift:** Diese Seite ist Chiffretext.
**Link:** Zurück zum lesbaren Teil
