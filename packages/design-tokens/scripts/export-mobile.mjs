/**
 * Writes the mobile apps' design tokens (M10, docs/MOBILE.md):
 *  - `…/ui/theme/MosaicTokens.kt` (Compose);
 *  - `apps/mobile/android/app/src/main/res/values/dimens.xml` (XML layouts: the Autofill popup);
 *  - `apps/mobile/ios/MosaicKit/Theme/MosaicTokens.swift` (SwiftUI).
 * Both are committed, and CI fails when they drift from the tokens. Run with
 * `pnpm --filter @mosaic/design-tokens export:mobile` (Node strips the types of the sources).
 */
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { androidDimensXml, androidTokensPackage, kotlinTokens, swiftTokens } from "../src/mobileExport.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const files = [
  [
    join(
      root,
      "apps/mobile/android/app/src/main/kotlin",
      ...androidTokensPackage.split("."),
      "MosaicTokens.kt",
    ),
    kotlinTokens(),
  ],
  [join(root, "apps/mobile/android/app/src/main/res/values/dimens.xml"), androidDimensXml()],
  [join(root, "apps/mobile/ios/MosaicKit/Theme/MosaicTokens.swift"), swiftTokens()],
];
for (const [path, content] of files) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, content);
}
