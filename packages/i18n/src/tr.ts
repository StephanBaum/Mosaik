/** Turkish UI strings, assembled from `catalog/tr/`. */
import { app } from "./catalog/tr/app.ts";
import { errors } from "./catalog/tr/errors.ts";
import { onboarding } from "./catalog/tr/onboarding.ts";
import { unlock } from "./catalog/tr/unlock.ts";
import { vault } from "./catalog/tr/vault.ts";
import { items } from "./catalog/tr/items.ts";
import { generator } from "./catalog/tr/generator.ts";
import { clipboard } from "./catalog/tr/clipboard.ts";
import { insights } from "./catalog/tr/insights.ts";
import { importMessages } from "./catalog/tr/import.ts";
import { agent } from "./catalog/tr/agent.ts";
import { importGuide } from "./catalog/tr/importGuide.ts";
import { programFill } from "./catalog/tr/programFill.ts";
import { browser } from "./catalog/tr/browser.ts";
import { ssh } from "./catalog/tr/ssh.ts";
import { settings } from "./catalog/tr/settings.ts";
import { palette } from "./catalog/tr/palette.ts";
import { shell } from "./catalog/tr/shell.ts";
import { otp } from "./catalog/tr/otp.ts";
import { files } from "./catalog/tr/files.ts";
import { ext } from "./catalog/tr/ext.ts";
import { sync } from "./catalog/tr/sync.ts";
import { sharing } from "./catalog/tr/sharing.ts";
import { rotation } from "./catalog/tr/rotation.ts";
import { credentialRotation } from "./catalog/tr/credentialRotation.ts";
import { breach } from "./catalog/tr/breach.ts";
import { mobile } from "./catalog/tr/mobile.ts";
import { mobileUx } from "./catalog/tr/mobileUx.ts";
import { mobileFolder } from "./catalog/tr/mobileFolder.ts";
import { web } from "./catalog/en/web/index.ts";
import type { MessageKey } from "./en.ts";

export const tr: Readonly<Record<MessageKey, string>> = {
  // The website is not translated into the app-only languages (ADR-0168): its keys fall back to English.
  ...web,
  ...app,
  ...errors,
  ...onboarding,
  ...unlock,
  ...vault,
  ...items,
  ...generator,
  ...clipboard,
  ...insights,
  ...importMessages,
  ...importGuide,
  ...ssh,
  ...browser,
  ...agent,
  ...programFill,
  ...settings,
  ...palette,
  ...shell,
  ...otp,
  ...files,
  ...ext,
  ...sync,
  ...sharing,
  ...rotation,
  ...credentialRotation,
  ...breach,
  ...mobile,
  ...mobileUx,
  ...mobileFolder,
};
