/**
 * English UI strings for the website v2 frame: navigation, footer, page titles and
 * descriptions, the capture card, and the small pages (thanks, confirmed, legal, 404).
 * Copy: docs/superpowers/specs/2026-11-08-website-v2.md §1 and §5, verbatim.
 */
export const site = {
  // Navigation
  "web.nav.security": "Security",
  "web.nav.compare": "Compare",
  "web.crumbs.label": "Breadcrumb",

  // Footer
  "web.footer.product": "Product",
  "web.footer.start": "Get started",
  "web.footer.tools": "Free resources",
  "web.footer.compare": "Compare",
  "web.footer.faq": "FAQ",
  "web.footer.download": "Download",
  "web.footer.switch": "Switch guide",
  "web.footer.verify": "Check a download",
  "web.footer.selfCheck": "How exposed are you?",
  "web.footer.switchKit": "Switch kit",
  "web.footer.checklist": "Switch checklist",
  "web.footer.overview": "Overview",
  "web.footer.vs1password": "vs 1Password",
  "web.footer.vsBitwarden": "vs Bitwarden",
  "web.footer.imprint": "Imprint",
  "web.footer.privacy": "Privacy",

  // Page titles and descriptions
  "web.home.meta.title": "{name}: Every login. One click. Yours alone.",
  "web.home.meta.description":
    "{name} is a password manager that keeps your logins on your devices, readable only by you. No ads. No tracking.",
  "web.features.meta.title": "Features · {name}",
  "web.features.meta.description":
    "Autofill that can’t be tricked, passkeys on every device, 2FA codes that fill themselves, and more. Everything {name} does.",
  "web.security.meta.title": "Security · {name}",
  "web.security.meta.description":
    "How {name} locks every entry on your device, lets you choose where it’s kept, and checks that the server plays fair.",
  "web.compare.meta.title": "{name} compared with 1Password, Bitwarden and more",
  "web.compare.meta.description":
    "A fair comparison of {name}, 1Password, Bitwarden, Proton Pass, KeePassXC and Enpass. Every cell links to its source.",
  "web.vs1password.meta.title": "{name} vs 1Password",
  "web.vs1password.meta.description":
    "Both lock your vault with two secrets. The difference is who you have to trust. A sourced comparison.",
  "web.vsBitwarden.meta.title": "{name} vs Bitwarden",
  "web.vsBitwarden.meta.description":
    "Both are cheap to start. They make different choices about defaults and trust. A sourced comparison.",
  "web.pricingPage.meta.title": "Pricing · {name}",
  "web.pricingPage.meta.description":
    "The app is free, every feature on every device. You pay only if you want us to run the sync.",
  "web.teamsPage.meta.title": "{name} for teams and families",
  "web.teamsPage.meta.description":
    "Shared vaults for small teams and families, kept in your own company cloud or on our hosted server.",
  "web.downloadPage.meta.title": "Download {name}",
  "web.downloadPage.meta.description":
    "Get {name} free for Windows, macOS, Linux, iPhone, Android and your browser. Every download is signed.",
  "web.next.meta.title": "Your first five minutes · {name}",
  "web.next.meta.description": "Set up {name} in five steps: vault, Emergency Kit, import, extension and sync.",
  "web.verify.meta.title": "How to check a download · {name}",
  "web.verify.meta.description": "Check that a {name} download really comes from us, with its SHA-256 checksum.",
  "web.switchPage.meta.title": "Switch to {name} in minutes",
  "web.switchPage.meta.description":
    "Move from 1Password, Bitwarden, LastPass, KeePass, Enpass or your browser. Export, import, clean up.",
  "web.check.meta.title": "How exposed are you? A two-minute self-check · {name}",
  "web.check.meta.description":
    "Seven multiple-choice questions about how you keep your logins today. No passwords asked, nothing stored, nothing sent.",
  "web.faqPage.meta.title": "Questions and answers · {name}",
  "web.faqPage.meta.description": "What we can see, what happens if you forget your master password, and more.",
  "web.thanks.meta.title": "Check your inbox · {name}",
  "web.thanks.meta.description": "Confirm your address to get the switch kit.",
  "web.confirmed.meta.title": "You’re in · {name}",
  "web.confirmed.meta.description": "Your address is confirmed.",
  "web.imprint.meta.title": "Imprint · {name}",
  "web.imprint.meta.description": "Who is responsible for this website.",
  "web.privacy.meta.title": "Privacy · {name}",
  "web.privacy.meta.description": "How this website handles your data.",
  "web.notFound.meta.title": "Page not found · {name}",
  "web.notFound.meta.description": "This page fell apart.",

  // Capture card (switch kit)
  "web.capture.title": "Switching soon? Get the switch kit.",
  "web.capture.text":
    "The complete kit as a printable PDF, plus five short mails that take you through the move, one step at a time.",
  "web.capture.label": "Email address",
  "web.capture.button": "Send me the kit",
  "web.capture.privacy":
    "We’ll ask you to confirm first. No tracking in our mails, one click to unsubscribe, and we never pass your address on.",
  "web.capture.error": "That doesn’t look like an email address.",
  "web.capture.soon.title": "Mail sign-up opens soon.",
  "web.capture.soon.text": "The whole switch guide is already here, no address needed.",
  "web.capture.soon.link": "Open the switch guide",

  // Thanks and confirmed
  "web.thanks.title": "Check your inbox.",
  "web.thanks.text":
    "We’ve sent a link to confirm your address. It’s valid for 7 days. Nothing there? Look in your spam folder, or send the form again if you mistyped.",
  "web.thanks.link": "Meanwhile: download {name}",
  "web.confirmed.title": "You’re in.",
  "web.confirmed.text": "The first mail is on its way.",

  // Legal
  "web.imprint.title": "Imprint",
  "web.privacy.title": "Privacy",
  "web.legal.responsible": "Responsible for this website",
  "web.legal.address": "Address",
  "web.legal.contact": "Contact",

  // 404
  "web.notFound.title": "This page fell apart.",
  "web.notFound.body": "The address may be old or mistyped.",
  "web.notFound.home": "Home",
  "web.notFound.check": "Self-check",
  "web.notFound.download": "Download",

  // Product shots and mockups (alt texts)
  "web.shot.unlock": "The {name} unlock screen, asking for the master password.",
  "web.shot.item": "A login in {name} with its user name, password and a running one-time code.",
  "web.shot.passkeys": "A passkey saved in {name}.",
  "web.shot.totp": "A one-time code in {name}, with the ring that shows how long it stays valid.",
  "web.shot.list": "The list of logins in the {name} desktop app.",
  "web.shot.insights": "The {name} security overview: weak, reused and leaked passwords.",
  "web.shot.import": "The {name} import preview, with duplicates marked before anything is saved.",
  "web.teamsPage.mock.label": "Sample: a shared vault with three members and their safety-code status.",
  "web.teamsPage.mock.vault": "Family",
  "web.teamsPage.mock.count": "3 members",
  "web.teamsPage.mock.verified": "Verified",
  "web.teamsPage.mock.pending": "Compare code",
  "web.security.stolen.mock": "Sample: rows of sealed data, unreadable without your two keys.",
} as const satisfies Record<string, string>;
