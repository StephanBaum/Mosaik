/** Italian UI strings, assembled from `catalog/it/`. */
import { app } from "./catalog/it/app.ts";
import { errors } from "./catalog/it/errors.ts";
import { onboarding } from "./catalog/it/onboarding.ts";
import { unlock } from "./catalog/it/unlock.ts";
import { vault } from "./catalog/it/vault.ts";
import { items } from "./catalog/it/items.ts";
import { generator } from "./catalog/it/generator.ts";
import { clipboard } from "./catalog/it/clipboard.ts";
import { insights } from "./catalog/it/insights.ts";
import { importMessages } from "./catalog/it/import.ts";
import { agent } from "./catalog/it/agent.ts";
import { importGuide } from "./catalog/it/importGuide.ts";
import { programFill } from "./catalog/it/programFill.ts";
import { browser } from "./catalog/it/browser.ts";
import { ssh } from "./catalog/it/ssh.ts";
import { settings } from "./catalog/it/settings.ts";
import { palette } from "./catalog/it/palette.ts";
import { shell } from "./catalog/it/shell.ts";
import { otp } from "./catalog/it/otp.ts";
import { files } from "./catalog/it/files.ts";
import { ext } from "./catalog/it/ext.ts";
import { sync } from "./catalog/it/sync.ts";
import { sharing } from "./catalog/it/sharing.ts";
import { rotation } from "./catalog/it/rotation.ts";
import { credentialRotation } from "./catalog/it/credentialRotation.ts";
import { breach } from "./catalog/it/breach.ts";
import { mobile } from "./catalog/it/mobile.ts";
import { mobileUx } from "./catalog/it/mobileUx.ts";
import { mobileFolder } from "./catalog/it/mobileFolder.ts";
import { web } from "./catalog/en/web/index.ts";
import type { MessageKey } from "./en.ts";

export const it: Readonly<Record<MessageKey, string>> = {
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
