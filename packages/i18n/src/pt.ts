/** Brazilian Portuguese UI strings, assembled from `catalog/pt/`. */
import { app } from "./catalog/pt/app.ts";
import { errors } from "./catalog/pt/errors.ts";
import { onboarding } from "./catalog/pt/onboarding.ts";
import { unlock } from "./catalog/pt/unlock.ts";
import { vault } from "./catalog/pt/vault.ts";
import { items } from "./catalog/pt/items.ts";
import { generator } from "./catalog/pt/generator.ts";
import { clipboard } from "./catalog/pt/clipboard.ts";
import { insights } from "./catalog/pt/insights.ts";
import { importMessages } from "./catalog/pt/import.ts";
import { agent } from "./catalog/pt/agent.ts";
import { importGuide } from "./catalog/pt/importGuide.ts";
import { programFill } from "./catalog/pt/programFill.ts";
import { browser } from "./catalog/pt/browser.ts";
import { ssh } from "./catalog/pt/ssh.ts";
import { settings } from "./catalog/pt/settings.ts";
import { palette } from "./catalog/pt/palette.ts";
import { shell } from "./catalog/pt/shell.ts";
import { otp } from "./catalog/pt/otp.ts";
import { files } from "./catalog/pt/files.ts";
import { ext } from "./catalog/pt/ext.ts";
import { sync } from "./catalog/pt/sync.ts";
import { sharing } from "./catalog/pt/sharing.ts";
import { rotation } from "./catalog/pt/rotation.ts";
import { credentialRotation } from "./catalog/pt/credentialRotation.ts";
import { breach } from "./catalog/pt/breach.ts";
import { mobile } from "./catalog/pt/mobile.ts";
import { mobileUx } from "./catalog/pt/mobileUx.ts";
import { mobileFolder } from "./catalog/pt/mobileFolder.ts";
import type { MessageKey } from "./en.ts";

export const pt: Readonly<Record<MessageKey, string>> = {
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
