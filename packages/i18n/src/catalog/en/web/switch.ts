/**
 * English strings of the Switch guide (website v2 §5.7). Menu paths follow each vendor's help
 * pages as of 2026-11; the CSV line matches crates/import (it reads the common column names).
 */
export const switchGuide = {
  "web.switchPage.sub":
    "Export from your old app, open the file in {name}, check the preview. Your passwords keep working, so there’s nothing to reset.",
  "web.switchPage.step1.title": "Export.",
  "web.switchPage.step1.text": "Save your vault as a file from your old app, on your own computer.",
  "web.switchPage.step2.title": "Import.",
  "web.switchPage.step2.text":
    "Open the file in {name}. You see everything before anything is saved, with duplicates marked.",
  "web.switchPage.step3.title": "Clean up.",
  "web.switchPage.step3.text": "Delete the export file and empty the trash. It isn’t encrypted.",
  "web.switchPage.pick": "Pick the app you’re leaving.",
  "web.switchPage.src.1password": "1Password",
  "web.switchPage.src.1password.text":
    "In the 1Password desktop app, choose File, then Export, and pick your account. Save it as a 1PUX file. In {name}, choose Import and open that file.",
  "web.switchPage.src.bitwarden": "Bitwarden",
  "web.switchPage.src.bitwarden.text":
    "In Bitwarden, open Export vault and choose the .json format. In {name}, choose Import and open the file. Passkeys come along.",
  "web.switchPage.src.lastpass": "LastPass",
  "web.switchPage.src.lastpass.text":
    "In LastPass, open Advanced options, then Export, and save the CSV file. In {name}, choose Import, pick CSV and open the file.",
  "web.switchPage.src.keepass": "KeePass and KeePassXC",
  "web.switchPage.src.keepass.text":
    "No export needed. In {name}, choose Import and open your .kdbx database (KDBX 4). You’ll be asked for its master password once.",
  "web.switchPage.src.enpass": "Enpass",
  "web.switchPage.src.enpass.text":
    "In Enpass, choose File, then Export, and save as .json. In {name}, choose Import and open the file.",
  "web.switchPage.src.browsers": "Chrome and other browsers",
  "web.switchPage.src.browsers.text":
    "In Chrome, open Google Password Manager, then Settings, then Export passwords, and save the CSV file. Edge and Firefox have the same option under their password settings. In {name}, choose Import, pick CSV and open the file.",
  "web.switchPage.src.other": "Anything else",
  "web.switchPage.src.other.text":
    "Most apps can export a CSV spreadsheet. {name} reads the common column names.",
  "web.switchPage.safety.title": "The one risky minute.",
  "web.switchPage.safety.text":
    "An export file holds every password in plain text. Make it on your own computer, import it right away, then delete it and empty the trash. Don’t email it to yourself and don’t put it in a cloud folder.",
  "web.switchPage.passkeys.title": "Passkeys come too.",
  "web.switchPage.passkeys.text":
    "From apps that support the new passkey exchange format, and from Bitwarden and KeePassXC exports. Others keep passkeys locked to their app; for those, add a new passkey on the site after you switch.",
  "web.switchPage.checklist.title": "Your checklist.",
  "web.switchPage.checklist.c1": "Install {name} and create your vault.",
  "web.switchPage.checklist.c2": "Print or save your Emergency Kit and write your master password on it by hand.",
  "web.switchPage.checklist.c3": "Export from your old app.",
  "web.switchPage.checklist.c4": "Import into {name} and check the preview.",
  "web.switchPage.checklist.c5": "Delete the export file and empty the trash.",
  "web.switchPage.checklist.c6": "Add the browser extension and sign in to one site with a click.",
  "web.switchPage.checklist.c7": "Choose where to sync: your cloud folder or hosted.",
  "web.switchPage.checklist.c8": "Once everything works, delete your old account.",
  "web.switchPage.checklist.print": "Print checklist",
  "web.switchPage.faq.title": "Questions about switching",
  "web.switchPage.faq.q1": "Do I have to change my passwords?",
  "web.switchPage.faq.a1":
    "No. They keep working. {name} shows you which ones are weak or reused, so you can fix the important ones first.",
  "web.switchPage.faq.q2": "Can I go back?",
  "web.switchPage.faq.a2": "Yes. Export your whole vault at any time.",
  "web.switchPage.faq.q3": "What about my 2FA codes?",
  "web.switchPage.faq.a3":
    "Codes stored in your old password manager come along with the login. Codes in a separate authenticator app stay there until you scan them into {name}.",
} as const satisfies Record<string, string>;
