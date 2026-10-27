/**
 * Mosaic UI string catalogs. All user-facing text comes from here
 * (docs/UX_GUIDELINES.md §6). Messages are ICU MessageFormat, formatted with
 * intl-messageformat (FormatJS, BSD-3-Clause; ADR-0019).
 */
import { IntlMessageFormat } from "intl-messageformat";
import { de } from "./de.ts";
import { en, type MessageKey } from "./en.ts";

export type { MessageKey };

export const locales = ["en", "de"] as const;
export type Locale = (typeof locales)[number];

export const catalogs: Readonly<Record<Locale, Readonly<Record<MessageKey, string>>>> = { en, de };

/** Values for a message's ICU arguments. */
export type MessageValues = Readonly<Record<string, string | number | Date>>;

const formatters = new Map<string, IntlMessageFormat>();

/**
 * Formats `key` in `locale` with ICU plural/select rules and `Intl` number and date
 * formatting. Throws if a value the message needs is missing.
 */
export function formatMessage(locale: Locale, key: MessageKey, values?: MessageValues): string {
  const id = `${locale}:${key}`;
  let formatter = formatters.get(id);
  if (formatter === undefined) {
    formatter = new IntlMessageFormat(catalogs[locale][key], locale);
    formatters.set(id, formatter);
  }
  const result = formatter.format(values);
  return Array.isArray(result) ? result.join("") : String(result);
}

/** Pick the best supported locale for a BCP 47 preference list; English is the fallback. */
export function negotiateLocale(preferred: readonly string[]): Locale {
  for (const tag of preferred) {
    const base = tag.toLowerCase().split("-")[0];
    const match = locales.find((locale) => locale === base);
    if (match) {
      return match;
    }
  }
  return "en";
}

/**
 * Names of all ICU arguments in a message, including arguments nested in plural/select
 * branches, e.g. `{n, plural, one {# for {who}}}` → `n`, `who`. `#` is not an argument.
 * Apostrophe quoting is not used in Mosaic catalogs and is not interpreted.
 */
export function icuArguments(message: string): Set<string> {
  const names = new Set<string>();
  const end = parseMessage(message, 0, names, false);
  if (end !== message.length) {
    throw new Error(`Unbalanced "}" in message: ${message}`);
  }
  return names;
}

const branchingTypes = new Set(["plural", "select", "selectordinal"]);

// Parses message text from `i`; returns the index of the closing "}" (nested) or the end.
function parseMessage(s: string, i: number, names: Set<string>, nested: boolean): number {
  while (i < s.length) {
    const ch = s[i];
    if (ch === "{") {
      i = parseArgument(s, i + 1, names);
    } else if (ch === "}") {
      if (nested) {
        return i;
      }
      throw new Error(`Unbalanced "}" in message: ${s}`);
    } else {
      i++;
    }
  }
  if (nested) {
    throw new Error(`Unbalanced "{" in message: ${s}`);
  }
  return i;
}

// Parses `name[, type[, style | branches]]}` starting after "{"; returns the index after "}".
function parseArgument(s: string, i: number, names: Set<string>): number {
  const header = /^\s*([A-Za-z_]\w*)\s*(?:,\s*(\w+)\s*)?([,}])/.exec(s.slice(i));
  if (!header) {
    throw new Error(`Unbalanced or malformed "{" in message: ${s}`);
  }
  const [matched, name, type, terminator] = header as unknown as [
    string,
    string,
    string | undefined,
    string,
  ];
  names.add(name);
  i += matched.length;
  if (terminator === "}") {
    return i;
  }
  if (type !== undefined && branchingTypes.has(type)) {
    // Branches: selector {message} … until the closing "}".
    for (;;) {
      while (i < s.length && /\s/.test(s[i] ?? "")) {
        i++;
      }
      if (i >= s.length) {
        throw new Error(`Unbalanced "{" in message: ${s}`);
      }
      if (s[i] === "}") {
        return i + 1;
      }
      const selector = /^[^\s{}]+/.exec(s.slice(i));
      if (!selector) {
        throw new Error(`Malformed branch in message: ${s}`);
      }
      i += selector[0].length;
      while (i < s.length && /\s/.test(s[i] ?? "")) {
        i++;
      }
      if (selector[0].includes(":")) {
        continue; // e.g. offset:1
      }
      if (s[i] !== "{") {
        throw new Error(`Expected "{" after branch selector in message: ${s}`);
      }
      i = parseMessage(s, i + 1, names, true) + 1;
    }
  }
  // Simple style such as `{d, date, short}`: skip to the closing "}".
  const close = s.indexOf("}", i);
  if (close === -1) {
    throw new Error(`Unbalanced "{" in message: ${s}`);
  }
  return close + 1;
}
