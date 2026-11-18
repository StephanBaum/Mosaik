/** Dutch UI strings, assembled from `catalog/nl/`. */
import { app } from "./catalog/nl/app.ts";
import { errors } from "./catalog/nl/errors.ts";
import { onboarding } from "./catalog/nl/onboarding.ts";
import { unlock } from "./catalog/nl/unlock.ts";
import { vault } from "./catalog/nl/vault.ts";
import { items } from "./catalog/nl/items.ts";
import { generator } from "./catalog/nl/generator.ts";
import { clipboard } from "./catalog/nl/clipboard.ts";
import { insights } from "./catalog/nl/insights.ts";
import { importMessages } from "./catalog/nl/import.ts";
import { agent } from "./catalog/nl/agent.ts";
import { importGuide } from "./catalog/nl/importGuide.ts";
import { programFill } from "./catalog/nl/programFill.ts";
import { browser } from "./catalog/nl/browser.ts";
import { ssh } from "./catalog/nl/ssh.ts";
import { settings } from "./catalog/nl/settings.ts";
import { palette } from "./catalog/nl/palette.ts";
import { shell } from "./catalog/nl/shell.ts";
import { otp } from "./catalog/nl/otp.ts";
import { files } from "./catalog/nl/files.ts";
import { ext } from "./catalog/nl/ext.ts";
import { sync } from "./catalog/nl/sync.ts";
import { sharing } from "./catalog/nl/sharing.ts";
import { rotation } from "./catalog/nl/rotation.ts";
import { credentialRotation } from "./catalog/nl/credentialRotation.ts";
import { breach } from "./catalog/nl/breach.ts";
import { mobile } from "./catalog/nl/mobile.ts";
import { mobileUx } from "./catalog/nl/mobileUx.ts";
import { mobileFolder } from "./catalog/nl/mobileFolder.ts";
import type { MessageKey } from "./en.ts";

export const nl: Readonly<Record<MessageKey, string>> = {
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
