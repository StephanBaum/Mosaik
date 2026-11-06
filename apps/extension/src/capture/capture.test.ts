import { afterEach, describe, expect, it } from "vitest";
import type { VisibilityEnv } from "../content/visibility.ts";
import {
  type CaptureWindow,
  containerOf,
  extract,
  isTextEntry,
  mayReport,
  MIN_GAP_MS,
  passwordsOf,
  submitControl,
  Throttle,
} from "./capture.ts";

/** Layout does not exist in jsdom: every field is visible unless it sits in `[data-hidden]`. */
const env: VisibilityEnv = {
  checkVisibility: (el) => el.closest("[data-hidden]") === null,
  box: () => ({ left: 10, top: 10, right: 210, bottom: 40, width: 200, height: 30 }),
  opacity: () => 1,
  parent: (el) => el.parentElement,
  viewport: () => ({ width: 1000, height: 800, scrollX: 0, scrollY: 0, docWidth: 1000, docHeight: 800 }),
  elementAt: (el) => el,
  clip: () => ({ overflow: false, shape: false }),
};

function form(markup: string, values: Record<string, string>): HTMLFormElement {
  document.body.innerHTML = `<form id="f">${markup}</form>`;
  for (const [id, value] of Object.entries(values)) {
    (document.getElementById(id) as HTMLInputElement).value = value;
  }
  return document.getElementById("f") as HTMLFormElement;
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("what a submitted form yields", () => {
  it("a login: the trimmed user name and the password", () => {
    const f = form('<input id="u" autocomplete="username"><input id="p" type="password">', {
      u: "  ada@example.com ",
      p: "s3cret",
    });
    expect(extract(f, env)).toEqual({ username: "ada@example.com", password: "s3cret", newPassword: null });
  });

  it("a login without a user name field has no user name", () => {
    const f = form('<input id="p" type="password">', { p: "s3cret" });
    expect(extract(f, env)).toEqual({ username: null, password: "s3cret", newPassword: null });
  });

  it("a sign-up: the password, when the confirmation is equal", () => {
    const f = form(
      '<input id="u" type="email"><input id="p" type="password" autocomplete="new-password"><input id="c" type="password" autocomplete="new-password">',
      { u: "a@b.example", p: "Gen-pw-1", c: "Gen-pw-1" },
    );
    expect(extract(f, env)).toEqual({ username: "a@b.example", password: "Gen-pw-1", newPassword: null });
  });

  it("a sign-up with different confirmations yields nothing", () => {
    const f = form(
      '<input id="p" type="password" autocomplete="new-password"><input id="c" type="password" autocomplete="new-password">',
      { p: "one", c: "two" },
    );
    expect(extract(f, env)).toBeNull();
  });

  it("a change form: the current password and the new one", () => {
    const f = form(
      '<input id="o" type="password" autocomplete="current-password"><input id="n" type="password" autocomplete="new-password"><input id="c" type="password" autocomplete="new-password">',
      { o: "old-pw", n: "new-pw", c: "new-pw" },
    );
    expect(extract(f, env)).toEqual({ username: null, password: "old-pw", newPassword: "new-pw" });
  });

  it("a change form whose fields carry no hints: the first is the current password", () => {
    const f = form(
      '<input id="a" type="password"><input id="b" type="password"><input id="c" type="password">',
      { a: "old-pw", b: "new-pw", c: "new-pw" },
    );
    expect(extract(f, env)).toEqual({ username: null, password: "old-pw", newPassword: "new-pw" });
  });

  it("an empty password yields nothing, so an empty submit offers no save", () => {
    expect(extract(form('<input id="u"><input id="p" type="password">', { u: "ada" }), env)).toBeNull();
  });

  it("a password over 1024 characters yields nothing", () => {
    expect(extract(form('<input id="p" type="password">', { p: "x".repeat(1025) }), env)).toBeNull();
    expect(extract(form('<input id="p" type="password">', { p: "x".repeat(1024) }), env)).not.toBeNull();
  });

  it("a user name over 256 characters is left out, not truncated", () => {
    const f = form('<input id="u" autocomplete="username"><input id="p" type="password">', { u: "u".repeat(257), p: "pw" });
    expect(extract(f, env)).toEqual({ username: null, password: "pw", newPassword: null });
  });

  it("reads only the submitted form, not another form on the page", () => {
    document.body.innerHTML =
      '<form id="a"><input id="u1" autocomplete="username"><input id="p1" type="password"></form>' +
      '<form id="b"><input id="u2" autocomplete="username"><input id="p2" type="password"></form>';
    (document.getElementById("u1") as HTMLInputElement).value = "first";
    (document.getElementById("p1") as HTMLInputElement).value = "pw-first";
    (document.getElementById("u2") as HTMLInputElement).value = "second";
    (document.getElementById("p2") as HTMLInputElement).value = "pw-second";
    expect(extract(document.getElementById("b") as HTMLElement, env)).toMatchObject({ username: "second", password: "pw-second" });
    expect(extract(document.getElementById("a") as HTMLElement, env)).toMatchObject({ username: "first", password: "pw-first" });
  });

  it("ignores a password field a page hides", () => {
    const f = form('<input id="u" autocomplete="username"><div data-hidden><input id="p" type="password"></div>', {
      u: "ada",
      p: "hidden-pw",
    });
    expect(extract(f, env)).toBeNull();
  });

  it("a form without a password yields nothing (a search box, a user-name-first step)", () => {
    expect(extract(form('<input id="q" type="search">', { q: "x" }), env)).toBeNull();
    expect(extract(form('<input id="u" type="email">', { u: "a@b.example" }), env)).toBeNull();
  });
});

describe("passwordsOf", () => {
  it.each([
    [[], [], null],
    [["a"], [], { password: "a", newPassword: null }],
    [[], ["n", "n"], { password: "n", newPassword: null }],
    [["o"], ["n", "n"], { password: "o", newPassword: "n" }],
    [["o"], ["n", "m"], null],
    [[], ["o", "n", "n"], { password: "o", newPassword: "n" }],
    [[""], ["n"], null],
  ])("current %j new %j", (current, fresh, expected) => {
    expect(passwordsOf(current, fresh)).toEqual(expected);
  });
});

describe("where a click or a key counts", () => {
  it("finds the form around a control, or the nearest ancestor that holds a password", () => {
    document.body.innerHTML =
      '<form id="f"><button id="b1">Go</button></form><div id="d"><input type="password"><span><button id="b2">Go</button></span></div><p><button id="b3">Go</button></p>';
    expect(containerOf(document.getElementById("b1") as Element)?.id).toBe("f");
    expect(containerOf(document.getElementById("b2") as Element)?.id).toBe("d");
    expect(containerOf(document.getElementById("b3") as Element)).toBeNull();
  });

  it("knows submit controls and the boxes Enter submits from", () => {
    document.body.innerHTML =
      '<button id="b"><span id="s">x</span></button><input id="i" type="submit"><div id="r" role="button">y</div><a id="a" href="#">z</a><input id="p" type="password"><input id="t">';
    const el = (id: string): Element => document.getElementById(id) as Element;
    expect(submitControl(el("s"))?.id).toBe("b");
    expect(submitControl(el("i"))?.id).toBe("i");
    expect(submitControl(el("r"))?.id).toBe("r");
    expect(submitControl(el("a"))).toBeNull();
    expect(submitControl(undefined)).toBeNull();
    expect(isTextEntry(el("p"))).toBe(true);
    expect(isTextEntry(el("t"))).toBe(true);
    expect(isTextEntry(el("i"))).toBe(false);
    expect(isTextEntry(el("b"))).toBe(false);
  });
});

describe("Throttle", () => {
  it("allows one capture per two seconds", () => {
    let now = 10_000;
    const throttle = new Throttle(() => now);
    expect(throttle.allow()).toBe(true);
    now += MIN_GAP_MS - 1;
    expect(throttle.allow()).toBe(false);
    now += 1;
    expect(throttle.allow()).toBe(true);
  });
});

describe("which documents may report a login", () => {
  const SITE = "https://bank.example";

  function top(origin: string, urlOrigin = origin): CaptureWindow {
    const win: { parent?: CaptureWindow; frameElement: null; origin: string; location: { origin: string } } = {
      frameElement: null,
      origin,
      location: { origin: urlOrigin },
    };
    win.parent = win as CaptureWindow;
    return win as CaptureWindow;
  }

  function child(parent: CaptureWindow, origin: string, frameElement: Element): CaptureWindow {
    return { parent, frameElement, origin, location: { origin } };
  }

  it("the top document of a site", () => {
    expect(mayReport(top(SITE), env)).toBe(true);
  });

  it("never an opaque document, whatever its URL names (CSP sandbox, sandboxed frame)", () => {
    expect(mayReport(top("null", SITE), env)).toBe(false);
    expect(mayReport(top("null"), env)).toBe(false);
    document.body.innerHTML = '<iframe id="f"></iframe>';
    expect(mayReport(child(top(SITE), "null", document.getElementById("f") as Element), env)).toBe(false);
  });

  it("a visible same-origin frame, but not an invisible one or one under another site", () => {
    document.body.innerHTML = '<iframe id="seen"></iframe><div data-hidden><iframe id="unseen"></iframe></div>';
    const el = (id: string): Element => document.getElementById(id) as Element;
    expect(mayReport(child(top(SITE), SITE, el("seen")), env)).toBe(true);
    expect(mayReport(child(top(SITE), SITE, el("unseen")), env)).toBe(false);
    expect(mayReport(child(top("https://evil.example"), SITE, el("seen")), env)).toBe(false);
  });
});
