/** English UI strings: onboarding. ICU MessageFormat. */
export const onboarding = {
  "onboarding.secretKey.title": "Your Secret Key",
  "onboarding.secretKey.body":
    "Your Secret Key and master password together unlock your vault. We can't recover either of them.",
  "onboarding.secretKey.verify": "Type the last group of your Secret Key to confirm you saved it.",
  "onboarding.secretKey.mistyped":
    "That doesn't match. Check the last group in your Emergency Kit.",
  "onboarding.step": "Step {current} of {total}",
  "onboarding.create.title": "Create your account",
  "onboarding.create.body":
    "Your master password and a Secret Key we generate on this device together unlock your vault. Neither is ever sent to the server.",
  "onboarding.create.email": "Email",
  "onboarding.create.password": "Master password",
  "onboarding.create.passwordHint": "Four unrelated words work well. Use it only for Mosaic.",
  "onboarding.create.confirm": "Confirm master password",
  "onboarding.create.mismatch":
    "The two passwords don't match. Type the same master password in both fields.",
  "onboarding.create.submit": "Create account",
  "onboarding.create.working": "Creating your account. This takes a few seconds.",
  "onboarding.kit.accountId": "Account ID",
  "onboarding.kit.secretKey": "Secret Key",
  "onboarding.kit.lastGroup": "Last group of your Secret Key",
  "onboarding.kit.confirm": "Confirm",
  "kit.title": "Save your Emergency Kit",
  "kit.body":
    "The kit is a PDF with your Secret Key. Together with your master password it opens your vault on a new device. Keep it somewhere safe, away from this computer.",
  "kit.secretKey": "Your Secret Key",
  "kit.save": "Save Emergency Kit (PDF)",
  "kit.saved": "Saved. Keep the file somewhere safe.",
  "kit.cancelled": "Not saved yet. Save the kit to continue.",
  "kit.recover.alreadySaved": "I already saved it",
  "kit.recover.title": "Save your Emergency Kit again",
  "kit.recover.body":
    "You haven't confirmed your Emergency Kit yet. Enter your master password to save it again, then confirm the last group of your Secret Key.",
  "onboarding.import.title": "Bring your passwords along",
  "onboarding.import.body":
    "Import from another password manager now, or do it later from the vault.",
  "onboarding.import.start": "Import from another app",
  "onboarding.import.skip": "Skip for now",
  "onboarding.done.title": "You're all set",
  "onboarding.done.body": "Your vault is ready. Lock it any time with the Lock button.",
  "onboarding.done.open": "Open my vault",
} as const satisfies Record<string, string>;
