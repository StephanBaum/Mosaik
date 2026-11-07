# Landingpage-Texte: Deutsch

Fertige Seitentexte in Seitenreihenfolge. Jede Sektion hat ihre ID (Anker und Präfix der i18n-Schlüssel), Eyebrow, Headline, Fließtext, Punkte und Buttons. Hinweise für das Design stehen in [eckigen Klammern] und erscheinen nicht auf der Seite. Einziger Platzhalter ist `{PRICE}`. Die IDs sind dieselben wie in `landing-page.en.md`.

---

## nav

- **Logo:** Mosaic
- **Links:** So funktioniert's (`#how-it-works`) · Funktionen (`#features`) · Preise (`#pricing`) · Teams (`#teams`) · FAQ (`#faq`)
- **Button:** Kostenlos laden (`#download`)
- **Sprachwahl:** EN · DE
- **Menü-Button (aria):** Menü öffnen / Menü schließen

---

## hero

[Vollflächiges Hero-Bild: dunkle Acryl-Landschaft, offenes Feld mit einer einsamen Scheune an einem windigen Abend, Pixel-Sorting. Text links oder links unten auf dem dunkelsten Teil von Himmel oder Feld. Zeilen so kurz lassen wie hier umbrochen.]

- **Eyebrow:** Passwort-Manager für alle deine Geräte

**Headline-Optionen**

1. **Jeder Login. Ein Klick.<br>Nur für dich.** *(Empfehlung)*
2. **Merk dir ein Passwort.<br>Den Rest macht Mosaic.**
3. **Alles, womit du dich anmeldest.<br>An einem Ort, den nur du öffnest.**

[Warum 1: Das ganze Versprechen in sechs Wörtern: alles an einem Ort, automatisch ausgefüllt, privat. Drei kurze Takte, die über dem Bild gut stehen. Option 2 ist die wärmere, einfachere Alternative.]

- **Subhead:**
  Passwörter, Passkeys und 2FA-Codes an einem Ort.<br>
  Mit einem Klick ausgefüllt, auf jedem Gerät.<br>
  So verschlossen, dass nur du sie lesen kannst.
- **Primärer Button:** Kostenlos herunterladen (`#download`)
- **Sekundärer Button:** So funktioniert's (`#how-it-works`)
- **Zeile unter den Buttons:** Kostenlos für Windows, macOS, Linux, iPhone und Android.
- **Alt-Text Hero-Bild:** Eine einsame Scheune auf einem offenen Feld in der Abenddämmerung, das Gras biegt sich im Wind, gemalt in dunklem Acryl und in Streifen sortierter Pixel aufgelöst.

---

## problem

- **Eyebrow:** Der ganz normale Wahnsinn
- **Headline:** Zu viele Passwörter. Zu wenig Kopf dafür.

**Fließtext:**
Du hast mehr Konten, als sich irgendjemand merken kann. Also machst du, was alle machen: ein paar Lieblingspasswörter überall, ein paar auf einem Zettel, und beim Rest klickst du auf „Passwort vergessen“. Klappt prima. Bis es das nicht mehr tut.

**Punkte:**
- **Ein Leck, alle Türen offen.** Eine Seite, die du mal kurz benutzt hast, wird gehackt. Die Angreifer probieren deine E-Mail und dein Passwort überall sonst aus. Und wo du es wiederverwendet hast, sind sie drin.
- **Fälschungen sehen echt aus.** Ein Buchstabe anders in der Adresse, dasselbe Logo, dasselbe Anmeldefeld. Du tippst dein Passwort ein, bevor es dir auffällt.
- **Codes, Codes, Codes.** Jeder wichtige Login will zusätzlich einen sechsstelligen Code vom Handy. Handy raus, App suchen, schnell abtippen, bevor er abläuft.
- **Passkeys bleiben hängen.** Du legst auf dem Handy einen Passkey an, setzt dich an den Laptop, und da ist er nicht.

**Schlusszeile:**
Ein guter Passwort-Manager löst das alles. Die eigentliche Frage ist, welchem du alles anvertraust.

---

## where-it-lives

- **Eyebrow:** Wo dein Tresor liegt
- **Headline:** Die meisten Passwort-Manager lagern deinen Tresor auf ihren Servern.

**Fließtext:**
So synchronisieren sie deine Geräte. Dein Tresor ist verschlüsselt, das ist gut. Aber es heißt auch: Eine Firma hat Millionen Tresore an einem Ort. Für Angreifer ist das ein Ziel, für das sich jahrelange Arbeit lohnt.

**Geschichte 1**
- **Label:** 2022
- **Headline:** LastPass wird gehackt.
- **Text:** Angreifer brachen bei LastPass ein und nahmen Kopien der Tresore von Kunden mit. Die Passwörter darin waren verschlüsselt, die Website-Adressen nicht. Und ein gestohlener Tresor läuft nie ab: Die Angreifer können offline Master-Passwörter durchprobieren, so lange sie wollen, und niemand bekommt es je mit.

**Geschichte 2**
- **Label:** 2026
- **Headline:** Forscher drehen den Server gegen seine Nutzer.
- **Text:** Ein Team der ETH Zürich und der USI hat gezeigt: Ein gehackter Server eines Passwort-Managers kann mehr, als nur auf deinen Daten zu sitzen. Er kann die Leute, die Bitwarden, LastPass und Dashlane nutzen, aktiv angreifen, im schlimmsten Fall so weit, dass er an den Inhalt ihrer Tresore kommt. Manches davon ist inzwischen behoben. Die Lehre bleibt.

**Schlusszeile:**
Das heißt nicht, dass Passwort-Manager eine schlechte Idee sind. Sie sind immer noch viel sicherer als wiederverwendete Passwörter. Es heißt nur: Zwei Dinge zählen mehr, als dir bisher jemand gesagt hat. Wo dein Tresor liegt. Und ob deine App prüft, was sie zurückbekommt.

- **Quellenzeile (klein):** Quellen: LastPass, Hinweis zum Sicherheitsvorfall, Dezember 2022. Scarlata et al., „Zero Knowledge (About) Encryption“, USENIX Security 2026.

---

## how-it-works

- **Eyebrow:** So funktioniert Mosaic
- **Headline:** Auf deinem Gerät verschlossen. Gelagert, wo du willst. Jedes Mal geprüft.
- **Intro:** Mosaic ist so gebaut, dass deine Passwörter dir gehören, egal wo sie liegen.

**Schritt 1**
- **Nummer:** 01
- **Headline:** Zwei Schlüssel, beide in deiner Hand.
- **Text:** Dein Tresor öffnet sich nur mit zwei Schlüsseln. Dein Master-Passwort hast du im Kopf. Deinen Secret Key erzeugt dein Gerät bei der Einrichtung. Er bleibt auf deinen Geräten und in deinem Emergency Kit, das du ausdruckst oder speicherst. Alles wird verschlossen, bevor es dein Gerät verlässt. Eine gestohlene Kopie öffnet gar nichts, selbst wenn jemand dein Master-Passwort errät.

**Schritt 2**
- **Nummer:** 02
- **Headline:** Du entscheidest, wo synchronisiert wird.
- **Text:** Leg deinen Tresor in deinen eigenen Ordner bei Dropbox, iCloud Drive, OneDrive oder Nextcloud, oder in den Teamordner deiner Firma bei Dropbox Business. Oder wir übernehmen die Synchronisierung für dich. Das Schloss ist in beiden Fällen dasselbe. Deine Cloud speichert verschlüsselte Dateien und kann keinen einzigen Eintrag lesen.

**Schritt 3**
- **Nummer:** 03
- **Headline:** Es merkt, wenn etwas nicht stimmt.
- **Text:** Wenn ein Server oder ein Cloud-Ordner Mosaic eine alte Kopie deines Tresors unterjubelt, deine neuesten Änderungen zurückhält oder eine manipulierte Datei liefert, spielt Mosaic nicht still mit. Es warnt dich, und die Warnung bleibt, bis du sie dir angesehen hast.

- **Für Technikinteressierte (klein):** Jeder Eintrag wird einzeln mit XChaCha20-Poly1305 versiegelt, unter Schlüsseln, die per Argon2id aus deinem Master-Passwort und einem 128-Bit-Secret-Key abgeleitet sind. Signierte Checkpoints erkennen Rollbacks, eingefrorene Updates und auseinanderlaufende Kopien bei beiden Sync-Varianten. Nur veröffentlichte Standardbausteine.

[Visual-Vorschlag: drei Karten, eine pro Schritt. Schritt 3 zeigt den Warnhinweis der App in ruhigem Bernstein, beschriftet mit „Beispiel“.]

- **Alt-Text Visual Schritt 3:** Mosaic zeigt eine Warnung: Die synchronisierte Kopie des Tresors ist älter als eine, die deine Geräte schon kennen.

---

## features

- **Eyebrow:** Jeden Tag
- **Headline:** Alles, womit du dich anmeldest. Erledigt.
- **Intro:** Eine App für jedes Passwort, jeden Passkey und jeden Code, auf jedem Bildschirm, den du benutzt.

**Punkte (Headline + Text):**

1. **Autofill, das sich nicht reinlegen lässt.**
   Ein Klick, und Mosaic füllt deinen Login aus. Nur auf genau der Seite, zu der er gehört, nie in einem versteckten Rahmen, nie auf einer Doppelgänger-Seite. Auf einer Fälschung passiert einfach nichts.
2. **Passkeys auf allen Geräten.**
   Leg Passkeys im Browser und auf dem Handy an und nutze sie dort. Sie landen auf all deinen Geräten, statt auf einem festzustecken.
3. **2FA-Codes, die sich selbst eintragen.**
   Einmal den QR-Code scannen. Ab dann steht der Code direkt nach deinem Passwort im Feld. Kein Griff mehr zum Handy.
4. **Eine Desktop-App, die Spaß macht.**
   Windows, macOS und Linux. Mit einem Tastenkürzel findest du alles. Dein Tresor öffnet sich per Fingerabdruck, Gesicht oder Sicherheitsschlüssel.
5. **Dein Handy ist voll dabei.**
   iPhone und Android tragen Logins und Passkeys direkt in deinen Apps ein.
6. **Mehr als Passwörter.**
   Sichere Notizen, Dateien, Karten, WLAN-Passwörter, Wiederherstellungscodes, Software-Lizenzen. Und überall eigene Felder, wenn du sie brauchst.
7. **Starke Passwörter ohne Aufwand.**
   Ein Klick, und du hast ein starkes Passwort. Mosaic zeigt dir, welche alten schwach, doppelt oder in einem bekannten Datenleck aufgetaucht sind. Die wichtigsten zuerst.
8. **Teilen mit Leuten, denen du vertraust.**
   Teil einen Login oder einen ganzen Tresor mit Familie oder Kollegen. Nimmst du jemanden raus, bekommt der Tresor ein neues Schloss.
9. **Notfallzugang.**
   Bestimm eine Person deines Vertrauens. Falls dir etwas passiert, kann sie Zugang anfragen und bekommt ihn nach einer Wartezeit, die du festlegst. Außer du sagst Nein.
10. **SSH-Schlüssel, gut verwahrt.**
    Für Entwickler: Deine SSH-Schlüssel liegen im Tresor. Der eingebaute SSH-Agent fragt dich vor jeder Nutzung.
11. **Direkt im Terminal.**
    Der Befehl `mosaic` findet Logins, erzeugt Passwörter und holt 2FA-Codes in deine Skripte.
12. **Kein Tracking. Gar keins.**
    Keine Analyse-Tools, keine Werbung, keine Nutzungsdaten. Mosaic schaut dir nicht über die Schulter.

**Plattformleiste:** Windows · macOS · Linux · iPhone · Android · Chrome · Edge · Firefox · Kommandozeile

[Visual-Vorschlag: Bento-Raster. Große Kacheln für 1 (Klick, dann Ausfüllen), 2, 3 und 4 mit echten Produktansichten, der Rest als kleine Kacheln.]

- **Alt-Text Autofill:** Die Browser-Erweiterung von Mosaic füllt nach einem Klick einen Login auf der Anmeldeseite aus.
- **Alt-Text Desktop:** Die Desktop-App von Mosaic mit Suchfeld und einer Liste von Logins.
- **Alt-Text Handy:** Ein iPhone trägt einen gespeicherten Login in den Anmeldebildschirm einer App ein.

---

## only-mosaic

- **Eyebrow:** Nur bei Mosaic
- **Headline:** Was die anderen nicht können.
- **Intro:** Wir haben Mosaic mit den bekanntesten Passwort-Managern verglichen. Drei Dinge kann nur Mosaic.

**Punkte:**

1. **Deine Cloud oder unsere. Dasselbe Schloss.**
   Synchronisiere über deinen eigenen Cloud-Ordner, den Dropbox-Business-Ordner deines Teams oder unseren Server, und wechsle, wann du willst. Bei den anderen musst du dich entscheiden: nur ihre Cloud, oder dein eigener Speicher ohne gehostete Option.
2. **Es merkt, wenn ein Server es austricksen will.**
   Eine alte Kopie, die als neu verkauft wird, zurückgehaltene Änderungen, eine manipulierte Datei: Mosaic erkennt das und sagt es dir. Keiner der anderen, die wir verglichen haben, beschreibt etwas Vergleichbares.
3. **Unser Server kann keine Passwörter durchprobieren.**
   Wenn du dich bei der gehosteten Synchronisierung anmeldest, beweist Mosaic, dass es dein Passwort kennt, ohne dem Server irgendetwas zu geben, woran er geratene Passwörter testen könnte. Nicht das Passwort, nicht seinen Hash. Keiner der anderen, die wir verglichen haben, meldet dich so an.

**Und anders als die meisten Passwort-Manager:**
- Ohne deinen Klick wird nichts ausgefüllt. Und es gibt keine Einstellung, die das ändert.
- Logins werden nur auf genau der Adresse ausgefüllt, für die du sie gespeichert hast. Ein Login für login.deinebank.de landet nie auf angebote.deinebank.de.
- Keine Telemetrie, niemals. Nicht standardmäßig an, nicht standardmäßig aus. Es gibt sie schlicht nicht.

- **Für Technikinteressierte (klein):** 1: ein Sync-Protokoll über zwei Transportwege (Ordner und gehostet). 2: Erkennung von Rollback und Freeze bei beiden, dazu Fork-Erkennung über mehrere Geräte bei der gehosteten Synchronisierung. 3: Anmeldung per OPAQUE; die anderen nutzen SRP oder schicken einen clientseitigen Hash.
- **Link:** Zum ausführlichen Vergleich
[Design: Der Link führt zur Vergleichsseite. Keine Logos anderer Anbieter in dieser Sektion.]

---

## switch

- **Eyebrow:** Umsteigen
- **Headline:** In ein paar Minuten bist du eingezogen.

**Fließtext:**
Exportier deinen Tresor aus deiner alten App, öffne die Datei in Mosaic und schau dir die Vorschau an. Doppelte Einträge fallen auf, bevor irgendetwas gespeichert wird. Deine Passwörter funktionieren genau wie vorher, du musst nichts zurücksetzen.

**Punkte:**
- **Bring alles mit:** 1Password, Bitwarden, LastPass, KeePass, Enpass, Chrome oder jeder Tabellen-Export.
- **Passkeys ziehen mit um,** aus Apps, die das neue Austauschformat für Passkeys unterstützen.
- **Gehen kannst du jederzeit.** Exportier deinen kompletten Tresor, wann immer du willst. Deine Daten sind nie eingesperrt.

- **Button:** Kostenlos herunterladen (`#download`)

---

## pricing

- **Eyebrow:** Preise
- **Headline:** Die App ist kostenlos. Komplett.
- **Intro:** Jede Funktion, auf jedem Gerät, ohne Kosten. Du zahlst nur, wenn wir die Synchronisierung für dich betreiben sollen.

**Karte 1: Kostenlos**
- **Name:** Kostenlos
- **Preis:** 0
- **Unterzeile:** Alles, was Mosaic kann, synchronisiert über eine Cloud, die du schon hast.
- **Punkte:**
  - Apps für Windows, macOS, Linux, iPhone und Android
  - Browser-Erweiterung für Chrome, Edge und Firefox
  - Passwörter, Passkeys, 2FA-Codes, SSH-Schlüssel, Notizen und Dateien
  - Import aus deinem alten Passwort-Manager
  - Synchronisierung über deinen eigenen Ordner bei Dropbox, iCloud Drive, OneDrive oder Nextcloud
  - Geteilte Tresore in einem geteilten Ordner oder Teamordner
  - Die Kommandozeile `mosaic`
- **Button:** Kostenlos herunterladen

**Karte 2: Gehostet**
- **Name:** Gehostet
- **Preis:** {PRICE}
- **Unterzeile:** Wir kümmern uns um die Synchronisierung. Du richtest nichts ein.
- **Punkte:**
  - Alles aus Kostenlos
  - Synchronisierung über den Mosaic-Server, ohne Einrichtung
  - Notfallzugang mit einer Wartezeit, die du festlegst
  - Zweistufige Anmeldung für dein Sync-Konto
  - Dasselbe Schloss wie bei Kostenlos: Wir können deinen Tresor nicht lesen
  - Zurück zum Cloud-Ordner, wann immer du willst
- **Button:** Mit gehosteter Synchronisierung starten

[Design: `{PRICE}` enthält den Abrechnungszeitraum, z. B. „X € / Monat“. Beide Karten gleich breit, Gehostet bekommt den Akzentrahmen.]

- **Zeile unter den Karten:** Keine Werbung, kein Tracking, kein Datenverkauf, in keinem der beiden Pläne. Die gehostete Synchronisierung ist das, wovon wir leben.

---

## teams

- **Eyebrow:** Teams und Familie
- **Headline:** Hol die Leute dazu, denen du vertraust.

**Fließtext:**
Das WLAN-Passwort, der Streaming-Login, das gemeinsame Bankkonto, die Server-Schlüssel, die dein ganzes Team braucht. Pack sie in einen geteilten Tresor, und alle haben, was sie brauchen. Auf ihren eigenen Geräten, hinter ihrem eigenen Schloss.

**Punkte:**
- **Geteilte Tresore.** Einer für die Familie, einer pro Team oder Projekt. Alle sehen, was in ihrem liegt, und nicht mehr.
- **Prüf, mit wem du teilst.** Vergleich einmal einen kurzen Sicherheitscode mit der anderen Person. Ändert sich ihr Schlüssel je, sagt Mosaic dir Bescheid.
- **Saubere Abschiede.** Verlässt jemand das Team, nimmst du die Person raus, und der Tresor bekommt ein neues Schloss.
- **In der Cloud deiner Firma.** Team-Tresore liegen in einem Dropbox-Business-Teamordner, auf Nextcloud oder auf unserem Server. Keine neue Anbieter-Cloud, die erst abgesegnet werden muss.
- **Ein Weg rein, wenn es drauf ankommt.** Mit dem Notfallzugang kommt eine Vertrauensperson an den Tresor, wenn du es nicht kannst.

- **Button:** Mosaic fürs Team holen (`#download`)

[Visual-Vorschlag: zwei oder drei Geräte verschiedener Personen rund um eine gemeinsame Tresor-Karte.]

- **Alt-Text:** Drei Geräte zeigen denselben geteilten Tresor, jedes von seiner eigenen Besitzerin oder seinem eigenen Besitzer geöffnet.

---

## faq

- **Eyebrow:** Fragen
- **Headline:** Bevor du installierst.

1. **Könnt ihr meine Passwörter sehen?**
   Nein. Dein Tresor wird auf deinem Gerät verschlossen, bevor er irgendwohin geht, und nur deine zwei Schlüssel öffnen ihn. Nutzt du die gehostete Synchronisierung, sehen wir deine E-Mail-Adresse, deine Geräte und wie viele verschlüsselte Daten du speicherst, damit wir dein Konto betreiben können. Was drinsteht, sehen wir nie.

2. **Was, wenn ich mein Master-Passwort vergesse?**
   Dann kann es niemand zurücksetzen. Wir nicht, sonst auch niemand. Das ist der Preis für einen Tresor, den nur du öffnen kannst, und wir sagen es dir lieber jetzt als nach der Anmeldung. In deinem Emergency Kit gibt es eine Zeile, in die du dein Master-Passwort von Hand schreiben kannst. Heb es gut auf. Hast du einen Notfallzugang eingerichtet, kommt außerdem eine Vertrauensperson nach einer Wartezeit an deinen Tresor.

3. **Ist es sicher, alles an einem Ort zu haben?**
   Sicherer als die Alternative. Wiederverwendete Passwörter heißen: Ein Leck öffnet viele Konten. Mit Mosaic bekommt jedes Konto sein eigenes starkes Passwort, der Tresor ist mit zwei Schlüsseln verschlossen, die nur du hast, er sperrt sich selbst, wenn du weggehst, und Logins werden nur auf genau der Seite ausgefüllt, zu der sie gehören.

4. **Was passiert, wenn Mosaic gehackt wird?**
   Die Angreifer bekommen verschlossene Daten, die sie nicht öffnen können. Ohne dein Master-Passwort und deinen Secret Key gibt es nichts zu lesen, und auf unserem Server liegt nichts, woran sich geratene Passwörter testen ließen. Manipuliert jemand deine synchronisierten Daten, warnt dich die App.

5. **Was, wenn es Mosaic irgendwann nicht mehr gibt?**
   Dein Tresor liegt auf deinen Geräten und, wenn du über einen Cloud-Ordner synchronisierst, in deiner eigenen Cloud. Die App läuft auch ohne uns weiter. Du kannst jederzeit alles exportieren, und den Sync-Server kannst du auch auf deinem eigenen Rechner betreiben.

6. **Warum ist die App kostenlos?**
   Weil sie uns nichts kostet: Sie läuft auf deinen Geräten und deinem Speicher. Geld nehmen wir für das Einzige, was uns wirklich Geld kostet, den Betrieb der Synchronisierung. Keine Werbung, kein Datenverkauf, kein Tracking.

7. **Muss ich alle meine Passwörter ändern?**
   Nein. Importieren und weitermachen. Mosaic zeigt dir, welche schwach, doppelt oder geleakt sind, damit du die wichtigen zuerst angehen kannst.

8. **Läuft das auf meinem Handy und in meinem Browser?**
   Ja. Die Apps für iPhone und Android tragen Logins und Passkeys direkt in deinen Apps ein. Die Erweiterungen für Chrome, Edge und Firefox füllen im Web aus, immer erst nach deinem Klick.

9. **Was, wenn ich Laptop oder Handy verliere?**
   Wer es findet, braucht trotzdem dein Master-Passwort. Mosaic sperrt sich selbst, wenn du eine Weile nichts tust und wenn das Gerät in den Ruhezustand geht. Mit der gehosteten Synchronisierung kannst du das verlorene Gerät außerdem von jedem anderen aus abmelden.

10. **Funktioniert es auch offline?**
    Ja. Dein Tresor liegt auf deinem Gerät, also läuft alles ohne Internet. Änderungen werden synchronisiert, sobald du wieder online bist. Haben zwei Geräte dasselbe bearbeitet, behältst du beide Versionen.

[Design: Akkordeon, erste Frage offen.]

---

## download

- **Eyebrow:** Mosaic holen
- **Headline:** Ein Passwort merken. Um den Rest kümmert sich Mosaic.
- **Fließtext:** Lad Mosaic herunter, nimm deine alten Passwörter mit und lass die App das Merken übernehmen. Kostenlos auf jedem Gerät.
- **Primärer Button:** Kostenlos herunterladen
- **Sekundärer Button:** Pläne vergleichen (`#pricing`)

**Plattform-Buttons:**
- Windows
- macOS
- Linux
- iPhone
- Android
- Chrome
- Edge
- Firefox
- Kommandozeile

- **Zeile unter den Buttons:** Jeder Download ist signiert. Prüf ihn, bevor du installierst.
- **Link:** So prüfst du einen Download

[Design: System der Besucher erkennen und dessen Button zuerst und größer zeigen. Das Hero-Bild darf hier wiederkommen, auf den Himmel zugeschnitten.]

---

## footer

- **Logo:** Mosaic
- **Über-Zeile:** Mosaic ist ein Passwort-Manager, der deine Logins auf deinen Geräten hält, lesbar nur für dich. Keine Werbung. Kein Tracking.
- **Spalte „Produkt“:** Funktionen · So funktioniert's · Preise · Teams und Familie · FAQ
- **Spalte „Download“:** Windows · macOS · Linux · iPhone · Android · Browser-Erweiterungen · Kommandozeile
- **Spalte „Mehr“:** Vergleich · So prüfst du einen Download · Kontakt
- **Rechtliches:** Impressum · Datenschutz
- **Sprachwahl:** English · Deutsch
