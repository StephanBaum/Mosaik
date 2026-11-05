import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { catalogs } from "../../../packages/i18n/src/index.ts";
import { DEV_CHROMIUM_EXTENSION_ID, FIREFOX_EXTENSION_ID } from "../src/shared/ids.ts";
import { ICON_SIZES, renderIcon } from "./icons.ts";
import { EXTENSION_PAGES_CSP, localeMessages, manifest, packageVersion } from "./manifest.ts";

const chromium = manifest("chromium", "chromium");
const firefox = manifest("firefox", "firefox");

describe("production manifest (chromium)", () => {
  it("has exactly the permissions of docs/EXTENSION_SECURITY.md", () => {
    expect([...(chromium["permissions"] as string[])].sort()).toEqual(
      ["activeTab", "contextMenus", "nativeMessaging", "scripting", "storage"].sort(),
    );
    expect(chromium["optional_host_permissions"]).toEqual(["https://*/*", "http://*/*"]);
  });

  it("grants no host access and exposes nothing to pages", () => {
    for (const key of ["host_permissions", "web_accessible_resources", "content_scripts", "key", "update_url"]) {
      expect(chromium, key).not.toHaveProperty(key);
    }
    expect(chromium["externally_connectable"]).toEqual({ ids: [] });
  });

  it("has the extension-page CSP of the spec, character for character", () => {
    expect(chromium["content_security_policy"]).toEqual({ extension_pages: EXTENSION_PAGES_CSP });
    expect(EXTENSION_PAGES_CSP).toBe(
      "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; " +
        "connect-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
    );
  });

  it("is a module service worker with a popup, a shortcut and localized strings", () => {
    expect(chromium["manifest_version"]).toBe(3);
    expect(chromium["background"]).toEqual({ service_worker: "background.js", type: "module" });
    expect(chromium["action"]).toMatchObject({ default_popup: "popup.html", default_title: "__MSG_extName__" });
    expect(chromium["name"]).toBe("__MSG_extName__");
    expect(chromium["description"]).toBe("__MSG_extDescription__");
    expect(chromium["default_locale"]).toBe("en");
    expect(chromium["minimum_chrome_version"]).toBe("121");
    expect(chromium["version"]).toBe(packageVersion());
    expect(chromium["commands"]).toEqual({
      "fill-login": {
        suggested_key: { default: "Ctrl+Shift+L", mac: "Command+Shift+L" },
        description: "__MSG_commandFill__",
      },
    });
  });

  it("lists an icon for every size", () => {
    expect(Object.keys(chromium["icons"] as object).sort()).toEqual(ICON_SIZES.map(String).sort());
  });
});

describe("firefox manifest", () => {
  it("differs only where the browser requires it", () => {
    expect(firefox["permissions"]).toEqual(chromium["permissions"]);
    expect(firefox["optional_host_permissions"]).toEqual(chromium["optional_host_permissions"]);
    expect(firefox["content_security_policy"]).toEqual(chromium["content_security_policy"]);
    expect(firefox["background"]).toEqual({ scripts: ["background.js"], type: "module" });
    expect(firefox["browser_specific_settings"]).toEqual({
      gecko: { id: FIREFOX_EXTENSION_ID, strict_min_version: "128.0" },
    });
    for (const key of ["key", "minimum_chrome_version", "externally_connectable", "host_permissions"]) {
      expect(firefox, key).not.toHaveProperty(key);
    }
  });
});

describe("dev and e2e manifests", () => {
  it("dev carries the dev key and nothing more", () => {
    const dev = manifest("chromium", "dev");
    expect(dev["key"]).toBe(readFileSync(join(import.meta.dirname, "../keys/dev-public-key.b64"), "utf8").trim());
    expect(dev).not.toHaveProperty("host_permissions");
    expect(dev["permissions"]).toEqual(chromium["permissions"]);
  });

  it("e2e adds host access to *.test and webNavigation, and keeps the rest", () => {
    const e2e = manifest("chromium", "e2e");
    expect(e2e["host_permissions"]).toEqual(["https://*.test/*", "http://*.test/*"]);
    expect(e2e["permissions"]).toEqual([...(chromium["permissions"] as string[]), "webNavigation"]);
    expect(e2e["externally_connectable"]).toEqual({ ids: [] });
    expect(e2e).not.toHaveProperty("web_accessible_resources");
    expect(e2e).toHaveProperty("key");
  });

  it("the dev key gives the extension ID that the host accepts in debug builds", () => {
    const der = Buffer.from(readFileSync(join(import.meta.dirname, "../keys/dev-public-key.b64"), "utf8").trim(), "base64");
    const hash = createHash("sha256").update(der).digest().subarray(0, 16);
    const id = [...hash].map((byte) => String.fromCharCode(97 + (byte >> 4)) + String.fromCharCode(97 + (byte & 15))).join("");
    expect(id).toBe(DEV_CHROMIUM_EXTENSION_ID);
  });
});

describe("locale files", () => {
  it.each(["en", "de"] as const)("%s has the four strings, from the shared catalog", (locale) => {
    const messages = localeMessages(locale);
    expect(Object.keys(messages).sort()).toEqual(["commandFill", "extDescription", "extName", "menuGenerate"]);
    expect(messages["extName"]?.message).toBe(catalogs[locale]["ext.manifest.name"]);
    expect(messages["commandFill"]?.message).toBe(catalogs[locale]["ext.command.fill"]);
  });

  it("keeps the description within the 132 characters the stores allow", () => {
    for (const locale of ["en", "de"] as const) {
      expect(localeMessages(locale)["extDescription"]?.message.length).toBeLessThanOrEqual(132);
    }
  });
});

describe("icons", () => {
  it("renders valid PNGs of each size", () => {
    for (const size of ICON_SIZES) {
      const png = renderIcon(size);
      expect(png.subarray(1, 4).toString("ascii")).toBe("PNG");
      expect(png.readUInt32BE(16)).toBe(size);
      expect(png.readUInt32BE(20)).toBe(size);
    }
  });
});
