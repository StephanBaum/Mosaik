/**
 * English UI strings: the copy reused from the v1 landing page (apps/web), used across the v2 pages. ICU MessageFormat, formatted at build
 * time only. Copy: marketing/landing-page.en.md, in page order, verbatim (typographic
 * quotes). "{name}" is the working product name and "{domain}" the placeholder domain;
 * neither is final. "{cmd}" marks the `mosaic` command, which the page sets as code.
 */
export const landing = {
  "web.brand.name": "Mosaic",
  "web.site.domain": "mosaic.example",

  // Header and navigation
  "web.skipLink": "Skip to content",
  "web.nav.label": "Main",
  "web.nav.home": "{name} home",
  "web.nav.features": "Features",
  "web.nav.pricing": "Pricing",
  "web.nav.teams": "Teams",
  "web.nav.cta": "Download free",
  "web.nav.open": "Open menu",
  "web.nav.close": "Close menu",
  "web.lang.label": "Language",
  "web.lang.en": "EN",
  "web.lang.de": "DE",
  "web.lang.enName": "English",
  "web.lang.deName": "Deutsch",

  // Hero
  "web.hero.eyebrow": "Password manager for every device",
  "web.hero.title1": "Every login.",
  "web.hero.title2": "One click.",
  "web.hero.title3": "Yours alone.",
  "web.hero.sub1": "Passwords, passkeys and 2FA codes in one place.",
  "web.hero.sub2": "Filled in with a click, on every device.",
  "web.hero.sub3": "Locked so that only you can read them.",
  "web.hero.ctaPrimary": "Download free",
  "web.hero.ctaSecondary": "See how it works",
  "web.hero.note": "Free for Windows, macOS, Linux, iPhone and Android.",

  // Where it lives
  "web.where.eyebrow": "Where your vault lives",
  "web.where.title": "Most password managers keep your vault on their servers.",
  "web.where.body":
    "That’s how they sync your devices. Your vault is encrypted, which is good. But it means one company holds millions of vaults in one place. That makes it a target worth years of effort.",
  "web.where.story1.label": "2022",
  "web.where.story1.title": "LastPass gets breached.",
  "web.where.story1.text":
    "Attackers broke into LastPass and walked out with copies of customers’ vaults. The passwords inside were encrypted. The website addresses were not. And a stolen vault never expires: the attackers can keep guessing master passwords offline, for as long as they like, and nobody will ever see them try.",
  "web.where.story2.label": "2026",
  "web.where.story2.title": "Researchers turn the server against its users.",
  "web.where.story2.text":
    "A team from ETH Zurich and USI showed that a hacked password-manager server can do more than sit on your data. It can actively attack the people using Bitwarden, LastPass and Dashlane, in the worst cases far enough to get at what’s in their vaults. Some of the problems have since been fixed. The lesson stands.",
  "web.where.closing":
    "None of this makes password managers a bad idea. They’re still far safer than reused passwords. It means two things matter more than anyone told you: where your vault lives, and whether your app checks what it gets back.",
  "web.where.sources":
    "Sources: LastPass security incident notice, December 2022. Scarlata et al., “Zero Knowledge (About) Encryption”, USENIX Security 2026.",

  // How it works
  "web.how.eyebrow": "How {name} works",
  "web.how.title": "Locked on your device. Kept where you choose. Checked every time.",
  "web.how.intro": "{name} is built so that your passwords stay yours, wherever they’re stored.",
  "web.how.step1.number": "01",
  "web.how.step1.title": "Two keys, both yours.",
  "web.how.step1.text":
    "Your vault opens with two keys. Your master password lives in your head. Your Secret Key is made by your device when you sign up and stays on your devices and on the Emergency Kit you print or save. Everything is locked before it leaves your device. A stolen copy opens nothing, even for someone who has guessed your master password.",
  "web.how.step2.number": "02",
  "web.how.step2.title": "You decide where it syncs.",
  "web.how.step2.text":
    "Keep your vault in your own Dropbox, iCloud Drive, OneDrive or Nextcloud folder, or in your company’s Dropbox Business team folder. Or let us run the sync for you. It’s the same lock either way. Your cloud stores scrambled files and can’t read a single entry.",
  "web.how.step3.number": "03",
  "web.how.step3.title": "It notices when something’s off.",
  "web.how.step3.text":
    "If a server or a cloud folder hands {name} an old copy of your vault, holds back your latest changes or serves a file that was tampered with, {name} doesn’t quietly go along. It warns you and keeps the warning up until you’ve looked.",
  "web.how.technicalLabel": "For the technical",
  "web.how.technical":
    "Each item is sealed on its own with XChaCha20-Poly1305 under keys derived with Argon2id from your master password and a 128-bit Secret Key. Signed checkpoints catch rollback, frozen updates and diverging copies on both sync options. Only published, standard building blocks.",
  "web.how.sample": "Sample",
  "web.how.warning": "The synced copy of your vault is older than one your devices have already seen.",
  "web.how.warningAlt":
    "{name} showing a warning that the synced copy of the vault is older than one the devices have already seen.",

  // Features
  "web.features.eyebrow": "Every day",
  "web.features.title": "Everything you log in with. Handled.",
  "web.features.intro": "One app for every password, passkey and code you have, on every screen you use.",
  "web.features.autofill.title": "Autofill that can’t be tricked.",
  "web.features.autofill.text":
    "Click, and {name} fills your login. Only on the exact site it belongs to, never in a hidden frame, never on a look-alike. On a fake page, nothing happens.",
  "web.features.passkeys.title": "Passkeys on every device.",
  "web.features.passkeys.text":
    "Create and use passkeys in your browser and on your phone. They sync to all your devices instead of getting stuck on one.",
  "web.features.codes.title": "2FA codes that fill themselves.",
  "web.features.codes.text":
    "Scan the QR code once. From then on, the code fills in right after your password. No more reaching for your phone.",
  "web.features.desktop.title": "A desktop app you’ll actually like.",
  "web.features.desktop.text":
    "Windows, macOS and Linux. Find anything with one shortcut. Open your vault with your fingerprint, your face or a security key.",
  "web.features.phone.title": "Your phone, fully covered.",
  "web.features.phone.text": "iPhone and Android, filling logins and passkeys right inside your apps.",
  "web.features.more.title": "More than passwords.",
  "web.features.more.text":
    "Secure notes, files, cards, Wi-Fi passwords, recovery codes, software licenses. Add your own fields to anything.",
  "web.features.generator.title": "Strong passwords, no effort.",
  "web.features.generator.text":
    "Make a strong password in one click. See which old ones are weak, reused or showed up in a known breach, and fix them first.",
  "web.features.sharing.title": "Share with the people you trust.",
  "web.features.sharing.text":
    "Share a login or a whole vault with family or coworkers. Remove someone, and the vault gets a new lock.",
  "web.features.emergency.title": "Emergency access.",
  "web.features.emergency.text":
    "Pick someone you trust. If something happens to you, they can ask for access and get it after a waiting period you set, unless you say no.",
  "web.features.ssh.title": "SSH keys, kept safe.",
  "web.features.ssh.text":
    "For developers: keep your SSH keys in the vault. The built-in SSH agent asks you before every use.",
  "web.features.terminal.title": "Right in your terminal.",
  "web.features.terminal.text": "The {cmd} command finds logins, makes passwords and pulls 2FA codes into your scripts.",
  "web.features.tracking.title": "No tracking. At all.",
  "web.features.tracking.text": "No analytics, no ads, no usage data. {name} doesn’t watch what you do.",
  "web.features.platformsLabel": "Platforms",
  "web.features.desktopAlt": "The {name} desktop app with a search box and a list of logins.",
  "web.how.syncAlt":
    "The sync settings of {name}: the sync server, the account, this device and the list of signed-in devices.",

  // Platforms (feature strip, download buttons, footer)
  "web.platform.windows": "Windows",
  "web.platform.macos": "macOS",
  "web.platform.linux": "Linux",
  "web.platform.iphone": "iPhone",
  "web.platform.android": "Android",
  "web.platform.chrome": "Chrome",
  "web.platform.edge": "Edge",
  "web.platform.firefox": "Firefox",
  "web.platform.cli": "Command line",
  "web.platform.extensions": "Browser extensions",

  // Only Mosaic
  "web.only.eyebrow": "Only {name}",
  "web.only.title": "What the others don’t do.",
  "web.only.intro":
    "We compared {name} with the best-known password managers. Three things only {name} does.",
  "web.only.cloud.title": "Your cloud or ours. Same lock.",
  "web.only.cloud.text":
    "Sync through your own cloud folder, your team’s Dropbox Business folder or our hosted server, and switch whenever you like. The others make you choose: their cloud only, or your own storage with no hosted option.",
  "web.only.notice.title": "It notices when a server tries to fool it.",
  "web.only.notice.text":
    "An old copy passed off as new, changes held back, a tampered file: {name} spots it and tells you. None of the others we compared describe anything like it.",
  "web.only.guess.title": "Our server can’t test password guesses.",
  "web.only.guess.text":
    "When you sign in to hosted sync, {name} proves it knows your password without handing the server anything a guess could be checked against. Not the password, not a hash of it. None of the others we compared sign you in this way.",
  "web.only.unlikeTitle": "And unlike most password managers:",
  "web.only.unlike1": "Nothing fills until you click. There’s no setting to change that.",
  "web.only.unlike2":
    "Logins fill only on the exact address you saved them for. A login for login.yourbank.com never fills on offers.yourbank.com.",
  "web.only.unlike3": "No telemetry, ever. Not on by default, not off by default. It doesn’t exist.",
  "web.only.technical":
    "1: one sync protocol over two transports (folder and hosted). 2: rollback and freeze detection on both, plus fork detection across devices on hosted sync. 3: OPAQUE sign-in; the others use SRP or send a client-side hash.",

  // Switching
  "web.switch.eyebrow": "Switching",
  "web.switch.title": "Moving in takes minutes.",
  "web.switch.bring.title": "Bring everything:",
  "web.switch.bring.text": "1Password, Bitwarden, LastPass, KeePass, Enpass, Chrome, or any spreadsheet export.",
  "web.switch.leave.title": "Leave whenever you want.",
  "web.switch.leave.text": "Export your whole vault at any time. Your data is never locked in.",

  // Pricing
  "web.pricing.eyebrow": "Pricing",
  "web.pricing.title": "The app is free. All of it.",
  "web.pricing.intro": "Every feature, on every device, at no cost. You pay only if you want us to run the sync.",
  "web.pricing.free.name": "Free",
  "web.pricing.free.price": "€0",
  "web.pricing.free.tagline": "Everything {name} does, synced through a cloud you already have.",
  "web.pricing.free.item1": "Apps for Windows, macOS, Linux, iPhone and Android",
  "web.pricing.free.item2": "Browser extension for Chrome, Edge and Firefox",
  "web.pricing.free.item3": "Passwords, passkeys, 2FA codes, SSH keys, notes and files",
  "web.pricing.free.item4": "Import from your old password manager",
  "web.pricing.free.item5": "Sync through your own Dropbox, iCloud Drive, OneDrive or Nextcloud folder",
  "web.pricing.free.item6": "Shared vaults in a shared or team folder",
  "web.pricing.free.item7": "The {cmd} command line",
  "web.pricing.free.cta": "Download free",
  "web.pricing.hosted.name": "Hosted",
  "web.pricing.hosted.price": "Price coming soon",
  "web.pricing.hosted.tagline": "We run the sync. You don’t set up a thing.",
  "web.pricing.hosted.item1": "Everything in Free",
  "web.pricing.hosted.item2": "Sync through the {name} server, nothing to configure",
  "web.pricing.hosted.item3": "Emergency access with a waiting period you set",
  "web.pricing.hosted.item4": "Two-step sign-in for your sync account",
  "web.pricing.hosted.item5": "Same lock as Free: we can’t read your vault",
  "web.pricing.hosted.item6": "Move back to a cloud folder whenever you like",
  "web.pricing.hosted.cta": "Start with hosted sync",
  "web.pricing.note": "No ads, no tracking and no data sales on either plan. Hosted sync is how we pay the bills.",

  // Teams and family
  "web.teams.eyebrow": "Teams and family",
  "web.teams.title": "Bring the people you trust.",
  "web.teams.body":
    "The Wi-Fi password, the streaming login, the bank account you share, the server keys your whole team needs. Put them in a shared vault and everyone has what they need, on their own devices, under their own lock.",
  "web.teams.shared.title": "Shared vaults.",
  "web.teams.shared.text": "One vault for the family, one per team or project. Everyone sees what’s in theirs, nothing more.",
  "web.teams.verify.title": "Check who you’re sharing with.",
  "web.teams.verify.text":
    "Compare a short safety code with the other person once. If their key ever changes, {name} tells you.",
  "web.teams.goodbye.title": "Clean goodbyes.",
  "web.teams.goodbye.text": "When someone leaves, remove them and the vault gets a new lock.",
  "web.teams.company.title": "In your company’s cloud.",
  "web.teams.company.text":
    "Keep team vaults in a Dropbox Business team folder or on Nextcloud, or on the hosted server. No new vendor cloud to approve.",
  "web.teams.emergency.title": "A way in when it counts.",
  "web.teams.emergency.text": "Emergency access lets a trusted person reach the vault if you can’t.",
  "web.teams.cta": "Get {name} for your team",

  // FAQ
  "web.faq.eyebrow": "Questions",
  "web.faq.title": "Before you install.",
  "web.faq.q1": "Can you see my passwords?",
  "web.faq.a1":
    "No. Your vault is locked on your device before it goes anywhere, and only your two keys open it. If you use hosted sync, we see your email address, your devices and how much locked data you store, so we can run your account. Never what’s inside.",
  "web.faq.q2": "What if I forget my master password?",
  "web.faq.a2":
    "Nobody can reset it. Not us, not anyone. That’s the price of a vault only you can open, and we’d rather say it now than after you sign up. Your Emergency Kit has a line to write your master password down by hand; keep it somewhere safe. If you set up emergency access, a person you trust can also reach your vault after a waiting period.",
  "web.faq.q3": "Is it safe to keep everything in one place?",
  "web.faq.a3":
    "Safer than the alternative. Reused passwords mean one leak opens many accounts. With {name} every account gets its own strong password, the vault is locked with two keys only you hold, it locks itself when you step away, and logins fill only on the exact site they belong to.",
  "web.faq.q4": "What happens if {name} gets hacked?",
  "web.faq.a4":
    "The attacker gets locked data they can’t open. Without your master password and your Secret Key there’s nothing to read, and our server holds nothing to test password guesses against. If someone tampers with your synced data, the app warns you.",
  "web.faq.q5": "What if {name} shuts down?",
  "web.faq.a5":
    "Your vault lives on your devices, and if you sync through a cloud folder, in your own cloud. The app keeps working without us. You can export everything at any time, and the sync server can run on your own machine.",
  "web.faq.q6": "Why is the app free?",
  "web.faq.a6":
    "Because running it costs us nothing: it lives on your devices and your storage. We charge for the one thing that does cost money, hosting the sync. No ads, no data sales, no tracking.",
  "web.faq.q7": "Do I have to change all my passwords?",
  "web.faq.a7":
    "No. Import them and keep going. {name} shows you which ones are weak, reused or leaked, so you can fix the important ones first.",
  "web.faq.q8": "Does it work on my phone and in my browser?",
  "web.faq.a8":
    "Yes. Apps for iPhone and Android fill logins and passkeys inside your apps. Extensions for Chrome, Edge and Firefox fill on the web, always after your click.",
  "web.faq.q9": "What if I lose my laptop or phone?",
  "web.faq.a9":
    "Whoever finds it still needs your master password. {name} locks itself when you’re idle and when the device goes to sleep. With hosted sync, you can also sign the lost device out from any other one.",
  "web.faq.q10": "Does it work offline?",
  "web.faq.a10":
    "Yes. Your vault is on your device, so everything works without internet. Changes sync the next time you’re online, and if two devices edited the same thing, you keep both versions.",

  // Download
  "web.download.eyebrow": "Get {name}",
  "web.download.title": "One password to remember. Everything else, handled.",
  "web.download.body":
    "Download {name}, bring over your old passwords and let the app do the remembering. Free on every device.",
  "web.download.ctaPrimary": "Download free",

  // Footer
  "web.footer.label": "Footer",
  "web.footer.about":
    "{name} is a password manager that keeps your logins on your devices, readable only by you. No ads. No tracking.",

} as const satisfies Record<string, string>;
