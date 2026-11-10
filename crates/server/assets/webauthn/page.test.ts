/// <reference types="node" />
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cssVariables } from "../../../../packages/design-tokens/src/css.ts";
import { webauthn as de } from "../../../../packages/i18n/src/catalog/de/webauthn.ts";
import { webauthn as en } from "../../../../packages/i18n/src/catalog/en/webauthn.ts";
import { start } from "./page.js";

const here = dirname(fileURLToPath(import.meta.url));
const read = (name: string): string =>
  readFileSync(join(here, name), "utf8").replaceAll("\r\n", "\n");
const html = read("index.html");
const stringsJson = read("strings.json");

const ID = "AAECAwQFBgcICQoLDA0ODw";
const EVIL = '<img src=x onerror="window.pwned=1">';

type Reply = { status: number; body?: unknown };
type Call = { path: string; body: unknown };

function mount(): void {
  const parsed = new DOMParser().parseFromString(html, "text/html");
  document.body.innerHTML = parsed.body.innerHTML;
}

function el(id: string): HTMLElement {
  const found = document.getElementById(id);
  if (found === null) throw new Error(`no #${id}`);
  return found;
}

/** A fetch that answers the strings itself and the POSTs from `replies`, in order, per path. */
function server(replies: Record<string, Reply[]>): { fetch: typeof fetch; calls: Call[] } {
  const calls: Call[] = [];
  const queue = new Map(Object.entries(replies).map(([k, v]) => [k, [...v]]));
  const fake = (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const path = typeof input === "string" ? input : input instanceof URL ? input.pathname : "";
    if (path === "/webauthn/strings.json") return Promise.resolve(new Response(stringsJson));
    const body: unknown = typeof init?.body === "string" ? JSON.parse(init.body) : null;
    calls.push({ path, body });
    const reply = queue.get(path)?.shift() ?? { status: 500 };
    return Promise.resolve(new Response(JSON.stringify(reply.body ?? null), { status: reply.status }));
  };
  return { fetch: fake, calls };
}

const options = (purpose: string, deviceName = "Alice's laptop") => ({
  status: 200,
  body: {
    purpose,
    deviceName,
    requestedAt: 1_800_000_000_000,
    publicKey: {
      challenge: "AAAA",
      user: { id: "AAAA", name: "alice@example.com", displayName: "alice@example.com" },
      excludeCredentials: [],
      allowCredentials: [{ type: "public-key", id: "AAAA" }],
    },
  },
});

const buf = (n: number): ArrayBuffer => new Uint8Array(n).fill(7).buffer;
const fakeCredential = {
  id: "cred",
  rawId: buf(4),
  type: "public-key",
  response: {
    clientDataJSON: buf(8),
    attestationObject: buf(8),
    authenticatorData: buf(8),
    signature: buf(8),
    userHandle: null,
    getTransports: () => ["usb"],
  },
};
const credentials = (create: () => Promise<unknown>): CredentialsContainer =>
  ({ create, get: create }) as unknown as CredentialsContainer;
const resolved = () => Promise.resolve(fakeCredential);

function type(code: string): void {
  const input = el("code") as HTMLInputElement;
  input.value = code;
  el("code-form").dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
}

afterEach(() => {
  document.body.innerHTML = "";
  history.replaceState(null, "", "/");
  vi.unstubAllGlobals();
  vi.resetModules();
});

describe("the security-key page", () => {
  it("reads_fragment_not_query", async () => {
    // The ID only in the query: the page does not use it.
    for (const [url, expectForm] of [
      [`/webauthn/?${ID}`, false],
      [`/webauthn/?h=${ID}`, false],
      [`/webauthn/#${ID}`, true],
    ] as const) {
      mount();
      history.replaceState(null, "", url);
      const s = server({ "/v1/webauthn/handoff/options": [{ status: 401 }] });
      vi.stubGlobal("fetch", s.fetch);
      vi.stubGlobal("navigator", { credentials: undefined, languages: ["en-US"] });
      vi.resetModules();
      await import("./page.js");
      await vi.waitFor(() => {
        expect(el("heading").textContent).not.toBe("");
      });
      expect(el("code-form").hidden).toBe(!expectForm);
      if (expectForm) {
        type("123456");
        await vi.waitFor(() => {
          expect(s.calls).toHaveLength(1);
        });
        expect(s.calls[0]).toEqual({
          path: "/v1/webauthn/handoff/options",
          body: { handoffId: ID, code: "123456" },
        });
      } else {
        expect(el("heading").textContent).toBe(en["webauthn.heading.invalid"]);
        expect(s.calls).toHaveLength(0);
      }
    }
  });

  it("renders_device_name_as_text", async () => {
    mount();
    const s = server({ "/v1/webauthn/handoff/options": [options("register", EVIL)] });
    await start({
      document,
      hash: `#${ID}`,
      fetch: s.fetch,
      credentials: credentials(() => new Promise(() => undefined)),
      languages: ["en"],
    });
    type("123456");
    await vi.waitFor(() => {
      expect(el("fact-device").textContent).toBe(EVIL);
    });
    expect(document.querySelector("img")).toBeNull();
    expect(document.querySelector("[onerror]")).toBeNull();
    expect(el("fact-account").textContent).toBe("alice@example.com");
    expect(el("heading").textContent).toBe(en["webauthn.heading.register"]);
    expect(el("status").textContent).toBe(en["webauthn.status.waiting"]);
    expect(el("code").getAttribute("value")).toBeNull();
  });

  it("shows_expired_and_failed_states", async () => {
    // 410 on the options: expired, with the way forward, in English and German.
    for (const [languages, strings] of [
      [["en-GB"], en],
      [["de-DE", "en"], de],
    ] as const) {
      mount();
      const s = server({ "/v1/webauthn/handoff/options": [{ status: 410 }] });
      await start({ document, hash: `#${ID}`, fetch: s.fetch, credentials: undefined, languages });
      type("123456");
      await vi.waitFor(() => {
        expect(el("heading").textContent).toBe(strings["webauthn.heading.expired"]);
      });
      expect(el("message").textContent).toBe(strings["webauthn.expired.body"]);
      expect(el("code-form").hidden).toBe(true);
      expect(document.documentElement.lang).toBe(languages[0].slice(0, 2));
    }

    // Wrong codes: a hint, then failed at the fifth.
    mount();
    const wrong = server({
      "/v1/webauthn/handoff/options": Array.from({ length: 5 }, () => ({ status: 401 })),
    });
    await start({ document, hash: `#${ID}`, fetch: wrong.fetch, credentials: undefined, languages: ["en"] });
    type("000000");
    await vi.waitFor(() => {
      expect(el("code-error").textContent).toBe(en["webauthn.code.invalid"]);
    });
    for (let n = 1; n < 5; n += 1) {
      type("000000");
      await vi.waitFor(() => {
        expect(wrong.calls).toHaveLength(n + 1);
      });
    }
    await vi.waitFor(() => {
      expect(el("heading").textContent).toBe(en["webauthn.heading.failed"]);
    });
    expect(el("message").textContent).toBe(en["webauthn.failed.body"]);

    // A rejected answer: failed. A run-out handoff at the finish: expired.
    for (const [status, heading] of [
      [401, en["webauthn.heading.failed"]],
      [410, en["webauthn.heading.expired"]],
    ] as const) {
      mount();
      const s = server({
        "/v1/webauthn/handoff/options": [options("login")],
        "/v1/webauthn/handoff/finish": [{ status }],
      });
      await start({
        document,
        hash: `#${ID}`,
        fetch: s.fetch,
        credentials: credentials(resolved),
        languages: ["en"],
      });
      type("123456");
      await vi.waitFor(() => {
        expect(el("heading").textContent).toBe(heading);
      });
      const finish = s.calls.find((c) => c.path === "/v1/webauthn/handoff/finish");
      expect(finish?.body).toEqual({
        handoffId: ID,
        credential: {
          id: "cred",
          rawId: "BwcHBw",
          type: "public-key",
          response: {
            clientDataJSON: "BwcHBwcHBwc",
            authenticatorData: "BwcHBwcHBwc",
            signature: "BwcHBwcHBwc",
          },
        },
      });
    }

    // A cancelled ceremony offers a retry and sends nothing.
    mount();
    const cancelled = server({ "/v1/webauthn/handoff/options": [options("login")] });
    const denied = (): Promise<unknown> =>
      Promise.reject(new DOMException("denied", "NotAllowedError"));
    await start({
      document,
      hash: `#${ID}`,
      fetch: cancelled.fetch,
      credentials: credentials(denied),
      languages: ["en"],
    });
    type("123456");
    await vi.waitFor(() => {
      expect(el("message").textContent).toBe(en["webauthn.cancelled"]);
    });
    expect(el("retry").hidden).toBe(false);
    expect(cancelled.calls).toHaveLength(1);
  });

  it("strings_json_is_current", () => {
    expect(stringsJson).toBe(`${JSON.stringify({ en, de }, null, 2)}\n`);
  });

  it("page_css_tokens_are_current", () => {
    const css = read("page.css");
    const from = css.indexOf("/* tokens:start");
    const to = css.indexOf("/* tokens:end */");
    const block = css.slice(css.indexOf("\n", from) + 1, to);
    expect(block).toBe(`${cssVariables()}\n`);
    // Nothing outside the token block holds a raw colour.
    expect(`${css.slice(0, from)}${css.slice(to)}`).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(/);
  });

  it("page_js_never_writes_html", () => {
    const js = read("page.js");
    expect(js).not.toMatch(/innerHTML|outerHTML|insertAdjacentHTML|document\.write|eval\(/);
    expect(html).not.toMatch(/<script(?![^>]*\bsrc=)/);
    expect(html).not.toMatch(/\son[a-z]+=|style=/);
  });
});
