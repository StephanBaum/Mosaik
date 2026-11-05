import { describe, expect, it } from "vitest";
import { hostOf, isPageOrigin, isSecureOrigin, originOf, unicodeHostOf } from "./origin.ts";

describe("originOf", () => {
  it.each([
    ["https://example.com/path?q=1#frag", "https://example.com"],
    ["http://example.com:8080/x", "http://example.com:8080"],
    ["https://example.com:443/", "https://example.com"],
    ["HTTPS://EXAMPLE.COM/", "https://example.com"],
    ["https://sub.example.co.uk/login", "https://sub.example.co.uk"],
  ])("%s -> %s", (url, origin) => {
    expect(originOf(url)).toBe(origin);
  });

  it("takes the host after the userinfo, so a lookalike cannot hide behind a name", () => {
    expect(originOf("https://bank.example@evil.example/")).toBe("https://evil.example");
    expect(originOf("https://user:pw@evil.example:8443/")).toBe("https://evil.example:8443");
  });

  it.each([
    undefined,
    "",
    "not a url",
    "about:blank",
    "about:srcdoc",
    "data:text/html,<p>",
    "blob:https://example.com/uuid",
    "file:///etc/passwd",
    "chrome://settings",
    "chrome-extension://abc/popup.html",
    "javascript:alert(1)",
    "ftp://example.com/",
  ])("%s is not a page origin", (url) => {
    expect(originOf(url)).toBeNull();
  });
});

describe("isPageOrigin, isSecureOrigin, hostOf", () => {
  it("accepts only canonical origins", () => {
    expect(isPageOrigin("https://example.com")).toBe(true);
    expect(isPageOrigin("https://example.com/")).toBe(false);
    expect(isPageOrigin("null")).toBe(false);
  });

  it("tells https from http", () => {
    expect(isSecureOrigin("https://example.com")).toBe(true);
    expect(isSecureOrigin("http://example.com")).toBe(false);
  });

  it("returns the host without scheme and port", () => {
    expect(hostOf("https://example.com:8443")).toBe("example.com");
    expect(hostOf("nonsense")).toBe("");
  });
});

describe("unicodeHostOf", () => {
  it("decodes punycode labels", () => {
    expect(unicodeHostOf("xn--bcher-kva.example")).toBe("bücher.example");
    expect(unicodeHostOf("xn--80ak6aa92e.com")).toBe("аррӏе.com");
    expect(unicodeHostOf("www.xn--mnchen-3ya.de")).toBe("www.münchen.de");
  });

  it("matches what the URL parser produces for a homoglyph host", () => {
    const ascii = new URL("https://bаnk.test").hostname;
    expect(ascii.startsWith("xn--")).toBe(true);
    expect(unicodeHostOf(ascii)).toBe("bаnk.test");
  });

  it("is null for hosts without punycode and for malformed labels", () => {
    expect(unicodeHostOf("example.com")).toBeNull();
    expect(unicodeHostOf("xn--!!.example")).toBeNull();
  });
});
