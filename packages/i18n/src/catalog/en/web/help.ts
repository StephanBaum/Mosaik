/**
 * English strings of the report and security challenge pages, the challenge block on the
 * security page and the two footer links. Facts follow SECURITY.md, docs/BUG_BOUNTY.md and
 * docs/CHALLENGE.md; the site itself takes no input, it links to GitHub.
 */
export const help = {
  "web.report.meta.title": "Report a problem · {name}",
  "web.report.meta.description": "Found a bug or a security hole in {name}? Here is where to report it, and what to leave out.",
  "web.report.eyebrow": "Report a problem",
  "web.report.title": "Found something? Tell us.",
  "web.report.sub": "Every report gets read. Security reports go through a private channel and get an answer within seven days.",
  "web.report.bug.title": "Something doesn’t work",
  "web.report.bug.text":
    "Open a public issue on GitHub. Tell us what you did, what you expected and what happened, plus your system and your {name} version.",
  "web.report.bug.link": "Open an issue on GitHub",
  "web.report.vuln.title": "You found a security hole",
  "web.report.vuln.text":
    "Please don’t post it in public. Report it privately on GitHub. We confirm within 7 days and assess within 14. We publish when it’s fixed, or after 90 days, whichever comes first.",
  "web.report.vuln.link": "Report privately on GitHub",
  "web.report.vuln.policy": "Read the security policy",
  "web.report.never.title": "Never send us secrets",
  "web.report.never.text":
    "No report needs your passwords, your master password, your Secret Key, your Emergency Kit or your vault files. If a screenshot shows any of them, cover it first.",
  "web.report.challenge": "Want to dig deeper? Join the security challenge.",
  "web.challenge.meta.title": "Security challenge · {name}",
  "web.challenge.meta.description": "Try to break {name}. Seven targets, a public test server, fair rules and safe harbor.",
  "web.challenge.eyebrow": "Security challenge",
  "web.challenge.title": "Break it. We mean it.",
  "web.challenge.sub":
    "We don’t hide behind a seal. We publish targets, hand you what an attacker would have, and thank everyone who finds a hole.",
  "web.challenge.rules": "Read the rules",
  "web.challenge.targetsLink": "See the targets",
  "web.challenge.open": "The test server is open. Its address is listed with the targets.",
  "web.challenge.soon": "The test server opens with the first season. The rules and targets are public already.",
  "web.challenge.how.title": "How it runs",
  "web.challenge.how1": "Open all the time, in seasons. Each season starts with fresh data and new flags.",
  "web.challenge.how2": "The test server is separate and holds no real data. It’s wiped every week.",
  "web.challenge.how3": "Find a flag, then report how you got it. A flag without steps doesn’t count.",
  "web.challenge.how4": "Safe harbor: research within the rules is welcome. The rules say exactly what we promise.",
  "web.challenge.targets.title": "Seven targets",
  "web.challenge.c1": "Hostile server: you get its full data directory.",
  "web.challenge.c2": "Stolen device: a locked local store, without the keystore.",
  "web.challenge.c3": "Export file: protected by a six-word passphrase.",
  "web.challenge.c4": "Second factor: the password and Secret Key are public, the second factor is not.",
  "web.challenge.c5": "Other accounts: read or change someone else’s records.",
  "web.challenge.c6": "Browser extension: make it fill the wrong site, or fill without a click.",
  "web.challenge.c7": "Desktop app: read vault data as another user or from a sandbox.",
  "web.challenge.scope":
    "Not part of it: denial of service, social engineering, physical access, third-party services and this website. Bugs on this website are still welcome as reports.",
  "web.challenge.reward":
    "By default we say thank you in the hall of fame, if you want to be listed. If a season has prizes, the rules list them.",
  "web.challenge.hallOfFame": "Hall of fame",
  "web.challenge.report": "Report a finding",
  "web.security.challenge.title": "Don’t take our word for it.",
  "web.security.challenge.text":
    "Try to break {name} in the open security challenge. Seven targets, a public test server, and a thank-you for every hole you find.",
  "web.security.challenge.link": "To the challenge",
  "web.footer.report": "Report a problem",
  "web.footer.challenge": "Security challenge",
} as const satisfies Record<string, string>;
