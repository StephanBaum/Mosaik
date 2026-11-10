/**
 * English strings of the security-key page that the sync server serves at `/webauthn/`
 * (CRYPTO_SPEC §9.3). Plain `{name}` placeholders only: the page substitutes them itself and
 * writes the result as text, never as HTML. `pnpm --filter @mosaic/i18n export:webauthn` writes
 * both catalogs to `crates/server/assets/webauthn/strings.json`.
 */
export const webauthn = {
  "webauthn.documentTitle": "Mosaic: security key",
  "webauthn.heading.code": "Check the code first",
  "webauthn.lead.code":
    "Type the 6 digits that the Mosaic app shows on your computer. Only then does this page ask for your security key.",
  "webauthn.code.label": "Code from the Mosaic app",
  "webauthn.code.submit": "Continue",
  "webauthn.code.format": "Enter the 6 digits.",
  "webauthn.code.invalid": "That code is not right. Check the digits in the Mosaic app.",
  "webauthn.heading.login": "Sign in with your security key",
  "webauthn.heading.register": "Add a security key",
  "webauthn.heading.confirm": "Confirm with your security key",
  "webauthn.facts.device": "Device",
  "webauthn.facts.time": "Asked at",
  "webauthn.facts.account": "Account",
  "webauthn.lead.ask":
    "Make sure the device and time above are yours. Then use your security key when your browser asks for it.",
  "webauthn.status.waiting": "Waiting for your security key.",
  "webauthn.status.sending": "Checking your answer.",
  "webauthn.retry": "Try again",
  "webauthn.cancelled":
    "Your security key was not used: the request was cancelled or ran out of time.",
  "webauthn.alreadyRegistered": "This security key is already added to your account.",
  "webauthn.browserError": "Your browser could not use a security key for this.",
  "webauthn.unsupported":
    "This browser cannot use security keys. Open the link in an up-to-date browser.",
  "webauthn.heading.done": "Done",
  "webauthn.done.login": "You are signed in. Go back to the Mosaic app. You can close this tab.",
  "webauthn.done.register":
    "Your security key is added. Go back to the Mosaic app. You can close this tab.",
  "webauthn.done.confirm": "Confirmed. Go back to the Mosaic app. You can close this tab.",
  "webauthn.heading.expired": "This request has run out",
  "webauthn.expired.body":
    "It was valid for a few minutes only. Start again in the Mosaic app on your computer.",
  "webauthn.heading.failed": "That did not work",
  "webauthn.failed.body":
    "Your security key was not accepted, or too many tries were used up. Start again in the Mosaic app on your computer.",
  "webauthn.heading.invalid": "This link is not complete",
  "webauthn.invalid.body":
    "Open the link from the Mosaic app on your computer. Do not retype it or take it from anywhere else.",
  "webauthn.rateLimited": "Too many tries. Wait a few minutes, then start again in the Mosaic app.",
  "webauthn.unavailable":
    "This server does not offer security keys right now. Use the Mosaic app to sign in another way.",
  "webauthn.network": "Could not reach the server. Check your connection and try again.",
} as const;
