/** The Emergency Kit PDF's texts, in the user's language (`shell.emergencyKit.*`). */
import type { MessageKey, MessageValues } from "@mosaic/i18n";
import type { KitLabels } from "../../ipc/contract/index.ts";

export function kitLabels(t: (key: MessageKey, values?: MessageValues) => string): KitLabels {
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
