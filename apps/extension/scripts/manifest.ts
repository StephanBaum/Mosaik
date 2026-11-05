/**
 * The extension manifest, one per build (docs/EXTENSION_SECURITY.md, "permissions"). The
 * production builds (`chromium`, `firefox`) hold exactly the permissions below and no host
 * permissions: pages get a content script only after a user gesture (`activeTab`). The `dev`
 * and `e2e` builds carry the dev key so the extension ID is fixed; `e2e` adds host access to
 * `*.test` for the hostile-page tests.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { de } from "../../../packages/i18n/src/de.ts";
import { en } from "../../../packages/i18n/src/en.ts";
import { FIREFOX_EXTENSION_ID } from "../src/shared/ids.ts";
import { ICON_SIZES } from "./icons.ts";

export type BuildMode = "chromium" | "firefox" | "dev" | "e2e";
export type Target = "chromium" | "firefox";

export const PERMISSIONS = ["activeTab", "scripting", "nativeMessaging", "contextMenus", "storage"] as const;
export const OPTIONAL_HOSTS = ["https://*/*", "http://*/*"] as const;
export const E2E_HOSTS = ["https://*.test/*", "http://*.test/*"] as const;

export const EXTENSION_PAGES_CSP = [
  "default-src 'none'",
  "script-src 'self'",
  "style-src 'self'",
  "img-src 'self'",
  "font-src 'self'",
  "connect-src 'none'",
  "base-uri 'none'",
  "form-action 'none'",
  "frame-ancestors 'none'",
].join("; ");

export const targetOf = (mode: BuildMode): Target => (mode === "firefox" ? "firefox" : "chromium");

const read = (path: string): string => readFileSync(join(import.meta.dirname, path), "utf8");

export const packageVersion = (): string => (JSON.parse(read("../package.json")) as { version: string }).version;

const iconPaths = (): Record<string, string> =>
  Object.fromEntries(ICON_SIZES.map((size) => [String(size), `icons/icon-${String(size)}.png`]));

function background(target: Target): Record<string, unknown> {
  return target === "firefox"
    ? { scripts: ["background.js"], type: "module" }
    : { service_worker: "background.js", type: "module" };
}

export function manifest(target: Target, mode: BuildMode): Record<string, unknown> {
  const result: Record<string, unknown> = {
    manifest_version: 3,
    name: "__MSG_extName__",
    description: "__MSG_extDescription__",
    default_locale: "en",
    version: packageVersion(),
    action: { default_popup: "popup.html", default_title: "__MSG_extName__", default_icon: iconPaths() },
    icons: iconPaths(),
    background: background(target),
    permissions: mode === "e2e" ? [...PERMISSIONS, "webNavigation"] : [...PERMISSIONS],
    optional_host_permissions: [...OPTIONAL_HOSTS],
    commands: {
      "fill-login": {
        suggested_key: { default: "Ctrl+Shift+L", mac: "Command+Shift+L" },
        description: "__MSG_commandFill__",
      },
    },
    content_security_policy: { extension_pages: EXTENSION_PAGES_CSP },
  };
  if (target === "chromium") {
    result["minimum_chrome_version"] = "121";
    result["externally_connectable"] = { ids: [] };
    if (mode === "dev" || mode === "e2e") {
      result["key"] = read("../keys/dev-public-key.b64").trim();
    }
  } else {
    result["browser_specific_settings"] = { gecko: { id: FIREFOX_EXTENSION_ID, strict_min_version: "128.0" } };
  }
  if (mode === "e2e") {
    result["host_permissions"] = [...E2E_HOSTS];
  }
  return result;
}

type Message = { message: string };

/** `_locales/<locale>/messages.json`, from the extension strings of the shared catalogs. */
export function localeMessages(locale: "en" | "de"): Record<string, Message> {
  const catalog = locale === "en" ? en : de;
  return {
    extName: { message: catalog["ext.manifest.name"] },
    extDescription: { message: catalog["ext.manifest.description"] },
    commandFill: { message: catalog["ext.command.fill"] },
    menuGenerate: { message: catalog["ext.command.generate"] },
  };
}
