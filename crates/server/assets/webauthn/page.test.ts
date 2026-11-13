/// <reference types="node" />
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cssVariables } from "../../../../packages/design-tokens/src/css.ts";
import { webauthn as de } from "../../../../packages/i18n/src/catalog/de/webauthn.ts";
import { webauthn as en } from "../../../../packages/i18n/src/catalog/en/webauthn.ts";
import { handoffId, pickLocale, start } from "./page.js";

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
    expect(el("message").textContent).toBe(en["webauthn.code.tooMany"]);

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
  it("clears_the_id_from_the_address_and_focuses_each_screen_heading", async () => {
    // M9-F72: the ID is gone from the address once it is read. M9-UX8: each screen starts at its heading.
    mount();
    const cleared = vi.fn();
    const s = server({ "/v1/webauthn/handoff/options": [options("login")] });
    await start({
      document,
      hash: `#${ID}`,
      fetch: s.fetch,
      credentials: credentials(() => new Promise(() => undefined)),
      languages: ["en"],
      clearFragment: cleared,
    });
    expect(cleared).toHaveBeenCalledTimes(1);
    type("123456");
    await vi.waitFor(() => {
      expect(el("heading").textContent).toBe(en["webauthn.heading.login"]);
    });
    expect(document.activeElement).toBe(el("heading"));
    expect(el("lead").textContent).toContain(en["webauthn.lead.ask"]);
    // An incomplete link has no ID to clear.
    mount();
    const none = vi.fn();
    await start({ document, hash: "#nope", fetch: s.fetch, credentials: undefined, languages: ["en"], clearFragment: none });
    expect(none).not.toHaveBeenCalled();
  });
});

// ---- M9 test-engineer pass ------------------------------------------------------------------

async function open(
  replies: Record<string, Reply[]>,
  creds: CredentialsContainer | undefined,
  languages: string[] = ["en"],
): Promise<ReturnType<typeof server>> {
  mount();
  const s = server(replies);
  await start({ document, hash: `#${ID}`, fetch: s.fetch, credentials: creds, languages });
  return s;
}

describe("the security-key page: edges", () => {
  it("handoffId_takes_only_a_22_char_base64url_value", () => {
    expect(handoffId(`#${ID}`)).toBe(ID);
    expect(handoffId(ID)).toBe(ID);
    for (const bad of [
      "",
      "#",
      `#${ID}A`,
      `#${ID.slice(1)}`,
      `#${ID.slice(1)}+`,
      `#${ID.slice(1)}=`,
      `#${ID}&x=1`,
      `# ${ID}`,
      `#${ID}\n`,
      "#éAECAwQFBgcICQoLDA0ODw",
    ]) {
      expect(handoffId(bad), bad).toBeNull();
    }
  });

  it("pickLocale_takes_the_first_supported_language_and_defaults_to_en", () => {
    expect(pickLocale(["fr-FR", "de-AT", "en"])).toBe("de");
    expect(pickLocale(["EN-us"])).toBe("en");
    expect(pickLocale(["DE"])).toBe("de");
    expect(pickLocale([])).toBe("en");
    expect(pickLocale(["ja", "zh-CN"])).toBe("en");
  });

  it("only_six_ascii_digits_are_sent_and_the_code_is_trimmed", async () => {
    const s = await open({ "/v1/webauthn/handoff/options": [{ status: 401 }] }, undefined);
    for (const bad of ["12345", "1234567", "12345a", "١٢٣٤٥٦", "", "12 456"]) {
      type(bad);
      expect(el("code-error").textContent, bad).toBe(en["webauthn.code.format"]);
      expect(el("code").getAttribute("aria-invalid")).toBe("true");
    }
    expect(s.calls).toHaveLength(0);
    type("  123456\n");
    await vi.waitFor(() => {
      expect(s.calls).toHaveLength(1);
    });
    expect(s.calls[0]?.body).toEqual({ handoffId: ID, code: "123456" });
  });

  it("four_wrong_codes_leave_the_form_and_a_fifth_ends_it", async () => {
    const s = await open(
      { "/v1/webauthn/handoff/options": Array.from({ length: 5 }, () => ({ status: 401 })) },
      undefined,
    );
    for (let n = 1; n <= 4; n += 1) {
      type("000000");
      await vi.waitFor(() => {
        expect(s.calls).toHaveLength(n);
        expect(el("code-error").textContent).toBe(en["webauthn.code.invalid"]);
      });
      expect(el("code-form").hidden).toBe(false);
      expect((el("code-submit") as HTMLButtonElement).disabled).toBe(false);
    }
    type("000000");
    await vi.waitFor(() => {
      expect(el("heading").textContent).toBe(en["webauthn.heading.failed"]);
    });
    expect(el("code-form").hidden).toBe(true);
  });

  it("other_answers_to_the_code_each_have_their_own_screen", async () => {
    for (const [status, heading, message] of [
      [409, "webauthn.heading.failed", "webauthn.unavailable"],
      [429, "webauthn.heading.failed", "webauthn.rateLimited"],
      [500, "webauthn.heading.failed", "webauthn.failed.body"],
      [410, "webauthn.heading.expired", "webauthn.expired.body"],
    ] as const) {
      await open({ "/v1/webauthn/handoff/options": [{ status }] }, undefined);
      type("123456");
      await vi.waitFor(() => {
        expect(el("heading").textContent, String(status)).toBe(en[heading]);
      });
      expect(el("message").textContent, String(status)).toBe(en[message]);
    }
    // A malformed 200 (unknown purpose, missing members, no body) is not trusted.
    for (const body of [
      { purpose: "nuke", deviceName: "d", requestedAt: 1, publicKey: {} },
      { purpose: "login" },
      null,
    ]) {
      await open({ "/v1/webauthn/handoff/options": [{ status: 200, body }] }, credentials(resolved));
      type("123456");
      await vi.waitFor(() => {
        expect(el("heading").textContent).toBe(en["webauthn.heading.failed"]);
      });
    }
  });

  it("a_network_error_on_the_code_keeps_the_form_for_another_try", async () => {
    mount();
    let fail = true;
    const real = server({ "/v1/webauthn/handoff/options": [{ status: 401 }] });
    const flaky = ((input: RequestInfo | URL, init?: RequestInit) =>
      fail && typeof input === "string" && input.endsWith("/options")
        ? Promise.reject(new TypeError("offline"))
        : real.fetch(input, init)) as typeof fetch;
    await start({ document, hash: `#${ID}`, fetch: flaky, credentials: undefined, languages: ["en"] });
    type("123456");
    await vi.waitFor(() => {
      expect(el("code-error").textContent).toBe(en["webauthn.network"]);
    });
    expect(el("code-form").hidden).toBe(false);
    fail = false;
    type("123456");
    await vi.waitFor(() => {
      expect(real.calls).toHaveLength(1);
    });
  });

  it("browser_errors_map_to_their_messages_and_retry_runs_the_ceremony_again", async () => {
    for (const [name, key] of [
      ["InvalidStateError", "webauthn.alreadyRegistered"],
      ["NotAllowedError", "webauthn.cancelled"],
      ["AbortError", "webauthn.cancelled"],
      ["SecurityError", "webauthn.browserError"],
    ] as const) {
      let calls = 0;
      const create = (): Promise<unknown> => {
        calls += 1;
        return Promise.reject(new DOMException("x", name));
      };
      const s = await open({ "/v1/webauthn/handoff/options": [options("register")] }, credentials(create));
      type("123456");
      await vi.waitFor(() => {
        expect(el("message").textContent, name).toBe(en[key]);
      });
      expect(el("retry").hidden).toBe(false);
      (el("retry") as HTMLButtonElement).click();
      await vi.waitFor(() => {
        expect(calls).toBe(2);
      });
      // Nothing but the options request ever reached the server.
      expect(s.calls.map((c) => c.path)).toEqual(["/v1/webauthn/handoff/options"]);
    }
  });

  it("a_network_error_on_finish_retries_the_same_credential_without_a_new_ceremony", async () => {
    mount();
    let ceremonies = 0;
    const create = (): Promise<unknown> => {
      ceremonies += 1;
      return resolved();
    };
    const finishes: unknown[] = [];
    let reachable = false;
    const real = server({ "/v1/webauthn/handoff/options": [options("confirm")] });
    const fetchFn = ((input: RequestInfo | URL, init?: RequestInit) => {
      if (typeof input === "string" && input.endsWith("/finish")) {
        finishes.push(typeof init?.body === "string" ? init.body : null);
        return reachable
          ? Promise.resolve(new Response("null", { status: 200 }))
          : Promise.reject(new TypeError("offline"));
      }
      return real.fetch(input, init);
    }) as typeof fetch;
    await start({ document, hash: `#${ID}`, fetch: fetchFn, credentials: credentials(create), languages: ["en"] });
    type("123456");
    await vi.waitFor(() => {
      expect(el("message").textContent).toBe(en["webauthn.network"]);
    });
    reachable = true;
    (el("retry") as HTMLButtonElement).click();
    await vi.waitFor(() => {
      expect(el("heading").textContent).toBe(en["webauthn.heading.done"]);
    });
    expect(el("message").textContent).toBe(en["webauthn.done.confirm"]);
    expect(ceremonies).toBe(1);
    expect(finishes).toHaveLength(2);
    expect(finishes[0]).toBe(finishes[1]);
  });

  it("options_are_decoded_for_the_browser_and_the_credential_is_encoded_for_the_server", async () => {
    let seen: { publicKey: Record<string, unknown> } | undefined;
    const create = (o: { publicKey: Record<string, unknown> }): Promise<unknown> => {
      seen = o;
      return Promise.resolve({
        ...fakeCredential,
        response: { ...fakeCredential.response, getTransports: () => [] },
      });
    };
    const opts = options("register");
    opts.body.publicKey.challenge = "_-8"; // base64url without padding
    opts.body.publicKey.excludeCredentials = [{ type: "public-key", id: "AQID" }] as never[];
    const s = await open(
      { "/v1/webauthn/handoff/options": [opts], "/v1/webauthn/handoff/finish": [{ status: 200 }] },
      credentials(create as () => Promise<unknown>),
    );
    type("123456");
    await vi.waitFor(() => {
      expect(el("heading").textContent).toBe(en["webauthn.heading.done"]);
    });
    expect(Array.from(seen?.publicKey["challenge"] as Uint8Array)).toEqual([0xff, 0xef]);
    expect(Array.from((seen?.publicKey["user"] as { id: Uint8Array }).id)).toEqual([0, 0, 0]);
    const exclude = seen?.publicKey["excludeCredentials"] as { id: Uint8Array }[];
    expect(Array.from(exclude[0]?.id ?? [])).toEqual([1, 2, 3]);
    const sent = s.calls.find((c) => c.path === "/v1/webauthn/handoff/finish")?.body as {
      credential: { response: Record<string, unknown> };
    };
    expect(sent.credential.response["transports"], "empty transports are left out").toBeUndefined();
    expect(sent.credential.response["attestationObject"]).toBe("BwcHBwcHBwc");
  });

  it("a_non_empty_user_handle_is_sent_and_an_empty_one_is_not", async () => {
    for (const [handle, expected] of [
      [new Uint8Array([1, 2, 3]).buffer, "AQID"],
      [new ArrayBuffer(0), undefined],
      [null, undefined],
    ] as const) {
      const create = (): Promise<unknown> =>
        Promise.resolve({
          ...fakeCredential,
          response: { ...fakeCredential.response, userHandle: handle },
        });
      const s = await open(
        { "/v1/webauthn/handoff/options": [options("login")], "/v1/webauthn/handoff/finish": [{ status: 200 }] },
        credentials(create),
      );
      type("123456");
      await vi.waitFor(() => {
        expect(s.calls).toHaveLength(2);
      });
      const sent = s.calls[1]?.body as { credential: { response: Record<string, unknown> } };
      expect(sent.credential.response["userHandle"]).toBe(expected);
    }
  });

  it("a_browser_without_webauthn_says_so_and_never_posts_a_finish", async () => {
    const s = await open({ "/v1/webauthn/handoff/options": [options("register")] }, undefined);
    type("123456");
    await vi.waitFor(() => {
      expect(el("message").textContent).toBe(en["webauthn.unsupported"]);
    });
    expect(s.calls.map((c) => c.path)).toEqual(["/v1/webauthn/handoff/options"]);
  });
});
