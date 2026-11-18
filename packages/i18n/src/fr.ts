/** French UI strings, assembled from `catalog/fr/`. */
import { app } from "./catalog/fr/app.ts";
import { errors } from "./catalog/fr/errors.ts";
import { onboarding } from "./catalog/fr/onboarding.ts";
import { unlock } from "./catalog/fr/unlock.ts";
import { vault } from "./catalog/fr/vault.ts";
import { items } from "./catalog/fr/items.ts";
import { generator } from "./catalog/fr/generator.ts";
import { clipboard } from "./catalog/fr/clipboard.ts";
import { insights } from "./catalog/fr/insights.ts";
import { importMessages } from "./catalog/fr/import.ts";
import { agent } from "./catalog/fr/agent.ts";
import { importGuide } from "./catalog/fr/importGuide.ts";
import { programFill } from "./catalog/fr/programFill.ts";
import { browser } from "./catalog/fr/browser.ts";
import { ssh } from "./catalog/fr/ssh.ts";
import { settings } from "./catalog/fr/settings.ts";
import { palette } from "./catalog/fr/palette.ts";
import { shell } from "./catalog/fr/shell.ts";
import { otp } from "./catalog/fr/otp.ts";
import { files } from "./catalog/fr/files.ts";
import { ext } from "./catalog/fr/ext.ts";
import { sync } from "./catalog/fr/sync.ts";
import { sharing } from "./catalog/fr/sharing.ts";
import { rotation } from "./catalog/fr/rotation.ts";
import { credentialRotation } from "./catalog/fr/credentialRotation.ts";
import { breach } from "./catalog/fr/breach.ts";
import { mobile } from "./catalog/fr/mobile.ts";
import { mobileUx } from "./catalog/fr/mobileUx.ts";
import { mobileFolder } from "./catalog/fr/mobileFolder.ts";
import type { MessageKey } from "./en.ts";

export const fr: Readonly<Record<MessageKey, string>> = {
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
