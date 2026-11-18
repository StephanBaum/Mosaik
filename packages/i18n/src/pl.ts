/** Polish UI strings, assembled from `catalog/pl/`. */
import { app } from "./catalog/pl/app.ts";
import { errors } from "./catalog/pl/errors.ts";
import { onboarding } from "./catalog/pl/onboarding.ts";
import { unlock } from "./catalog/pl/unlock.ts";
import { vault } from "./catalog/pl/vault.ts";
import { items } from "./catalog/pl/items.ts";
import { generator } from "./catalog/pl/generator.ts";
import { clipboard } from "./catalog/pl/clipboard.ts";
import { insights } from "./catalog/pl/insights.ts";
import { importMessages } from "./catalog/pl/import.ts";
import { agent } from "./catalog/pl/agent.ts";
import { importGuide } from "./catalog/pl/importGuide.ts";
import { programFill } from "./catalog/pl/programFill.ts";
import { browser } from "./catalog/pl/browser.ts";
import { ssh } from "./catalog/pl/ssh.ts";
import { settings } from "./catalog/pl/settings.ts";
import { palette } from "./catalog/pl/palette.ts";
import { shell } from "./catalog/pl/shell.ts";
import { otp } from "./catalog/pl/otp.ts";
import { files } from "./catalog/pl/files.ts";
import { ext } from "./catalog/pl/ext.ts";
import { sync } from "./catalog/pl/sync.ts";
import { sharing } from "./catalog/pl/sharing.ts";
import { rotation } from "./catalog/pl/rotation.ts";
import { credentialRotation } from "./catalog/pl/credentialRotation.ts";
import { breach } from "./catalog/pl/breach.ts";
import { mobile } from "./catalog/pl/mobile.ts";
import { mobileUx } from "./catalog/pl/mobileUx.ts";
import { mobileFolder } from "./catalog/pl/mobileFolder.ts";
import { web } from "./catalog/en/web/index.ts";
import type { MessageKey } from "./en.ts";

export const pl: Readonly<Record<MessageKey, string>> = {
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
