/**
 * English UI strings (source locale), assembled from the per-area modules in `catalog/en/`.
 * Every key must exist in every other locale (enforced by the type of `de` and per module).
 */
import { app } from "./catalog/en/app.ts";
import { errors } from "./catalog/en/errors.ts";
import { onboarding } from "./catalog/en/onboarding.ts";
import { unlock } from "./catalog/en/unlock.ts";
import { vault } from "./catalog/en/vault.ts";
import { items } from "./catalog/en/items.ts";
import { generator } from "./catalog/en/generator.ts";
import { clipboard } from "./catalog/en/clipboard.ts";
import { insights } from "./catalog/en/insights.ts";
import { importMessages } from "./catalog/en/import.ts";
import { browser } from "./catalog/en/browser.ts";
import { ssh } from "./catalog/en/ssh.ts";
import { settings } from "./catalog/en/settings.ts";
import { palette } from "./catalog/en/palette.ts";
import { shell } from "./catalog/en/shell.ts";
import { otp } from "./catalog/en/otp.ts";
import { files } from "./catalog/en/files.ts";
import { ext } from "./catalog/en/ext.ts";
import { sync } from "./catalog/en/sync.ts";

export const en = {
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
  ...ssh,
  ...browser,
  ...settings,
  ...palette,
  ...shell,
  ...otp,
  ...files,
  ...ext,
  ...sync,
} as const satisfies Record<string, string>;

export type MessageKey = keyof typeof en;
