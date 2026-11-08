/**
 * English strings new on Pricing, Teams, Download (with Next steps and Check a download) and
 * FAQ (website v2 §5.8 to §5.11). Cards, items and answers reuse landing copy.
 */
export const plans = {
  // Pricing
  "web.pricingPage.teamsLink": "Paying for a team? See how shared vaults work.",
  "web.pricingPage.faqTitle": "Questions about paying",

  // Teams
  "web.teamsPage.storage.title": "Where your team vaults live.",
  "web.teamsPage.storage.company": "Your company cloud: a Dropbox Business team folder, or Nextcloud. Nothing new to approve.",
  "web.teamsPage.storage.hosted": "Our hosted server: no setup, and the same lock.",
  "web.teamsPage.storage.own": "Your own server: the same sync server, run on your machine.",
  "web.teamsPage.setup.title": "Set up in an afternoon.",
  "web.teamsPage.setup.s1": "Install {name} for everyone who needs it.",
  "web.teamsPage.setup.s2": "Create a shared vault and choose where it lives.",
  "web.teamsPage.setup.s3": "Invite people, and compare safety codes once.",
  "web.teamsPage.setup.s4": "When someone leaves, remove them. The vault gets a new lock.",
  "web.teamsPage.price": "Team pricing is coming soon. Shared vaults in your own team folder work on the free app too.",
  "web.teamsPage.contact.title": "Planning a rollout?",
  "web.teamsPage.contact.text":
    "Tell us how many people and where your files live. We answer every message ourselves.",
  "web.teamsPage.contact.button": "Write to us",
  "web.teamsPage.contact.capture": "Get the team setup guide by mail",

  // Download
  "web.downloadPage.eyebrow": "Download",
  "web.downloadPage.title": "Get {name}. Free on every device.",
  "web.downloadPage.sub":
    "One app for your computer, your phone and your browser. Your vault stays in sync between them.",
  "web.downloadPage.cta": "Download for {os}",
  "web.downloadPage.ctaAny": "See all downloads",
  "web.downloadPage.other": "Not your system? All downloads below.",
  "web.downloadPage.computer": "Computer",
  "web.downloadPage.phone": "Phone",
  "web.downloadPage.browser": "Browser",
  "web.downloadPage.cli": "Command line",
  "web.downloadPage.get": "Download",
  "web.downloadPage.soon": "Coming soon",
  "web.downloadPage.detected": "Your system",
  "web.downloadPage.signed.title": "Every download is signed.",
  "web.downloadPage.signed.text": "Check that a file really comes from us before you install it. It takes a minute.",
  "web.downloadPage.signed.link": "How to check a download",
  "web.downloadPage.next.title": "Installed? Here’s what comes next.",
  "web.downloadPage.next.link": "Your first five minutes",

  // Next steps
  "web.next.title": "Your first five minutes.",
  "web.next.s1": "Install and create your vault. Pick a master password you can remember: a few random words work well.",
  "web.next.s2":
    "Save your Emergency Kit. Print it or save it, and write your master password on it by hand. Keep it somewhere safe. It’s the only way back in.",
  "web.next.s3": "Bring your passwords. Follow the switch guide for your old app.",
  "web.next.s3link": "Open the switch guide",
  "web.next.s4": "Add the browser extension. Then sign in to one site with a click and see it fill.",
  "web.next.s5": "Choose where to sync. Your own cloud folder is free; hosted sync needs no setup.",
  "web.next.s5link": "See the plans",

  // Check a download
  "web.verify.title": "How to check a download.",
  "web.verify.text": "Each file has a SHA-256 checksum, listed next to it on the download page.",
  "web.verify.windows": "Windows (PowerShell)",
  "web.verify.unix": "macOS and Linux",
  "web.verify.file": "file",
  "web.verify.match": "The result must match the checksum we list. If it doesn’t, don’t install the file.",
  "web.verify.back": "Back to downloads",

  // FAQ
  "web.faqPage.safety": "Safety",
  "web.faqPage.account": "Your account",
  "web.faqPage.using": "Using it",
} as const satisfies Record<string, string>;
