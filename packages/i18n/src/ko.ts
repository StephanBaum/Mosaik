/** Korean UI strings, assembled from `catalog/ko/`. */
import { app } from "./catalog/ko/app.ts";
import { errors } from "./catalog/ko/errors.ts";
import { onboarding } from "./catalog/ko/onboarding.ts";
import { unlock } from "./catalog/ko/unlock.ts";
import { vault } from "./catalog/ko/vault.ts";
import { items } from "./catalog/ko/items.ts";
import { generator } from "./catalog/ko/generator.ts";
import { clipboard } from "./catalog/ko/clipboard.ts";
import { insights } from "./catalog/ko/insights.ts";
import { importMessages } from "./catalog/ko/import.ts";
import { agent } from "./catalog/ko/agent.ts";
import { importGuide } from "./catalog/ko/importGuide.ts";
import { programFill } from "./catalog/ko/programFill.ts";
import { browser } from "./catalog/ko/browser.ts";
import { ssh } from "./catalog/ko/ssh.ts";
import { settings } from "./catalog/ko/settings.ts";
import { palette } from "./catalog/ko/palette.ts";
import { shell } from "./catalog/ko/shell.ts";
import { otp } from "./catalog/ko/otp.ts";
import { files } from "./catalog/ko/files.ts";
import { ext } from "./catalog/ko/ext.ts";
import { sync } from "./catalog/ko/sync.ts";
import { sharing } from "./catalog/ko/sharing.ts";
import { rotation } from "./catalog/ko/rotation.ts";
import { credentialRotation } from "./catalog/ko/credentialRotation.ts";
import { breach } from "./catalog/ko/breach.ts";
import { mobile } from "./catalog/ko/mobile.ts";
import { mobileUx } from "./catalog/ko/mobileUx.ts";
import { mobileFolder } from "./catalog/ko/mobileFolder.ts";
import type { MessageKey } from "./en.ts";

export const ko: Readonly<Record<MessageKey, string>> = {
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
