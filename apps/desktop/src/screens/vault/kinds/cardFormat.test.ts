import { describe, expect, it } from "vitest";
import {
  brandFromNumber,
  cardNumberHint,
  groupCardNumber,
  isExpired,
  luhnValid,
  normalizeCardNumber,
  normalizeExpiry,
} from "./cardFormat.ts";

describe("card number", () => {
  it("removes spaces and dashes", () => {
    expect(normalizeCardNumber("4111 1111-1111 1111")).toBe("4111111111111111");
  });

  it("checks Luhn", () => {
    expect(luhnValid("4111111111111111")).toBe(true);
    expect(luhnValid("4111111111111112")).toBe(false);
    expect(luhnValid("378282246310005")).toBe(true);
    expect(luhnValid("")).toBe(false);
    expect(luhnValid("41a1")).toBe(false);
  });

  it("hints at the length first, then the check, and never for an empty field", () => {
    expect(cardNumberHint("")).toBeNull();
    expect(cardNumberHint("4111")).toBe("length");
    expect(cardNumberHint("4111 1111 1111 1112")).toBe("luhn");
    expect(cardNumberHint("4111 1111 1111 1111")).toBeNull();
    expect(cardNumberHint("41111111111111111111")).toBe("length");
  });

  it("groups American Express 4-6-5 and everything else in fours", () => {
    expect(groupCardNumber("378282246310005")).toBe("3782 822463 10005");
    expect(groupCardNumber("4111111111111111")).toBe("4111 1111 1111 1111");
    expect(groupCardNumber("41111")).toBe("4111 1");
    expect(groupCardNumber("3782822")).toBe("3782 822");
  });

  it("finds the brand by prefix", () => {
    expect(brandFromNumber("4111111111111111")).toBe("Visa");
    expect(brandFromNumber("5105105105105100")).toBe("Mastercard");
    expect(brandFromNumber("2221000000000009")).toBe("Mastercard");
    expect(brandFromNumber("2720990000000007")).toBe("Mastercard");
    expect(brandFromNumber("2721")).toBeNull();
    expect(brandFromNumber("378282246310005")).toBe("American Express");
    expect(brandFromNumber("340000000000009")).toBe("American Express");
    expect(brandFromNumber("6011111111111117")).toBe("Discover");
    expect(brandFromNumber("6500000000000002")).toBe("Discover");
    expect(brandFromNumber("3530111333300000")).toBe("JCB");
    expect(brandFromNumber("9")).toBeNull();
    expect(brandFromNumber("")).toBeNull();
  });
});

describe("card expiry", () => {
  it.each([
    ["04/29", "04/2029"],
    ["0429", "04/2029"],
    ["4/29", "04/2029"],
    ["04/2029", "04/2029"],
    ["042029", "04/2029"],
    [" 12 / 31 ", "12/2031"],
  ])("normalizes %s", (input, expected) => {
    expect(normalizeExpiry(input)).toBe(expected);
  });

  it.each(["13/29", "00/29", "4", "04-29", "abcd", "04/2", ""])("refuses %s", (input) => {
    expect(normalizeExpiry(input)).toBeNull();
  });

  it("is expired after the end of its month", () => {
    expect(isExpired("04/2029", new Date(2029, 3, 30))).toBe(false);
    expect(isExpired("04/2029", new Date(2029, 4, 1))).toBe(true);
    expect(isExpired("04/2029", new Date(2028, 11, 31))).toBe(false);
    expect(isExpired("not a date", new Date(2040, 0, 1))).toBe(false);
  });
});
