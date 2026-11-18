/** Simplified Chinese UI strings, assembled from `catalog/zh/`. */
import { app } from "./catalog/zh/app.ts";
import { errors } from "./catalog/zh/errors.ts";
import { onboarding } from "./catalog/zh/onboarding.ts";
import { unlock } from "./catalog/zh/unlock.ts";
import { vault } from "./catalog/zh/vault.ts";
import { items } from "./catalog/zh/items.ts";
import { generator } from "./catalog/zh/generator.ts";
import { clipboard } from "./catalog/zh/clipboard.ts";
import { insights } from "./catalog/zh/insights.ts";
import { importMessages } from "./catalog/zh/import.ts";
import { agent } from "./catalog/zh/agent.ts";
import { importGuide } from "./catalog/zh/importGuide.ts";
import { programFill } from "./catalog/zh/programFill.ts";
import { browser } from "./catalog/zh/browser.ts";
import { ssh } from "./catalog/zh/ssh.ts";
import { settings } from "./catalog/zh/settings.ts";
import { palette } from "./catalog/zh/palette.ts";
import { shell } from "./catalog/zh/shell.ts";
import { otp } from "./catalog/zh/otp.ts";
import { files } from "./catalog/zh/files.ts";
import { ext } from "./catalog/zh/ext.ts";
import { sync } from "./catalog/zh/sync.ts";
import { sharing } from "./catalog/zh/sharing.ts";
import { rotation } from "./catalog/zh/rotation.ts";
import { credentialRotation } from "./catalog/zh/credentialRotation.ts";
import { breach } from "./catalog/zh/breach.ts";
import { mobile } from "./catalog/zh/mobile.ts";
import { mobileUx } from "./catalog/zh/mobileUx.ts";
import { mobileFolder } from "./catalog/zh/mobileFolder.ts";
import { web } from "./catalog/en/web/index.ts";
import type { MessageKey } from "./en.ts";

export const zh: Readonly<Record<MessageKey, string>> = {
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
