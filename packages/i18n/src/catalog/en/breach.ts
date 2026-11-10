/** English UI strings: the opt-in breach check (M7b). ICU MessageFormat. */
export const breach = {
  "breach.card.title": "Breach check",
  "breach.card.off": "Off. Mosaic never contacts Have I Been Pwned unless you turn this on.",
  "breach.card.about":
    "Checks your passwords against known data breaches. Only the first 5 characters of each password's SHA-1 hash leave this device.",
  "breach.mode.label": "Check against known data breaches",
  "breach.mode.off": "Off",
  "breach.mode.manual": "Only when I choose",
  "breach.mode.weekly": "Weekly, after unlock",
  "breach.card.checkNow": "Check now",
  "breach.card.stop": "Stop",
  "breach.card.turnOff": "Turn off",

  "breach.progress.starting": "Checking your passwords…",
  "breach.progress.step": "Checking… {done} of {total}",

  "breach.result.never": "Not checked yet.",
  "breach.result.last":
    "Last checked {date, date, medium}. {found, plural, =0 {No password was found in a breach.} one {# password was found in a breach.} other {# passwords were found in a breach.}}",
  "breach.result.finished":
    "Checked {checked, plural, one {# password} other {# passwords}}. {found, plural, =0 {None was found in a breach.} one {# was found in a breach.} other {# were found in a breach.}}",
  "breach.result.upToDate": "Everything was checked recently. Nothing new to look up.",

  "breach.error.offline":
    "Couldn't reach Have I Been Pwned. The last results are shown. Try again when you're online.",
  "breach.error.rateLimited":
    "Have I Been Pwned asked Mosaic to slow down. Try again in a few minutes.",
  "breach.error.protocol":
    "Have I Been Pwned sent an answer Mosaic doesn't trust, so some passwords weren't checked. Try again later.",
  "breach.error.cancelled": "The check was stopped. Results that were finished are kept.",

  "breach.consent.title": "Check your passwords against known data breaches?",
  "breach.consent.body":
    "Mosaic sends the first 5 characters of each password's SHA-1 hash to Have I Been Pwned (api.pwnedpasswords.com). The service sees your IP address and those prefixes, never a password. Results stay on this device.",
  "breach.consent.detail":
    "Mosaic sends the prefixes one by one in random order and asks for padded answers. It is still a request to a third party, so this stays off until you agree. You can turn it off again at any time.",
  "breach.consent.agree": "Check now",
  "breach.consent.decline": "Not now",
} as const satisfies Record<string, string>;
