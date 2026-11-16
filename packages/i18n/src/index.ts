/**
 * Mosaic UI string catalogs. All user-facing text comes from here
 * (docs/UX_GUIDELINES.md §6). Messages are ICU MessageFormat, formatted with
 * intl-messageformat (FormatJS, BSD-3-Clause; ADR-0019).
 */
import { IntlMessageFormat } from "intl-messageformat";
import { de } from "./de.ts";
import { en, type MessageKey } from "./en.ts";
import { es } from "./es.ts";
import { fr } from "./fr.ts";
import { it } from "./it.ts";
import { ja } from "./ja.ts";
import { ko } from "./ko.ts";
import type { Locale } from "./locales.ts";
import { nl } from "./nl.ts";
import { pl } from "./pl.ts";
import { pt } from "./pt.ts";
import { tr } from "./tr.ts";
import { zh } from "./zh.ts";

export type { MessageKey };
export {
  isMachineTranslated,
  localeNames,
  localeOf,
  locales,
  negotiateLocale,
  pluralCategories,
  reviewedLocales,
  type Locale,
} from "./locales.ts";

export const catalogs: Readonly<Record<Locale, Readonly<Record<MessageKey, string>>>> = {
  en,
  de,
  es,
  fr,
  it,
  pt,
  nl,
  pl,
  tr,
  ja,
  ko,
  zh,
};

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

/**
 * Names of all ICU arguments in a message, including arguments nested in plural/select
 * branches, e.g. `{n, plural, one {# for {who}}}` → `n`, `who`. `#` is not an argument.
 * Apostrophe quoting is not used in Mosaic catalogs and is not interpreted.
 */
export function icuArguments(message: string): Set<string> {
  return parseIcu(message).names;
}

/**
 * The branch selectors of every `plural` in a message, at any depth, e.g.
 * `{n, plural, =0 {none} one {#} other {#}}` → `[["=0", "one", "other"]]`.
 */
export function icuPluralSelectors(message: string): string[][] {
  return parseIcu(message).plurals;
}

interface Found {
  names: Set<string>;
  plurals: string[][];
}

function parseIcu(message: string): Found {
  const found: Found = { names: new Set<string>(), plurals: [] };
  const end = parseMessage(message, 0, found, false);
  if (end !== message.length) {
    throw new Error(`Unbalanced "}" in message: ${message}`);
  }
  return found;
}

const branchingTypes = new Set(["plural", "select", "selectordinal"]);

// Parses message text from `i`; returns the index of the closing "}" (nested) or the end.
function parseMessage(s: string, i: number, found: Found, nested: boolean): number {
  while (i < s.length) {
    const ch = s[i];
    if (ch === "{") {
      i = parseArgument(s, i + 1, found);
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

const headerPattern = /^\s*([A-Za-z_]\w*)\s*(?:,\s*(\w+)\s*)?([,}])/;

interface ArgumentHeader {
  name: string;
  type: string | undefined;
  terminator: string;
  length: number;
}

function parseHeader(s: string, i: number): ArgumentHeader {
  const match = headerPattern.exec(s.slice(i));
  if (!match) {
    throw new Error(`Unbalanced or malformed "{" in message: ${s}`);
  }
  const [matched, name, type, terminator] = match as unknown as [
    string,
    string,
    string | undefined,
    string,
  ];
  return { name, type, terminator, length: matched.length };
}

function skipSpace(s: string, i: number): number {
  let j = i;
  while (j < s.length && /\s/.test(s[j] ?? "")) {
    j++;
  }
  return j;
}

// Parses one `selector {message}` (or `offset:n`) at `i`; returns the index after it. The
// selector goes to `selectors`.
function parseBranch(s: string, i: number, found: Found, selectors: string[]): number {
  if (i >= s.length) {
    throw new Error(`Unbalanced "{" in message: ${s}`);
  }
  const selector = /^[^\s{}]+/.exec(s.slice(i));
  if (!selector) {
    throw new Error(`Malformed branch in message: ${s}`);
  }
  const next = skipSpace(s, i + selector[0].length);
  if (selector[0].includes(":")) {
    return next; // e.g. offset:1
  }
  if (s[next] !== "{") {
    throw new Error(`Expected "{" after branch selector in message: ${s}`);
  }
  selectors.push(selector[0]);
  return parseMessage(s, next + 1, found, true) + 1;
}

// Branches until the closing "}"; returns the index after it. A plural's selectors are recorded.
function parseBranches(s: string, start: number, found: Found, plural: boolean): number {
  const selectors: string[] = [];
  if (plural) {
    found.plurals.push(selectors);
  }
  let i = skipSpace(s, start);
  while (s[i] !== "}") {
    i = skipSpace(s, parseBranch(s, i, found, selectors));
  }
  return i + 1;
}

// Parses `name[, type[, style | branches]]}` starting after "{"; returns the index after "}".
function parseArgument(s: string, i: number, found: Found): number {
  const header = parseHeader(s, i);
  found.names.add(header.name);
  const next = i + header.length;
  if (header.terminator === "}") {
    return next;
  }
  if (header.type !== undefined && branchingTypes.has(header.type)) {
    return parseBranches(s, next, found, header.type === "plural");
  }
  // Simple style such as `{d, date, short}`: skip to the closing "}".
  const close = s.indexOf("}", next);
  if (close === -1) {
    throw new Error(`Unbalanced "{" in message: ${s}`);
  }
  return close + 1;
}
