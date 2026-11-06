import { describe, expect, it } from "vitest";
import { b64urlByteLength, decodeB64url, encodeB64url, isB64urlBytes } from "./b64url.ts";

describe("base64url", () => {
  it.each([0, 1, 2, 3, 4, 5, 16, 31, 32, 255, 1023])("round trips %i bytes and agrees with Node", (n) => {
    const bytes = Uint8Array.from({ length: n }, (_, i) => (i * 37 + 11) & 255);
    const text = encodeB64url(bytes);
    expect(text).toBe(Buffer.from(bytes).toString("base64url"));
    expect(decodeB64url(text)).toEqual(bytes);
    expect(b64urlByteLength(text)).toBe(n);
  });

  it.each(["AA==", "A", "AAAAA", "AB", "+/+/", "AA A", "é", " AAAA"])("refuses %j", (text) => {
    expect(decodeB64url(text)).toBeNull();
  });

  it("refuses stray trailing bits (a non-canonical form)", () => {
    expect(decodeB64url("AB")).toBeNull();
    expect(decodeB64url("AA")).toEqual(new Uint8Array(1));
  });

  it("checks the decoded size against the limits", () => {
    const check = isB64urlBytes(16, 32);
    expect(check(encodeB64url(new Uint8Array(15)))).toBe(false);
    expect(check(encodeB64url(new Uint8Array(16)))).toBe(true);
    expect(check(encodeB64url(new Uint8Array(32)))).toBe(true);
    expect(check(encodeB64url(new Uint8Array(33)))).toBe(false);
    expect(check(5)).toBe(false);
    expect(check("!!!!")).toBe(false);
  });
});
