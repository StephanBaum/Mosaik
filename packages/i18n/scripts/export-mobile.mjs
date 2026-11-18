/**
 * Writes the mobile apps' strings (M10, docs/MOBILE.md; languages: ADR-0168):
 *  - Android `res/<values dir>/strings.xml` per language, `res/xml/locales_config.xml` and
 *    `l10n/L10n.kt`;
 *  - iOS `Resources/Localizable.xcstrings`, `Resources/InfoPlist.xcstrings` (the Face ID and
 *    camera usage texts) and `MosaicKit/L10n/L10n.swift`.
 * All are committed, and CI fails when they drift from the catalogs. Run with
 * `pnpm --filter @mosaic/i18n export:mobile` (Node strips the types of the sources).
 */
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { mobileShared } from "../src/catalog/mobileShared.ts";
import { de } from "../src/de.ts";
import { en } from "../src/en.ts";
import { es } from "../src/es.ts";
import { fr } from "../src/fr.ts";
import { it } from "../src/it.ts";
import { ja } from "../src/ja.ts";
import { ko } from "../src/ko.ts";
import { locales } from "../src/locales.ts";
import { nl } from "../src/nl.ts";
import { pl } from "../src/pl.ts";
import { pt } from "../src/pt.ts";
import { tr } from "../src/tr.ts";
import { zh } from "../src/zh.ts";
import {
  androidLocalesConfig,
  androidPackage,
  androidStrings,
  androidValuesDir,
  buildEntries,
  infoPlistUsageTexts,
  infoPlistXcstrings,
  kotlinL10n,
  mobileKeys,
  swiftL10n,
  withMobileErrorTexts,
  xcstrings,
} from "../src/mobileExport.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const android = join(root, "apps", "mobile", "android", "app", "src", "main");
const ios = join(root, "apps", "mobile", "ios");

const sources = { en, de, es, fr, it, pt, nl, pl, tr, ja, ko, zh };
const catalogs = Object.fromEntries(
  locales.map((locale) => [locale, withMobileErrorTexts(sources[locale])]),
);
const entries = buildEntries(catalogs, mobileKeys(en, mobileShared));

const files = [
  ...locales.map((locale) => [
    join(android, "res", androidValuesDir(locale), "strings.xml"),
    androidStrings(entries, locale),
  ]),
  [join(android, "res", "xml", "locales_config.xml"), androidLocalesConfig(locales)],
  [
    join(android, "kotlin", ...androidPackage.split("."), "ui", "l10n", "L10n.kt"),
    kotlinL10n(entries),
  ],
  [join(ios, "Resources", "Localizable.xcstrings"), xcstrings(entries)],
  [join(ios, "Resources", "InfoPlist.xcstrings"), infoPlistXcstrings(catalogs, infoPlistUsageTexts)],
  [join(ios, "MosaicKit", "L10n", "L10n.swift"), swiftL10n(entries)],
];
for (const [path, content] of files) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content);
}
