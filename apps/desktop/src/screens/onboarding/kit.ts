/** The Emergency Kit PDF's texts, in the user's language (`shell.emergencyKit.*`). */
import { formatMessage, type MessageKey, type MessageValues } from "@mosaic/i18n";
import type { KitLabels } from "../../ipc/contract/index.ts";

type Translate = (key: MessageKey, values?: MessageValues) => string;

function labelsWith(t: Translate): KitLabels {
  return {
    title: t("shell.emergencyKit.title"),
    intro: t("shell.emergencyKit.intro"),
    email: t("shell.emergencyKit.email"),
    accountId: t("shell.emergencyKit.accountId"),
    signInUrl: t("shell.emergencyKit.signInUrl"),
    signInUrlValue: t("shell.emergencyKit.signInUrlLocal"),
    secretKey: t("shell.emergencyKit.secretKey"),
    masterPassword: t("shell.emergencyKit.masterPassword"),
    masterPasswordHint: t("shell.emergencyKit.masterPasswordHint"),
    footer: t("shell.emergencyKit.footer"),
  };
}

// WinAnsiEncoding (PDF 32000-1 Annex D): printable ASCII, Latin-1 and the extra characters at
// 0x80 to 0x9F, as in `src-tauri/src/emergency_kit.rs`.
const WIN_ANSI_EXTRA = new Set("€‚ƒ„…†‡ˆ‰Š‹ŒŽ‘’“”•–—˜™š›œžŸ");

/** Whether the PDF's base-14 fonts can print `text` (else Rust writes `?` for a character). */
export function isWinAnsi(text: string): boolean {
  for (const c of text) {
    const code = c.codePointAt(0) ?? 0;
    const latin = (code >= 0x20 && code <= 0x7e) || (code >= 0xa0 && code <= 0xff);
    if (!latin && !WIN_ANSI_EXTRA.has(c)) {
      return false;
    }
  }
  return true;
}

/**
 * The kit's labels in the user's language, or in English when that language needs characters
 * the PDF's fonts lack (Polish, Turkish, Japanese, Korean, Chinese; ADR-0168): a kit of `?` would
 * be no use in an emergency.
 */
export function kitLabels(t: Translate): KitLabels {
  const labels = labelsWith(t);
  if (Object.values(labels).every(isWinAnsi)) {
    return labels;
  }
  return labelsWith((key, values) => formatMessage("en", key, values));
}

/** The Secret Key's format (CRYPTO_SPEC §5.2) with every group but the last masked. */
export const MASKED_KEY_PREFIX = `M1-${"••••-".repeat(6)}`;
