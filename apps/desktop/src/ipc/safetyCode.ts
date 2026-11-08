/**
 * Safety codes (CRYPTO_SPEC §8.1): 26 Crockford Base32 characters in groups `5-5-5-5-6`.
 * The comparison on screen ignores case, spaces and hyphens and accepts the Crockford aliases
 * (`I` and `L` for `1`, `O` for `0`), like `Fingerprint::matches_code` in Rust. A code is a
 * public value; this only helps people compare it. The backend checks the code again.
 */

/** The code without separators, upper case, aliases resolved. */
export function normalizeCode(code: string): string {
  return Array.from(code.toUpperCase())
    .filter((char) => !/[\s-]/u.test(char))
    .map((char) => (char === "I" || char === "L" ? "1" : char === "O" ? "0" : char))
    .join("");
}

/** Whether two texts are the same safety code. */
export function sameCode(a: string, b: string): boolean {
  const left = normalizeCode(a);
  return left !== "" && left === normalizeCode(b);
}

const GROUPS = [5, 5, 5, 5, 6] as const;

/** The code in its display groups (`XXXXX XXXXX XXXXX XXXXX XXXXXX`); other input is returned as is. */
export function groupCode(code: string): string[] {
  const flat = normalizeCode(code);
  if (flat.length !== GROUPS.reduce((sum, size) => sum + size, 0)) {
    return [code];
  }
  const out: string[] = [];
  let at = 0;
  for (const size of GROUPS) {
    out.push(flat.slice(at, at + size));
    at += size;
  }
  return out;
}
