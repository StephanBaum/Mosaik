// UNTRANSLATED: copy of English, replace every value
/**
 * Korean UI strings: programFill. Same keys as English (checked by the type).
 * ICU plural categories of Korean: other (CLDR; `other` is required).
 */
import type { programFill as en } from "../en/programFill.ts";

export const programFill = {
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
  "programFill.link.packaged": "{name} · Store app · Publisher {publisher}",
  "programFill.link.packagedNoPublisher": "{name} · Store app",
  "programFill.link.signed": "{exe} · Signed by {signer}, checked by Windows",
  "programFill.link.unsigned": "{exe} · Not signed · {path}",
  "programFill.link.unsignedNote":
    "Mosaic will fill only this exact file. If it changes, Mosaic asks again.",
  "programFill.link.lookalike": "Careful: this name looks like {similar}, which you linked before.",
  "programFill.link.hashChange": "Linked file {old}…, this file {new}…",
  "programFill.link.login": "Login: {title}",
  "programFill.link.confirm": "Link and sign in",
  "programFill.link.confirmFill": "Link and fill",
  "programFill.link.confirmUnsigned": "Link this unsigned program",
  "programFill.link.confirmAgain": "Confirm again to link",
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
  "programFill.settings.title": "Fill in programs",
  "programFill.settings.description":
    "Mosaic watches which field has focus in other programs on this device to offer your logins there. Nothing is stored or sent.",
  "programFill.settings.enable": "Offer logins in programs",
  "programFill.settings.submit": "Sign in after filling",
  "programFill.settings.hotkey": "Shortcut to open the list",
  "programFill.settings.hotkeyNone": "No shortcut",
  "programFill.settings.hotkeySave": "Save list shortcut",
  "programFill.settings.blocked": "Programs Mosaic doesn't offer logins for",
  "programFill.settings.blockedNone": "None",
  "programFill.settings.unblock": "Offer again",
  "programFill.settings.notWindows": "Only on Windows for now.",
  "programFill.settings.mosaicElevated":
    "Mosaic runs as administrator, so it doesn't fill programs. Start it normally.",
  "programFill.settings.hostFailed":
    "Mosaic couldn't start watching programs. Restart Mosaic to try again.",
  "programFill.card.title": "Fill logins in programs too",
  "programFill.card.turnOn": "Turn on",
  "programFill.card.notNow": "Not now",
  "programFill.item.program": "Program",
  "programFill.item.storeApp": "Store app",
  "programFill.item.signedBy": "Signed by {signer}",
  "programFill.item.unsigned": "Not signed: {path}",
  "programFill.item.submit": "Sign in after filling",
  "programFill.item.unlink": "Unlink",
} satisfies Record<keyof typeof en, string>;
