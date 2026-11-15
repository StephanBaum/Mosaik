/** English UI strings: filling logins in other programs (ADR-0164). ICU MessageFormat. */
export const programFill = {
  // The chip and the list at a field in another program
  "programFill.chip.label": "Fill with Mosaic",
  "programFill.chip.locked": "Unlock Mosaic to fill",
  "programFill.list.title": "Logins for {program}",
  "programFill.list.none": "No login for {program} yet",
  "programFill.list.signerChanged": "This program's signer changed. Link again?",
  "programFill.list.fileChanged": "This program changed since you linked it. Link again?",
  "programFill.list.suggestions": "Suggestions",
  "programFill.list.search": "Search logins",
  "programFill.list.link": "Link a login…",
  "programFill.list.block": "Don't offer for this program",
  // The link step
  "programFill.link.packaged": "{name} · Store app · Publisher {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · Store app",
  "programFill.link.signed": "{exe} · Signed by {signer}, checked by Windows",
  "programFill.link.unsigned": "{exe} · Not signed · {path}",
  "programFill.link.unsignedNote":
    "Mosaic will fill only this exact file. If it changes, Mosaic asks again.",
  "programFill.link.lookalike":
    "Careful: this name looks like {similar}, which you linked before.",
  "programFill.link.hashChange": "Linked file {old}…, this file {new}…",
  "programFill.link.login": "Login: {title}",
  "programFill.link.confirm": "Link and sign in",
  "programFill.link.confirmFill": "Link and fill",
  "programFill.link.confirmUnsigned": "Link this unsigned program",
  "programFill.link.confirmAgain": "Confirm again to link",
  // Notices
  "programFill.notice.elevated":
    "Mosaic can't fill programs that run as administrator. Copy the password from Mosaic instead.",
  "programFill.notice.noField": "Mosaic can't see a login field here.",
  "programFill.notice.aborted":
    "Mosaic stopped filling because the window changed. Check the field.",
  "programFill.notice.abortedCleared":
    "Mosaic stopped filling because the window changed and removed what it had typed.",
  "programFill.notice.modifiersHeld":
    "Release Shift, Ctrl, Alt and the Windows key, then pick the login again.",
  "programFill.notice.notResponding":
    "This program didn't answer in time. Nothing more was filled.",
  "programFill.notice.close": "Close",
} as const satisfies Record<string, string>;
