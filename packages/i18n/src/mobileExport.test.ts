/// <reference types="node" />
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { mobileErrorCodes, mobileShared } from "./catalog/mobileShared.ts";
import { de } from "./de.ts";
import { en } from "./en.ts";
import {
  accessorName,
  androidStrings,
  buildEntries,
  escapeAndroidText,
  kotlinL10n,
  mobileKeys,
  swiftL10n,
  xcstrings,
} from "./mobileExport.ts";

const entriesOf = (enMsgs: Record<string, string>, deMsgs: Record<string, string>) =>
  buildEntries(enMsgs, deMsgs, Object.keys(enMsgs));

describe("mobile export", () => {
  it("a plural without a visible count gets an xcstrings count substitution", () => {
    const entries = entriesOf(
      { "a.urls": "{count, plural, one {Website} other {Websites}}" },
      { "a.urls": "{count, plural, one {Website} other {Websites}}" },
    );
    const parsed = JSON.parse(xcstrings(entries)) as {
      strings: Record<string, { localizations: Record<string, Record<string, unknown>> }>;
    };
    const en = parsed.strings["a.urls"]?.localizations["en"];
    expect(en?.["stringUnit"]).toEqual({ state: "translated", value: "%#@count@" });
    expect(en?.["substitutions"]).toMatchObject({ count: { argNum: 1, formatSpecifier: "lld" } });
  });

  it("android xml comments contain no double hyphen", () => {
    const xml = androidStrings(entriesOf({ "a.b": "x" }, { "a.b": "y" }), "en");
    for (const comment of xml.match(/<!--([\s\S]*?)-->/g) ?? []) {
      expect(comment.slice(4, -3)).not.toContain("--");
    }
  });

  it("placeholders become positional android and xcode args", () => {
    const entries = entriesOf(
      { "mobile.greet": "{name} has {thing}, {name}!" },
      { "mobile.greet": "{thing} gehört {name}." },
    );
    // Positions come from the English first appearance: name = 1, thing = 2.
    expect(androidStrings(entries, "en")).toContain(
      '<string name="mobile_greet">%1$s has %2$s, %1$s!</string>',
    );
    expect(androidStrings(entries, "de")).toContain(
      '<string name="mobile_greet">%2$s gehört %1$s.</string>',
    );
    const xc = JSON.parse(xcstrings(entries)) as {
      strings: Record<string, { localizations: Record<string, { stringUnit: { value: string } }> }>;
    };
    const loc = xc.strings["mobile.greet"]?.localizations;
    expect(loc?.["en"]?.stringUnit.value).toBe("%1$@ has %2$@, %1$@!");
    expect(loc?.["de"]?.stringUnit.value).toBe("%2$@ gehört %1$@.");
    expect(kotlinL10n(entries)).toContain(
      "fun greet(r: Resources, name: String, thing: String): String =\n        r.getString(R.string.mobile_greet, name, thing)",
    );
    expect(swiftL10n(entries)).toContain("public static func greet(name: String, thing: String)");
  });

  it("plural becomes android plurals and xcstrings variations", () => {
    const entries = entriesOf(
      { "mobile.left": "{who}: {n, plural, one {# try left} other {# tries left}}!" },
      { "mobile.left": "{who}: {n, plural, one {# Versuch übrig} other {# Versuche übrig}}!" },
    );
    const xml = androidStrings(entries, "en");
    expect(xml).toContain('<plurals name="mobile_left">');
    expect(xml).toContain('<item quantity="one">%1$s: %2$d try left!</item>');
    expect(xml).toContain('<item quantity="other">%1$s: %2$d tries left!</item>');
    const xc = JSON.parse(xcstrings(entries)) as {
      strings: Record<string, { localizations: Record<string, { variations: unknown }> }>;
    };
    expect(xc.strings["mobile.left"]?.localizations["de"]?.variations).toEqual({
      plural: {
        one: { stringUnit: { state: "translated", value: "%1$@: %2$lld Versuch übrig!" } },
        other: { stringUnit: { state: "translated", value: "%1$@: %2$lld Versuche übrig!" } },
      },
    });
    expect(kotlinL10n(entries)).toContain(
      "r.getQuantityString(R.plurals.mobile_left, n, who, n)",
    );
    expect(kotlinL10n(entries)).toContain("fun left(r: Resources, who: String, n: Int): String");
    expect(swiftL10n(entries)).toContain("public static func left(who: String, n: Int) -> String");
  });

  it("android special characters are escaped", () => {
    expect(escapeAndroidText(`It's "q" & <b> \\ 50%\nx`, false)).toBe(
      `It\\'s \\"q\\" &amp; &lt;b> \\\\ 50%\\nx`,
    );
    const entries = entriesOf(
      {
        "mobile.at": "@home {x}",
        "mobile.q": "?help",
        "mobile.pct": "100% sure",
        "mobile.pctArg": "{x}% done",
      },
      {
        "mobile.at": "@Zuhause {x}",
        "mobile.q": "?Hilfe",
        "mobile.pct": "100% sicher",
        "mobile.pctArg": "{x}% fertig",
      },
    );
    const xml = androidStrings(entries, "en");
    expect(xml).toContain('<string name="mobile_at">\\@home %1$s</string>');
    expect(xml).toContain('<string name="mobile_q">\\?help</string>');
    // No arguments: never formatted, so the % stays single and the resource says so.
    expect(xml).toContain('<string name="mobile_pct" formatted="false">100% sure</string>');
    expect(xml).toContain('<string name="mobile_pctArg">%1$s%% done</string>');
  });

  it("select and nesting fail with the key name", () => {
    const fails = (message: string): string => {
      try {
        entriesOf({ "mobile.bad": message }, { "mobile.bad": message });
      } catch (error) {
        return (error as Error).message;
      }
      return "";
    };
    expect(fails("{k, select, a {A} other {B}}")).toMatch(/^mobile\.bad: "select"/);
    expect(fails("{n, selectordinal, one {#st} other {#th}}")).toMatch(/^mobile\.bad: "selectordinal"/);
    expect(fails("{n, plural, one {{m, plural, one {a} other {b}}} other {c}}")).toMatch(
      /^mobile\.bad: plural inside a plural/,
    );
    expect(fails("{n, plural, =0 {none} other {#}}")).toMatch(/^mobile\.bad: plural selector "=0"/);
    expect(fails("{n, plural, one {#}}")).toMatch(/^mobile\.bad: the plural has no "other"/);
    expect(fails("{a, plural, one {#} other {#}} {b, plural, one {#} other {#}}")).toMatch(
      /^mobile\.bad: more than one plural/,
    );
    expect(fails("{n, number}")).toMatch(/^mobile\.bad: "number"/);
    expect(fails("oops }")).toMatch(/^mobile\.bad: unbalanced/);
  });

  it("resource names are unique", () => {
    const names = mobileKeys(en, mobileShared).map((k) => k.replaceAll(".", "_"));
    expect(new Set(names).size).toBe(names.length);
    const accessors = buildEntries(en, de, mobileKeys(en, mobileShared)).map((e) =>
      accessorName(e.key),
    );
    expect(new Set(accessors).size).toBe(accessors.length);
    // `a.b` and `aB` would both become the accessor `aB`.
    expect(() => entriesOf({ "mobile.a.b": "x", "mobile.aB": "y" }, { "mobile.a.b": "x", "mobile.aB": "y" })).toThrow(
      /clashes/,
    );
  });

  it("de and en have identical mobile keys and arguments", () => {
    const own = (catalog: Record<string, string>): string[] =>
      Object.keys(catalog).filter((k) => k.startsWith("mobile.")).sort();
    expect(own(de)).toEqual(own(en));
    // buildEntries throws when a pair uses different arguments or plural kinds.
    const entries = buildEntries(en, de, mobileKeys(en, mobileShared));
    for (const entry of entries) {
      expect(entry.messages.de.args.slice().sort(), entry.key).toEqual(
        entry.messages.en.args.slice().sort(),
      );
    }
    expect(() => entriesOf({ "mobile.x": "{a}" }, { "mobile.x": "{b}" })).toThrow(/^mobile\.x: DE and EN/);
  });

  it("every mobileShared key exists", () => {
    for (const key of mobileShared) {
      expect(Object.keys(en), key).toContain(key);
      expect(Object.keys(de), key).toContain(key);
    }
    // The code list is the one of `MobileError::code()` in crates/mobile/src/error.rs.
    const here = dirname(fileURLToPath(import.meta.url));
    const rust = readFileSync(join(here, "..", "..", "..", "crates", "mobile", "src", "error.rs"), "utf8");
    const codes = [...rust.matchAll(/^\s+[A-Z][A-Za-z0-9]*\s*=>\s*"([A-Za-z0-9]+)",?\s*$/gm)].map(
      (m) => m[1],
    );
    expect([...mobileErrorCodes]).toEqual(codes);
  });
});
