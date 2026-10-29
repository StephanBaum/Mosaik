/** German UI strings, informal "du", assembled from `catalog/de/`. */
import { app } from "./catalog/de/app.ts";
import { errors } from "./catalog/de/errors.ts";
import { onboarding } from "./catalog/de/onboarding.ts";
import { unlock } from "./catalog/de/unlock.ts";
import { vault } from "./catalog/de/vault.ts";
import { items } from "./catalog/de/items.ts";
import { generator } from "./catalog/de/generator.ts";
import { clipboard } from "./catalog/de/clipboard.ts";
import { insights } from "./catalog/de/insights.ts";
import { importMessages } from "./catalog/de/import.ts";
import { ssh } from "./catalog/de/ssh.ts";
import { settings } from "./catalog/de/settings.ts";
import { palette } from "./catalog/de/palette.ts";
import { shell } from "./catalog/de/shell.ts";
import type { MessageKey } from "./en.ts";

export const de: Readonly<Record<MessageKey, string>> = {
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
  ...settings,
  ...palette,
  ...shell,
};
