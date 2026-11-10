/** English UI strings: credential rotation (M7b), the rotate-now queue and the guided change. ICU MessageFormat. */
export const credentialRotation = {
  "credentialRotation.tablist": "Security views",
  "credentialRotation.tab.overview": "Overview",
  "credentialRotation.tab.rotate": "Rotate now",
  "credentialRotation.tab.rotateCount": "Rotate now, {count} to change",

  "credentialRotation.queue.title": "Rotate now",
  "credentialRotation.queue.intro":
    "The passwords worth changing first. Mosaic guides you, but you change each password on the site yourself.",
  "credentialRotation.queue.empty": "Nothing to change right now.",
  "credentialRotation.queue.count": "{count, plural, one {# password to change} other {# passwords to change}}",
  "credentialRotation.queue.list": "Passwords to change",
  "credentialRotation.queue.snoozedHeading": "Snoozed",
  "credentialRotation.queue.snoozedUntil": "Snoozed until {date, date, medium}",
  "credentialRotation.queue.reason.weak": "Weak",
  "credentialRotation.queue.reason.reused": "Reused",
  "credentialRotation.queue.reason.old": "Old",
  "credentialRotation.queue.reason.breached": "In a breach",
  "credentialRotation.queue.reason.unfinished": "Change not finished",
  "credentialRotation.queue.change": "Change password",
  "credentialRotation.queue.continue": "Continue change",
  "credentialRotation.queue.editItem": "Edit item",
  "credentialRotation.queue.manualHint": "Mosaic can't guide this one. Open the item and change it yourself.",
  "credentialRotation.queue.actionFor": "{action}: {title}",
  "credentialRotation.queue.snooze": "Snooze",
  "credentialRotation.queue.snoozeFor": "Snooze {title}",
  "credentialRotation.queue.snoozeDays": "{days, plural, one {# day} other {# days}}",
  "credentialRotation.queue.snoozed": "Snoozed for {days, plural, one {# day} other {# days}}.",
  "credentialRotation.queue.dismiss": "Dismiss",
  "credentialRotation.queue.dismissFor": "Dismiss {title}",
  "credentialRotation.queue.dismissed": "Dismissed. It comes back when the password changes.",

  "credentialRotation.panel.title": "Change password for {title}",
  "credentialRotation.panel.step": "Step {current} of {total}",
  "credentialRotation.panel.cancel": "Cancel",

  "credentialRotation.steps.new.title": "Choose a new password",
  "credentialRotation.steps.new.intro":
    "Mosaic saves the new password in this item before you use it anywhere, so it can't get lost. The current password stays in place until you confirm.",
  "credentialRotation.steps.new.label": "New password",
  "credentialRotation.steps.new.use": "Use this password",
  "credentialRotation.steps.new.saving": "Saving…",
  "credentialRotation.steps.site.title": "Change it on the site",
  "credentialRotation.steps.site.body":
    "Open the site's password page in your browser, click Mosaic in the toolbar and choose Fill change form. Mosaic never submits the form.",
  "credentialRotation.steps.site.copyLink": "Copy link to the change page",
  "credentialRotation.steps.site.linkCopied": "Link copied.",
  "credentialRotation.steps.site.linkHint":
    "The link is not a secret. If the site has no page at that address, find its password settings by hand.",
  "credentialRotation.steps.site.noLink":
    "This login has no https address, so Mosaic has no change page to suggest. Change the password on the site by hand.",
  "credentialRotation.steps.site.copyPassword": "Copy new password",
  "credentialRotation.steps.site.next": "I've tried it",
  "credentialRotation.steps.result.title": "Did the site accept the new password?",
  "credentialRotation.steps.result.body":
    "Mosaic can't see the site. Tell it what happened so the right password stays in this item. Nothing is deleted either way.",
  "credentialRotation.steps.result.working": "Saving…",

  "credentialRotation.result.worked": "It worked",
  "credentialRotation.result.failed": "It didn't work",
  "credentialRotation.result.notSure": "Not sure yet",
  "credentialRotation.result.workedDone":
    "Done. The new password is now this item's password. The old one is in its history.",
  "credentialRotation.result.failedDone":
    "The current password stays. The new one is kept in the history as not confirmed, in case the site applied it after all.",
  "credentialRotation.result.notSureDone": "The change stays open on this item. Come back when you know.",

  "credentialRotation.banner.title": "Password change started on {date, date, medium}",
  "credentialRotation.banner.prepared": "The new password is kept here until you confirm. No page has received it yet.",
  "credentialRotation.banner.delivered":
    "The new password is kept here until you confirm. The browser extension filled it into the site's form.",
  "credentialRotation.banner.reveal": "Reveal new password",
  "credentialRotation.banner.hide": "Hide new password",
  "credentialRotation.banner.copy": "Copy new password",
  "credentialRotation.banner.newPassword": "New password",
  "credentialRotation.banner.copyLink": "Copy link to the change page",

  "credentialRotation.history.title": "Password history",
  "credentialRotation.history.list": "Earlier passwords",
  "credentialRotation.history.entry": "Replaced {date, date, medium}",
  "credentialRotation.history.unconfirmed": "Not confirmed",
  "credentialRotation.history.unconfirmedEntry": "Tried {date, date, medium}",
  "credentialRotation.history.unconfirmedHint":
    "The site may have applied this one. Restore it if signing in with the current password fails.",
  "credentialRotation.history.reveal": "Reveal password from {date, date, medium}",
  "credentialRotation.history.hide": "Hide password from {date, date, medium}",
  "credentialRotation.history.restore": "Restore",
  "credentialRotation.history.restoreFor": "Restore the password from {date, date, medium}",
  "credentialRotation.history.restored": "Restored. The previous password is in the history.",

  "credentialRotation.policy.label": "Remind me to change this password",
  "credentialRotation.policy.none": "Never",
  "credentialRotation.policy.every": "Every {days} days",
  "credentialRotation.policy.help":
    "A reminder puts the item in the Rotate now list. Mosaic sends no notification and makes no request for it.",

  "credentialRotation.url.help":
    "Optional. The https address of the site's password page, on the same site as this login. Mosaic never opens it for you.",

  "credentialRotation.settings.title": "Password reminders",
  "credentialRotation.settings.body": "Mosaic can tell you after you unlock how many passwords are worth changing.",
  "credentialRotation.settings.reminders": "Show how many passwords to change after unlock",
  "credentialRotation.settings.remindersHint": "A message in the app only. No notification, no network request.",

  "credentialRotation.reminder.toast": "{count, plural, one {# password to change} other {# passwords to change}}",
  "credentialRotation.reminder.show": "Show",
} as const satisfies Record<string, string>;
