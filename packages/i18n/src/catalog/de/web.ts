/**
 * German UI strings: the landing page (apps/web). Informal "du". Same keys as English
 * (checked by the type). Copy: marketing/landing-page.de.md, in page order, verbatim.
 */
import type { web as en } from "../en/web.ts";

export const web = {
  "web.brand.name": "Mosaic",
  "web.site.domain": "mosaic.example",
  "web.meta.title": "{name}: Jeder Login. Ein Klick. Nur für dich.",
  "web.meta.description":
    "{name} ist ein Passwort-Manager, der deine Logins auf deinen Geräten hält, lesbar nur für dich. Keine Werbung. Kein Tracking.",

  // Header and navigation
  "web.skipLink": "Zum Inhalt springen",
  "web.nav.label": "Hauptnavigation",
  "web.nav.home": "{name} Startseite",
  "web.nav.howItWorks": "So funktioniert’s",
  "web.nav.features": "Funktionen",
  "web.nav.pricing": "Preise",
  "web.nav.teams": "Teams",
  "web.nav.faq": "FAQ",
  "web.nav.cta": "Kostenlos laden",
  "web.nav.open": "Menü öffnen",
  "web.nav.close": "Menü schließen",
  "web.lang.label": "Sprache",
  "web.lang.en": "EN",
  "web.lang.de": "DE",
  "web.lang.enName": "English",
  "web.lang.deName": "Deutsch",

  // Hero
  "web.hero.eyebrow": "Passwort-Manager für alle deine Geräte",
  "web.hero.title1": "Jeder Login.",
  "web.hero.title2": "Ein Klick.",
  "web.hero.title3": "Nur für dich.",
  "web.hero.sub1": "Passwörter, Passkeys und 2FA-Codes an einem Ort.",
  "web.hero.sub2": "Mit einem Klick ausgefüllt, auf jedem Gerät.",
  "web.hero.sub3": "So verschlossen, dass nur du sie lesen kannst.",
  "web.hero.ctaPrimary": "Kostenlos herunterladen",
  "web.hero.ctaSecondary": "So funktioniert’s",
  "web.hero.note": "Kostenlos für Windows, macOS, Linux, iPhone und Android.",
  "web.hero.imageAlt":
    "Eine einsame Scheune auf einem offenen Feld in der Abenddämmerung, das Gras biegt sich im Wind, gemalt in dunklem Acryl und in Streifen sortierter Pixel aufgelöst.",

  // Problem
  "web.problem.eyebrow": "Der ganz normale Wahnsinn",
  "web.problem.title": "Zu viele Passwörter. Zu wenig Kopf dafür.",
  "web.problem.body":
    "Du hast mehr Konten, als sich irgendjemand merken kann. Also machst du, was alle machen: ein paar Lieblingspasswörter überall, ein paar auf einem Zettel, und beim Rest klickst du auf „Passwort vergessen“. Klappt prima. Bis es das nicht mehr tut.",
  "web.problem.leak.title": "Ein Leck, alle Türen offen.",
  "web.problem.leak.text":
    "Eine Seite, die du mal kurz benutzt hast, wird gehackt. Die Angreifer probieren deine E-Mail und dein Passwort überall sonst aus. Und wo du es wiederverwendet hast, sind sie drin.",
  "web.problem.fake.title": "Fälschungen sehen echt aus.",
  "web.problem.fake.text":
    "Ein Buchstabe anders in der Adresse, dasselbe Logo, dasselbe Anmeldefeld. Du tippst dein Passwort ein, bevor es dir auffällt.",
  "web.problem.codes.title": "Codes, Codes, Codes.",
  "web.problem.codes.text":
    "Jeder wichtige Login will zusätzlich einen sechsstelligen Code vom Handy. Handy raus, App suchen, schnell abtippen, bevor er abläuft.",
  "web.problem.passkeys.title": "Passkeys bleiben hängen.",
  "web.problem.passkeys.text":
    "Du legst auf dem Handy einen Passkey an, setzt dich an den Laptop, und da ist er nicht.",
  "web.problem.closing":
    "Ein guter Passwort-Manager löst das alles. Die eigentliche Frage ist, welchem du alles anvertraust.",

  // Where it lives
  "web.where.eyebrow": "Wo dein Tresor liegt",
  "web.where.title": "Die meisten Passwort-Manager lagern deinen Tresor auf ihren Servern.",
  "web.where.body":
    "So synchronisieren sie deine Geräte. Dein Tresor ist verschlüsselt, das ist gut. Aber es heißt auch: Eine Firma hat Millionen Tresore an einem Ort. Für Angreifer ist das ein Ziel, für das sich jahrelange Arbeit lohnt.",
  "web.where.imageAlt":
    "Dieselbe Scheune bei Nacht unter schwerem Sturmhimmel, ein Fenster erleuchtet, der obere Himmel in Streifen sortierter Pixel aufgelöst.",
  "web.where.story1.label": "2022",
  "web.where.story1.title": "LastPass wird gehackt.",
  "web.where.story1.text":
    "Angreifer brachen bei LastPass ein und nahmen Kopien der Tresore von Kunden mit. Die Passwörter darin waren verschlüsselt, die Website-Adressen nicht. Und ein gestohlener Tresor läuft nie ab: Die Angreifer können offline Master-Passwörter durchprobieren, so lange sie wollen, und niemand bekommt es je mit.",
  "web.where.story2.label": "2026",
  "web.where.story2.title": "Forscher drehen den Server gegen seine Nutzer.",
  "web.where.story2.text":
    "Ein Team der ETH Zürich und der USI hat gezeigt: Ein gehackter Server eines Passwort-Managers kann mehr, als nur auf deinen Daten zu sitzen. Er kann die Leute, die Bitwarden, LastPass und Dashlane nutzen, aktiv angreifen, im schlimmsten Fall so weit, dass er an den Inhalt ihrer Tresore kommt. Manches davon ist inzwischen behoben. Die Lehre bleibt.",
  "web.where.closing":
    "Das heißt nicht, dass Passwort-Manager eine schlechte Idee sind. Sie sind immer noch viel sicherer als wiederverwendete Passwörter. Es heißt nur: Zwei Dinge zählen mehr, als dir bisher jemand gesagt hat. Wo dein Tresor liegt. Und ob deine App prüft, was sie zurückbekommt.",
  "web.where.sources":
    "Quellen: LastPass, Hinweis zum Sicherheitsvorfall, Dezember 2022. Scarlata et al., „Zero Knowledge (About) Encryption“, USENIX Security 2026.",

  // How it works
  "web.how.eyebrow": "So funktioniert {name}",
  "web.how.title": "Auf deinem Gerät verschlossen. Gelagert, wo du willst. Jedes Mal geprüft.",
  "web.how.intro": "{name} ist so gebaut, dass deine Passwörter dir gehören, egal wo sie liegen.",
  "web.how.step1.number": "01",
  "web.how.step1.title": "Zwei Schlüssel, beide in deiner Hand.",
  "web.how.step1.text":
    "Dein Tresor öffnet sich nur mit zwei Schlüsseln. Dein Master-Passwort hast du im Kopf. Deinen Secret Key erzeugt dein Gerät bei der Einrichtung. Er bleibt auf deinen Geräten und in deinem Emergency Kit, das du ausdruckst oder speicherst. Alles wird verschlossen, bevor es dein Gerät verlässt. Eine gestohlene Kopie öffnet gar nichts, selbst wenn jemand dein Master-Passwort errät.",
  "web.how.step2.number": "02",
  "web.how.step2.title": "Du entscheidest, wo synchronisiert wird.",
  "web.how.step2.text":
    "Leg deinen Tresor in deinen eigenen Ordner bei Dropbox, iCloud Drive, OneDrive oder Nextcloud, oder in den Teamordner deiner Firma bei Dropbox Business. Oder wir übernehmen die Synchronisierung für dich. Das Schloss ist in beiden Fällen dasselbe. Deine Cloud speichert verschlüsselte Dateien und kann keinen einzigen Eintrag lesen.",
  "web.how.step3.number": "03",
  "web.how.step3.title": "Es merkt, wenn etwas nicht stimmt.",
  "web.how.step3.text":
    "Wenn ein Server oder ein Cloud-Ordner {name} eine alte Kopie deines Tresors unterjubelt, deine neuesten Änderungen zurückhält oder eine manipulierte Datei liefert, spielt {name} nicht still mit. Es warnt dich, und die Warnung bleibt, bis du sie dir angesehen hast.",
  "web.how.technicalLabel": "Für Technikinteressierte",
  "web.how.technical":
    "Jeder Eintrag wird einzeln mit XChaCha20-Poly1305 versiegelt, unter Schlüsseln, die per Argon2id aus deinem Master-Passwort und einem 128-Bit-Secret-Key abgeleitet sind. Signierte Checkpoints erkennen Rollbacks, eingefrorene Updates und auseinanderlaufende Kopien bei beiden Sync-Varianten. Nur veröffentlichte Standardbausteine.",
  "web.how.sample": "Beispiel",
  "web.how.warning": "Die synchronisierte Kopie deines Tresors ist älter als eine, die deine Geräte schon kennen.",
  "web.how.warningAlt":
    "{name} zeigt eine Warnung: Die synchronisierte Kopie des Tresors ist älter als eine, die deine Geräte schon kennen.",

  // Features
  "web.features.eyebrow": "Jeden Tag",
  "web.features.title": "Alles, womit du dich anmeldest. Erledigt.",
  "web.features.intro":
    "Eine App für jedes Passwort, jeden Passkey und jeden Code, auf jedem Bildschirm, den du benutzt.",
  "web.features.autofill.title": "Autofill, das sich nicht reinlegen lässt.",
  "web.features.autofill.text":
    "Ein Klick, und {name} füllt deinen Login aus. Nur auf genau der Seite, zu der er gehört, nie in einem versteckten Rahmen, nie auf einer Doppelgänger-Seite. Auf einer Fälschung passiert einfach nichts.",
  "web.features.passkeys.title": "Passkeys auf allen Geräten.",
  "web.features.passkeys.text":
    "Leg Passkeys im Browser und auf dem Handy an und nutze sie dort. Sie landen auf all deinen Geräten, statt auf einem festzustecken.",
  "web.features.codes.title": "2FA-Codes, die sich selbst eintragen.",
  "web.features.codes.text":
    "Einmal den QR-Code scannen. Ab dann steht der Code direkt nach deinem Passwort im Feld. Kein Griff mehr zum Handy.",
  "web.features.desktop.title": "Eine Desktop-App, die Spaß macht.",
  "web.features.desktop.text":
    "Windows, macOS und Linux. Mit einem Tastenkürzel findest du alles. Dein Tresor öffnet sich per Fingerabdruck, Gesicht oder Sicherheitsschlüssel.",
  "web.features.phone.title": "Dein Handy ist voll dabei.",
  "web.features.phone.text": "iPhone und Android tragen Logins und Passkeys direkt in deinen Apps ein.",
  "web.features.more.title": "Mehr als Passwörter.",
  "web.features.more.text":
    "Sichere Notizen, Dateien, Karten, WLAN-Passwörter, Wiederherstellungscodes, Software-Lizenzen. Und überall eigene Felder, wenn du sie brauchst.",
  "web.features.generator.title": "Starke Passwörter ohne Aufwand.",
  "web.features.generator.text":
    "Ein Klick, und du hast ein starkes Passwort. Mosaic zeigt dir, welche alten schwach, doppelt oder in einem bekannten Datenleck aufgetaucht sind. Die wichtigsten zuerst.",
  "web.features.sharing.title": "Teilen mit Leuten, denen du vertraust.",
  "web.features.sharing.text":
    "Teil einen Login oder einen ganzen Tresor mit Familie oder Kollegen. Nimmst du jemanden raus, bekommt der Tresor ein neues Schloss.",
  "web.features.emergency.title": "Notfallzugang.",
  "web.features.emergency.text":
    "Bestimm eine Person deines Vertrauens. Falls dir etwas passiert, kann sie Zugang anfragen und bekommt ihn nach einer Wartezeit, die du festlegst. Außer du sagst Nein.",
  "web.features.ssh.title": "SSH-Schlüssel, gut verwahrt.",
  "web.features.ssh.text":
    "Für Entwickler: Deine SSH-Schlüssel liegen im Tresor. Der eingebaute SSH-Agent fragt dich vor jeder Nutzung.",
  "web.features.terminal.title": "Direkt im Terminal.",
  "web.features.terminal.text":
    "Der Befehl {cmd} findet Logins, erzeugt Passwörter und holt 2FA-Codes in deine Skripte.",
  "web.features.tracking.title": "Kein Tracking. Gar keins.",
  "web.features.tracking.text":
    "Keine Analyse-Tools, keine Werbung, keine Nutzungsdaten. {name} schaut dir nicht über die Schulter.",
  "web.features.platformsLabel": "Plattformen",
  "web.features.desktopAlt": "Die Desktop-App von {name} mit Suchfeld und einer Liste von Logins.",
  "web.features.generatorAlt":
    "Der Passwort-Generator von {name} mit einem starken Passwort aus 20 Zeichen und seinen Optionen.",
  "web.how.syncAlt":
    "Die Sync-Einstellungen von {name}: Server, Konto, dieses Gerät und die Liste der angemeldeten Geräte.",

  // Platforms (feature strip, download buttons, footer)
  "web.platform.windows": "Windows",
  "web.platform.macos": "macOS",
  "web.platform.linux": "Linux",
  "web.platform.iphone": "iPhone",
  "web.platform.android": "Android",
  "web.platform.chrome": "Chrome",
  "web.platform.edge": "Edge",
  "web.platform.firefox": "Firefox",
  "web.platform.cli": "Kommandozeile",
  "web.platform.extensions": "Browser-Erweiterungen",

  // Only Mosaic
  "web.only.eyebrow": "Nur bei {name}",
  "web.only.title": "Was die anderen nicht können.",
  "web.only.intro":
    "Wir haben {name} mit den bekanntesten Passwort-Managern verglichen. Drei Dinge kann nur {name}.",
  "web.only.cloud.title": "Deine Cloud oder unsere. Dasselbe Schloss.",
  "web.only.cloud.text":
    "Synchronisiere über deinen eigenen Cloud-Ordner, den Dropbox-Business-Ordner deines Teams oder unseren Server, und wechsle, wann du willst. Bei den anderen musst du dich entscheiden: nur ihre Cloud, oder dein eigener Speicher ohne gehostete Option.",
  "web.only.notice.title": "Es merkt, wenn ein Server es austricksen will.",
  "web.only.notice.text":
    "Eine alte Kopie, die als neu verkauft wird, zurückgehaltene Änderungen, eine manipulierte Datei: {name} erkennt das und sagt es dir. Keiner der anderen, die wir verglichen haben, beschreibt etwas Vergleichbares.",
  "web.only.guess.title": "Unser Server kann keine Passwörter durchprobieren.",
  "web.only.guess.text":
    "Wenn du dich bei der gehosteten Synchronisierung anmeldest, beweist {name}, dass es dein Passwort kennt, ohne dem Server irgendetwas zu geben, woran er geratene Passwörter testen könnte. Nicht das Passwort, nicht seinen Hash. Keiner der anderen, die wir verglichen haben, meldet dich so an.",
  "web.only.unlikeTitle": "Und anders als die meisten Passwort-Manager:",
  "web.only.unlike1": "Ohne deinen Klick wird nichts ausgefüllt. Und es gibt keine Einstellung, die das ändert.",
  "web.only.unlike2":
    "Logins werden nur auf genau der Adresse ausgefüllt, für die du sie gespeichert hast. Ein Login für login.deinebank.de landet nie auf angebote.deinebank.de.",
  "web.only.unlike3":
    "Keine Telemetrie, niemals. Nicht standardmäßig an, nicht standardmäßig aus. Es gibt sie schlicht nicht.",
  "web.only.technical":
    "1: ein Sync-Protokoll über zwei Transportwege (Ordner und gehostet). 2: Erkennung von Rollback und Freeze bei beiden, dazu Fork-Erkennung über mehrere Geräte bei der gehosteten Synchronisierung. 3: Anmeldung per OPAQUE; die anderen nutzen SRP oder schicken einen clientseitigen Hash.",

  // Switching
  "web.switch.eyebrow": "Umsteigen",
  "web.switch.title": "In ein paar Minuten bist du eingezogen.",
  "web.switch.body":
    "Exportier deinen Tresor aus deiner alten App, öffne die Datei in {name} und schau dir die Vorschau an. Doppelte Einträge fallen auf, bevor irgendetwas gespeichert wird. Deine Passwörter funktionieren genau wie vorher, du musst nichts zurücksetzen.",
  "web.switch.bring.title": "Bring alles mit:",
  "web.switch.bring.text": "1Password, Bitwarden, LastPass, KeePass, Enpass, Chrome oder jeder Tabellen-Export.",
  "web.switch.passkeys.title": "Passkeys ziehen mit um,",
  "web.switch.passkeys.text": "aus Apps, die das neue Austauschformat für Passkeys unterstützen.",
  "web.switch.leave.title": "Gehen kannst du jederzeit.",
  "web.switch.leave.text": "Exportier deinen kompletten Tresor, wann immer du willst. Deine Daten sind nie eingesperrt.",
  "web.switch.cta": "Kostenlos herunterladen",

  // Pricing
  "web.pricing.eyebrow": "Preise",
  "web.pricing.title": "Die App ist kostenlos. Komplett.",
  "web.pricing.intro":
    "Jede Funktion, auf jedem Gerät, ohne Kosten. Du zahlst nur, wenn wir die Synchronisierung für dich betreiben sollen.",
  "web.pricing.free.name": "Kostenlos",
  "web.pricing.free.price": "0 €",
  "web.pricing.free.tagline": "Alles, was {name} kann, synchronisiert über eine Cloud, die du schon hast.",
  "web.pricing.free.item1": "Apps für Windows, macOS, Linux, iPhone und Android",
  "web.pricing.free.item2": "Browser-Erweiterung für Chrome, Edge und Firefox",
  "web.pricing.free.item3": "Passwörter, Passkeys, 2FA-Codes, SSH-Schlüssel, Notizen und Dateien",
  "web.pricing.free.item4": "Import aus deinem alten Passwort-Manager",
  "web.pricing.free.item5": "Synchronisierung über deinen eigenen Ordner bei Dropbox, iCloud Drive, OneDrive oder Nextcloud",
  "web.pricing.free.item6": "Geteilte Tresore in einem geteilten Ordner oder Teamordner",
  "web.pricing.free.item7": "Die Kommandozeile {cmd}",
  "web.pricing.free.cta": "Kostenlos herunterladen",
  "web.pricing.hosted.name": "Gehostet",
  "web.pricing.hosted.price": "Preis folgt",
  "web.pricing.hosted.tagline": "Wir kümmern uns um die Synchronisierung. Du richtest nichts ein.",
  "web.pricing.hosted.item1": "Alles aus Kostenlos",
  "web.pricing.hosted.item2": "Synchronisierung über den {name}-Server, ohne Einrichtung",
  "web.pricing.hosted.item3": "Notfallzugang mit einer Wartezeit, die du festlegst",
  "web.pricing.hosted.item4": "Zweistufige Anmeldung für dein Sync-Konto",
  "web.pricing.hosted.item5": "Dasselbe Schloss wie bei Kostenlos: Wir können deinen Tresor nicht lesen",
  "web.pricing.hosted.item6": "Zurück zum Cloud-Ordner, wann immer du willst",
  "web.pricing.hosted.cta": "Mit gehosteter Synchronisierung starten",
  "web.pricing.note":
    "Keine Werbung, kein Tracking, kein Datenverkauf, in keinem der beiden Pläne. Die gehostete Synchronisierung ist das, wovon wir leben.",

  // Teams and family
  "web.teams.eyebrow": "Teams und Familie",
  "web.teams.title": "Hol die Leute dazu, denen du vertraust.",
  "web.teams.body":
    "Das WLAN-Passwort, der Streaming-Login, das gemeinsame Bankkonto, die Server-Schlüssel, die dein ganzes Team braucht. Pack sie in einen geteilten Tresor, und alle haben, was sie brauchen. Auf ihren eigenen Geräten, hinter ihrem eigenen Schloss.",
  "web.teams.shared.title": "Geteilte Tresore.",
  "web.teams.shared.text":
    "Einer für die Familie, einer pro Team oder Projekt. Alle sehen, was in ihrem liegt, und nicht mehr.",
  "web.teams.verify.title": "Prüf, mit wem du teilst.",
  "web.teams.verify.text":
    "Vergleich einmal einen kurzen Sicherheitscode mit der anderen Person. Ändert sich ihr Schlüssel je, sagt {name} dir Bescheid.",
  "web.teams.goodbye.title": "Saubere Abschiede.",
  "web.teams.goodbye.text":
    "Verlässt jemand das Team, nimmst du die Person raus, und der Tresor bekommt ein neues Schloss.",
  "web.teams.company.title": "In der Cloud deiner Firma.",
  "web.teams.company.text":
    "Team-Tresore liegen in einem Dropbox-Business-Teamordner, auf Nextcloud oder auf unserem Server. Keine neue Anbieter-Cloud, die erst abgesegnet werden muss.",
  "web.teams.emergency.title": "Ein Weg rein, wenn es drauf ankommt.",
  "web.teams.emergency.text":
    "Mit dem Notfallzugang kommt eine Vertrauensperson an den Tresor, wenn du es nicht kannst.",
  "web.teams.cta": "{name} fürs Team holen",

  // FAQ
  "web.faq.eyebrow": "Fragen",
  "web.faq.title": "Bevor du installierst.",
  "web.faq.q1": "Könnt ihr meine Passwörter sehen?",
  "web.faq.a1":
    "Nein. Dein Tresor wird auf deinem Gerät verschlossen, bevor er irgendwohin geht, und nur deine zwei Schlüssel öffnen ihn. Nutzt du die gehostete Synchronisierung, sehen wir deine E-Mail-Adresse, deine Geräte und wie viele verschlüsselte Daten du speicherst, damit wir dein Konto betreiben können. Was drinsteht, sehen wir nie.",
  "web.faq.q2": "Was, wenn ich mein Master-Passwort vergesse?",
  "web.faq.a2":
    "Dann kann es niemand zurücksetzen. Wir nicht, sonst auch niemand. Das ist der Preis für einen Tresor, den nur du öffnen kannst, und wir sagen es dir lieber jetzt als nach der Anmeldung. In deinem Emergency Kit gibt es eine Zeile, in die du dein Master-Passwort von Hand schreiben kannst. Heb es gut auf. Hast du einen Notfallzugang eingerichtet, kommt außerdem eine Vertrauensperson nach einer Wartezeit an deinen Tresor.",
  "web.faq.q3": "Ist es sicher, alles an einem Ort zu haben?",
  "web.faq.a3":
    "Sicherer als die Alternative. Wiederverwendete Passwörter heißen: Ein Leck öffnet viele Konten. Mit {name} bekommt jedes Konto sein eigenes starkes Passwort, der Tresor ist mit zwei Schlüsseln verschlossen, die nur du hast, er sperrt sich selbst, wenn du weggehst, und Logins werden nur auf genau der Seite ausgefüllt, zu der sie gehören.",
  "web.faq.q4": "Was passiert, wenn {name} gehackt wird?",
  "web.faq.a4":
    "Die Angreifer bekommen verschlossene Daten, die sie nicht öffnen können. Ohne dein Master-Passwort und deinen Secret Key gibt es nichts zu lesen, und auf unserem Server liegt nichts, woran sich geratene Passwörter testen ließen. Manipuliert jemand deine synchronisierten Daten, warnt dich die App.",
  "web.faq.q5": "Was, wenn es {name} irgendwann nicht mehr gibt?",
  "web.faq.a5":
    "Dein Tresor liegt auf deinen Geräten und, wenn du über einen Cloud-Ordner synchronisierst, in deiner eigenen Cloud. Die App läuft auch ohne uns weiter. Du kannst jederzeit alles exportieren, und den Sync-Server kannst du auch auf deinem eigenen Rechner betreiben.",
  "web.faq.q6": "Warum ist die App kostenlos?",
  "web.faq.a6":
    "Weil sie uns nichts kostet: Sie läuft auf deinen Geräten und deinem Speicher. Geld nehmen wir für das Einzige, was uns wirklich Geld kostet, den Betrieb der Synchronisierung. Keine Werbung, kein Datenverkauf, kein Tracking.",
  "web.faq.q7": "Muss ich alle meine Passwörter ändern?",
  "web.faq.a7":
    "Nein. Importieren und weitermachen. {name} zeigt dir, welche schwach, doppelt oder geleakt sind, damit du die wichtigen zuerst angehen kannst.",
  "web.faq.q8": "Läuft das auf meinem Handy und in meinem Browser?",
  "web.faq.a8":
    "Ja. Die Apps für iPhone und Android tragen Logins und Passkeys direkt in deinen Apps ein. Die Erweiterungen für Chrome, Edge und Firefox füllen im Web aus, immer erst nach deinem Klick.",
  "web.faq.q9": "Was, wenn ich Laptop oder Handy verliere?",
  "web.faq.a9":
    "Wer es findet, braucht trotzdem dein Master-Passwort. {name} sperrt sich selbst, wenn du eine Weile nichts tust und wenn das Gerät in den Ruhezustand geht. Mit der gehosteten Synchronisierung kannst du das verlorene Gerät außerdem von jedem anderen aus abmelden.",
  "web.faq.q10": "Funktioniert es auch offline?",
  "web.faq.a10":
    "Ja. Dein Tresor liegt auf deinem Gerät, also läuft alles ohne Internet. Änderungen werden synchronisiert, sobald du wieder online bist. Haben zwei Geräte dasselbe bearbeitet, behältst du beide Versionen.",

  // Download
  "web.download.eyebrow": "{name} holen",
  "web.download.title": "Ein Passwort merken. Um den Rest kümmert sich Mosaic.",
  "web.download.body":
    "Lad {name} herunter, nimm deine alten Passwörter mit und lass die App das Merken übernehmen. Kostenlos auf jedem Gerät.",
  "web.download.ctaPrimary": "Kostenlos herunterladen",
  "web.download.ctaSecondary": "Pläne vergleichen",
  "web.download.platformsLabel": "Downloads",
  "web.download.note": "Jeder Download ist signiert. Prüf ihn, bevor du installierst.",

  // Footer
  "web.footer.label": "Fußzeile",
  "web.footer.about":
    "{name} ist ein Passwort-Manager, der deine Logins auf deinen Geräten hält, lesbar nur für dich. Keine Werbung. Kein Tracking.",
  "web.footer.product": "Produkt",
  "web.footer.download": "Download",
  "web.footer.more": "Mehr",
  "web.footer.teams": "Teams und Familie",
  "web.footer.compare": "Vergleich",
  "web.footer.verify": "So prüfst du einen Download",

  // 404
  "web.notFound.title": "Unter dieser Adresse wohnt nichts.",
  "web.notFound.body": "Die Seite, die du suchst, gibt es hier nicht.",
  "web.notFound.back": "Zurück zum Anfang",
} satisfies Record<keyof typeof en, string>;
