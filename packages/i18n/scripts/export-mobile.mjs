/**
 * Writes the mobile apps' strings (M10, docs/MOBILE.md):
 *  - Android `res/values/strings.xml` and `res/values-de/strings.xml`, and `l10n/L10n.kt`;
 *  - iOS `Resources/Localizable.xcstrings` and `MosaicKit/L10n/L10n.swift`.
 * All are committed, and CI fails when they drift from the catalogs. Run with
 * `pnpm --filter @mosaic/i18n export:mobile` (Node strips the types of the sources).
 */
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { mobileShared } from "../src/catalog/mobileShared.ts";
import { de } from "../src/de.ts";
import { en } from "../src/en.ts";
import {
  androidPackage,
  androidStrings,
  buildEntries,
  kotlinL10n,
  mobileKeys,
  swiftL10n,
  xcstrings,
} from "../src/mobileExport.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const android = join(root, "apps", "mobile", "android", "app", "src", "main");
const ios = join(root, "apps", "mobile", "ios");

const entries = buildEntries(en, de, mobileKeys(en, mobileShared));

const files = [
  [join(android, "res", "values", "strings.xml"), androidStrings(entries, "en")],
  [join(android, "res", "values-de", "strings.xml"), androidStrings(entries, "de")],
  [
    join(android, "kotlin", ...androidPackage.split("."), "ui", "l10n", "L10n.kt"),
    kotlinL10n(entries),
  ],
  [join(ios, "Resources", "Localizable.xcstrings"), xcstrings(entries)],
  [join(ios, "MosaicKit", "L10n", "L10n.swift"), swiftL10n(entries)],
];
for (const [path, content] of files) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content);
}
