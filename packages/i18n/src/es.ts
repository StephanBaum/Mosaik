/** Spanish UI strings, assembled from `catalog/es/`. */
import { app } from "./catalog/es/app.ts";
import { errors } from "./catalog/es/errors.ts";
import { onboarding } from "./catalog/es/onboarding.ts";
import { unlock } from "./catalog/es/unlock.ts";
import { vault } from "./catalog/es/vault.ts";
import { items } from "./catalog/es/items.ts";
import { generator } from "./catalog/es/generator.ts";
import { clipboard } from "./catalog/es/clipboard.ts";
import { insights } from "./catalog/es/insights.ts";
import { importMessages } from "./catalog/es/import.ts";
import { agent } from "./catalog/es/agent.ts";
import { importGuide } from "./catalog/es/importGuide.ts";
import { programFill } from "./catalog/es/programFill.ts";
import { browser } from "./catalog/es/browser.ts";
import { ssh } from "./catalog/es/ssh.ts";
import { settings } from "./catalog/es/settings.ts";
import { palette } from "./catalog/es/palette.ts";
import { shell } from "./catalog/es/shell.ts";
import { otp } from "./catalog/es/otp.ts";
import { files } from "./catalog/es/files.ts";
import { ext } from "./catalog/es/ext.ts";
import { sync } from "./catalog/es/sync.ts";
import { sharing } from "./catalog/es/sharing.ts";
import { rotation } from "./catalog/es/rotation.ts";
import { credentialRotation } from "./catalog/es/credentialRotation.ts";
import { breach } from "./catalog/es/breach.ts";
import { mobile } from "./catalog/es/mobile.ts";
import { mobileUx } from "./catalog/es/mobileUx.ts";
import { mobileFolder } from "./catalog/es/mobileFolder.ts";
import type { MessageKey } from "./en.ts";

export const es: Readonly<Record<MessageKey, string>> = {
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
