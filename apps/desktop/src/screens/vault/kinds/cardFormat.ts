/**
 * Payment card helpers: the number's checks and grouping, the brand from its prefix and the
 * expiry format core stores (`MM/YYYY`). The hints never block a save; only an expiry that
 * cannot be turned into `MM/YYYY` does, because the stored format matters.
 */

/** The number without spaces and dashes. */
export function normalizeCardNumber(value: string): string {
  return value.replace(/[\s-]/g, "");
}

/** The Luhn check of a string of digits. */
export function luhnValid(digits: string): boolean {
  if (!/^\d+$/.test(digits)) {
    return false;
  }
  let sum = 0;
  let double = false;
  for (let index = digits.length - 1; index >= 0; index -= 1) {
    let digit = Number(digits[index]);
    if (double) {
      digit *= 2;
      if (digit > 9) {
        digit -= 9;
      }
    }
    sum += digit;
    double = !double;
  }
  return sum % 10 === 0;
}

/** What is wrong with a typed number, if anything: the length (12 to 19 digits) or the check. */
export function cardNumberHint(value: string): "length" | "luhn" | null {
  const digits = normalizeCardNumber(value);
  if (digits === "") {
    return null;
  }
  if (!/^\d{12,19}$/.test(digits)) {
    return "length";
  }
  return luhnValid(digits) ? null : "luhn";
}

function isAmex(digits: string): boolean {
  return digits.startsWith("34") || digits.startsWith("37");
}

/** Groups 4-6-5 for American Express, otherwise in fours. */
export function groupCardNumber(digits: string): string {
  const groups: string[] = [];
  let at = 0;
  for (const size of isAmex(digits) ? [4, 6, 5] : []) {
    if (at < digits.length) {
      groups.push(digits.slice(at, at + size));
      at += size;
    }
  }
  while (at < digits.length) {
    groups.push(digits.slice(at, at + 4));
    at += 4;
  }
  return groups.join(" ");
}

function isMastercard(digits: string): boolean {
  const two = digits.length >= 2 ? Number(digits.slice(0, 2)) : 0;
  const four = digits.length >= 4 ? Number(digits.slice(0, 4)) : 0;
  return (two >= 51 && two <= 55) || (four >= 2221 && four <= 2720);
}

/** The brand by prefix, or `null`. */
export function brandFromNumber(value: string): string | null {
  const digits = normalizeCardNumber(value);
  if (digits.startsWith("4")) {
    return "Visa";
  }
  if (isMastercard(digits)) {
    return "Mastercard";
  }
  if (isAmex(digits)) {
    return "American Express";
  }
  if (digits.startsWith("6011") || digits.startsWith("65")) {
    return "Discover";
  }
  return digits.startsWith("35") ? "JCB" : null;
}

/**
 * `MM/YY`, `MMYY`, `M/YY`, `MM/YYYY` or `MMYYYY` as core's `MM/YYYY` (YY becomes 20YY);
 * `null` for anything else or a month outside 01 to 12.
 */
export function normalizeExpiry(value: string): string | null {
  const text = value.trim();
  const match = /^(\d{1,2})\s*\/\s*(\d{2}|\d{4})$/.exec(text) ?? /^(\d{2})(\d{4}|\d{2})$/.exec(text);
  const month = match?.[1];
  const year = match?.[2];
  if (month === undefined || year === undefined) {
    return null;
  }
  const number = Number(month);
  if (number < 1 || number > 12) {
    return null;
  }
  return `${String(number).padStart(2, "0")}/${year.length === 2 ? `20${year}` : year}`;
}

/** Whether a stored `MM/YYYY` expiry is over (a card is good through the end of its month). */
export function isExpired(expiry: string, now: Date): boolean {
  const match = /^(\d{2})\/(\d{4})$/.exec(expiry);
  const month = Number(match?.[1]);
  const year = Number(match?.[2]);
  if (match === null || month < 1 || month > 12) {
    return false;
  }
  return year < now.getFullYear() || (year === now.getFullYear() && month < now.getMonth() + 1);
}
