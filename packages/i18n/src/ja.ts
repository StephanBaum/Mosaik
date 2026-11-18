/** Japanese UI strings, assembled from `catalog/ja/`. */
import { app } from "./catalog/ja/app.ts";
import { errors } from "./catalog/ja/errors.ts";
import { onboarding } from "./catalog/ja/onboarding.ts";
import { unlock } from "./catalog/ja/unlock.ts";
import { vault } from "./catalog/ja/vault.ts";
import { items } from "./catalog/ja/items.ts";
import { generator } from "./catalog/ja/generator.ts";
import { clipboard } from "./catalog/ja/clipboard.ts";
import { insights } from "./catalog/ja/insights.ts";
import { importMessages } from "./catalog/ja/import.ts";
import { agent } from "./catalog/ja/agent.ts";
import { importGuide } from "./catalog/ja/importGuide.ts";
import { programFill } from "./catalog/ja/programFill.ts";
import { browser } from "./catalog/ja/browser.ts";
import { ssh } from "./catalog/ja/ssh.ts";
import { settings } from "./catalog/ja/settings.ts";
import { palette } from "./catalog/ja/palette.ts";
import { shell } from "./catalog/ja/shell.ts";
import { otp } from "./catalog/ja/otp.ts";
import { files } from "./catalog/ja/files.ts";
import { ext } from "./catalog/ja/ext.ts";
import { sync } from "./catalog/ja/sync.ts";
import { sharing } from "./catalog/ja/sharing.ts";
import { rotation } from "./catalog/ja/rotation.ts";
import { credentialRotation } from "./catalog/ja/credentialRotation.ts";
import { breach } from "./catalog/ja/breach.ts";
import { mobile } from "./catalog/ja/mobile.ts";
import { mobileUx } from "./catalog/ja/mobileUx.ts";
import { mobileFolder } from "./catalog/ja/mobileFolder.ts";
import { web } from "./catalog/en/web/index.ts";
import type { MessageKey } from "./en.ts";

export const ja: Readonly<Record<MessageKey, string>> = {
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
