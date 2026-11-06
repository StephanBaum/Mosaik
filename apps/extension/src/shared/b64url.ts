/**
 * base64url without padding (RFC 4648 section 5), written out so the page shim and the bridge
 * depend on no global a page could replace (`atob`, `btoa`, `TextEncoder`).
 */
const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const LOOKUP: Record<string, number> = {};
for (let i = 0; i < ALPHABET.length; i++) {
  LOOKUP[ALPHABET.charAt(i)] = i;
}
const SHAPE = /^[A-Za-z0-9_-]*$/;

export function encodeB64url(bytes: Uint8Array): string {
  let out = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const a = bytes[i] ?? 0;
    const b = bytes[i + 1] ?? 0;
    const c = bytes[i + 2] ?? 0;
    const triple = (a << 16) | (b << 8) | c;
    out += ALPHABET.charAt((triple >> 18) & 63) + ALPHABET.charAt((triple >> 12) & 63);
    if (i + 1 < bytes.length) {
      out += ALPHABET.charAt((triple >> 6) & 63);
    }
    if (i + 2 < bytes.length) {
      out += ALPHABET.charAt(triple & 63);
    }
  }
  return out;
}

/** The decoded size of a well-formed base64url string, or `null` (bad characters, impossible length). */
export function b64urlByteLength(value: string): number | null {
  if (!SHAPE.test(value) || value.length % 4 === 1) {
    return null;
  }
  return Math.floor((value.length * 3) / 4);
}

/** Decodes base64url; `null` for anything that is not canonical (padding, other characters, stray bits). */
export function decodeB64url(value: string): Uint8Array | null {
  const size = b64urlByteLength(value);
  if (size === null) {
    return null;
  }
  const out = new Uint8Array(size);
  let bits = 0;
  let acc = 0;
  let at = 0;
  for (const char of value) {
    acc = (acc << 6) | (LOOKUP[char] ?? 0);
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      out[at++] = (acc >> bits) & 255;
      acc &= (1 << bits) - 1;
    }
  }
  return acc === 0 ? out : null;
}

/** A base64url string whose decoded size is within `min..max` bytes. */
export const isB64urlBytes =
  (min: number, max: number) =>
  (value: unknown): value is string => {
    if (typeof value !== "string") {
      return false;
    }
    const size = b64urlByteLength(value);
    return size !== null && size >= min && size <= max && decodeB64url(value) !== null;
  };
