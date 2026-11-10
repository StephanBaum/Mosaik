/**
 * Page origins as the browser serializes them. Matching is the desktop's job (mosaic-origin);
 * the extension only parses a tab or frame URL into an origin to report, and shows the
 * Unicode form of a punycode host.
 */

/** The ASCII origin of an http(s) URL, or null for anything else (about:, file:, data:, blob:, ...). */
export function originOf(url: string | undefined): string | null {
  if (url === undefined) {
    return null;
  }
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
      return null;
    }
    return parsed.origin;
  } catch {
    return null;
  }
}

/** Whether `origin` (as returned by `originOf`) is a valid page origin the extension may act on. */
export function isPageOrigin(origin: string): boolean {
  return originOf(origin) === origin;
}

export function isSecureOrigin(origin: string): boolean {
  return origin.startsWith("https://");
}

/** The host of an origin string, without scheme and port. */
export function hostOf(origin: string): string {
  try {
    return new URL(origin).hostname;
  } catch {
    return "";
  }
}

/**
 * A guess at the registrable domain of a host, without the public suffix list: the last two
 * labels, or the last three when the second-level label of a two-letter country domain is short
 * (`example.co.uk`, `example.com.au`). IP addresses and single labels have none. Only a second
 * opinion (the desktop decides with the list): it errs towards "not related".
 */
export function siteOf(host: string): string | null {
  if (host.includes(":") || host.includes("[") || /^[\d.]+$/.test(host)) {
    return null;
  }
  const labels = host.toLowerCase().split(".");
  if (labels.length < 2 || labels.some((label) => label === "")) {
    return null;
  }
  const tld = labels.at(-1) ?? "";
  const second = labels.at(-2) ?? "";
  const keep = tld.length === 2 && second.length <= 3 && labels.length >= 3 ? 3 : 2;
  return labels.slice(-keep).join(".");
}

/** Whether two origins are https and on the same site (the same guessed registrable domain). */
export function sameSite(a: string, b: string): boolean {
  if (!isSecureOrigin(a) || !isSecureOrigin(b)) {
    return false;
  }
  const first = siteOf(hostOf(a));
  return first !== null && first === siteOf(hostOf(b));
}

/** Whether two origins name the same port (the default port of https counts as one). */
export function samePort(a: string, b: string): boolean {
  try {
    return new URL(a).port === new URL(b).port;
  } catch {
    return false;
  }
}

// RFC 3492 bootstring parameters.
const BASE = 36;
const T_MIN = 1;
const T_MAX = 26;
const SKEW = 38;
const DAMP = 700;
const INITIAL_BIAS = 72;
const INITIAL_N = 128;

function adapt(delta: number, points: number, first: boolean): number {
  let d = first ? Math.floor(delta / DAMP) : delta >> 1;
  d += Math.floor(d / points);
  let k = 0;
  while (d > ((BASE - T_MIN) * T_MAX) >> 1) {
    d = Math.floor(d / (BASE - T_MIN));
    k += BASE;
  }
  return k + Math.floor(((BASE - T_MIN + 1) * d) / (d + SKEW));
}

function digitValue(code: number): number {
  if (code >= 0x30 && code <= 0x39) {
    return code - 0x30 + 26;
  }
  if (code >= 0x61 && code <= 0x7a) {
    return code - 0x61;
  }
  return -1;
}

/** Decodes one punycode label (without the `xn--` prefix); null if it is malformed. */
function punycodeDecode(input: string): string | null {
  const split = input.lastIndexOf("-");
  const output: number[] = Array.from(input.slice(0, Math.max(split, 0)), (c) => c.codePointAt(0) ?? 0);
  let n = INITIAL_N;
  let i = 0;
  let bias = INITIAL_BIAS;
  let index = split > 0 ? split + 1 : 0;
  while (index < input.length) {
    const oldI = i;
    let w = 1;
    for (let k = BASE; ; k += BASE) {
      const digit = digitValue(input.charCodeAt(index++));
      if (digit < 0) {
        return null;
      }
      i += digit * w;
      const t = k <= bias ? T_MIN : Math.min(Math.max(k - bias, T_MIN), T_MAX);
      if (digit < t) {
        break;
      }
      w *= BASE - t;
    }
    bias = adapt(i - oldI, output.length + 1, oldI === 0);
    n += Math.floor(i / (output.length + 1));
    i %= output.length + 1;
    if (n > 0x10ffff) {
      return null;
    }
    output.splice(i++, 0, n);
  }
  return String.fromCodePoint(...output);
}

/**
 * The host with every `xn--` label shown in Unicode, or null when no label is punycode
 * (or one is malformed, in which case the caller keeps showing the ASCII host).
 */
export function unicodeHostOf(host: string): string | null {
  const labels = host.split(".");
  if (!labels.some((label) => label.startsWith("xn--"))) {
    return null;
  }
  const decoded: string[] = [];
  for (const label of labels) {
    const text = label.startsWith("xn--") ? punycodeDecode(label.slice(4)) : label;
    if (text === null) {
      return null;
    }
    decoded.push(text);
  }
  return decoded.join(".");
}
