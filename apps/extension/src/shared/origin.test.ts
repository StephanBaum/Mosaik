import { describe, expect, it } from "vitest";
import { hostOf, isPageOrigin, isSecureOrigin, originOf, unicodeHostOf, sameSite, siteOf } from "./origin.ts";

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

describe("siteOf and sameSite", () => {
  it("guesses the registrable domain", () => {
    expect(siteOf("accounts.example.com")).toBe("example.com");
    expect(siteOf("example.com")).toBe("example.com");
    expect(siteOf("login.example.co.uk")).toBe("example.co.uk");
    expect(siteOf("example.com.au")).toBe("example.com.au");
  });

  it("has none for addresses, single labels and malformed hosts", () => {
    for (const host of ["localhost", "127.0.0.1", "[::1]", "::1", "", "a..b", "example."]) {
      expect(siteOf(host)).toBeNull();
    }
  });

  it("calls two https origins on one site related, and nothing else", () => {
    expect(sameSite("https://example.com", "https://accounts.example.com")).toBe(true);
    expect(sameSite("https://login.example.com", "https://accounts.example.com:8443")).toBe(true);
    expect(sameSite("https://example.com", "http://accounts.example.com")).toBe(false);
    expect(sameSite("https://example.com", "https://example.com.evil.test")).toBe(false);
    expect(sameSite("https://example.com", "https://evil-example.com")).toBe(false);
    expect(sameSite("https://co.uk", "https://evil.co.uk")).toBe(false);
    expect(sameSite("https://a.example.co.uk", "https://b.example.co.uk")).toBe(true);
    expect(sameSite("https://localhost", "https://localhost")).toBe(false);
    expect(sameSite("https://127.0.0.1", "https://127.0.0.1")).toBe(false);
  });
});
