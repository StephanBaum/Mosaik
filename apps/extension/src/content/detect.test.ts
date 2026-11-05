import { afterEach, describe, expect, it } from "vitest";
import { countFields, detect, hasActiveField } from "./detect.ts";
import type { VisibilityEnv } from "./visibility.ts";

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

function run(markup: string) {
  document.body.innerHTML = markup;
  return detect(document, env);
}

const ids = (list: readonly HTMLInputElement[]): string[] => list.map((el) => el.id);

afterEach(() => {
  document.body.innerHTML = "";
});

describe("login forms", () => {
  it("finds username and password", () => {
    const d = run('<form><input id="u" name="login"><input id="p" type="password"></form>');
    expect(ids(d.username)).toEqual(["u"]);
    expect(ids(d.password)).toEqual(["p"]);
    expect(d.newPassword).toEqual([]);
    expect(countFields(d)).toEqual({ username: 1, password: 1, newPassword: 0, otp: 0 });
  });

  it("finds a form without a <form> element", () => {
    const d = run('<div><input id="u" type="email"><div><input id="p" type="password"></div></div>');
    expect(ids(d.username)).toEqual(["u"]);
    expect(ids(d.password)).toEqual(["p"]);
  });

  it("ignores a username candidate more than five levels away in a formless page", () => {
    const deep = "<div>".repeat(6) + '<input id="p" type="password">' + "</div>".repeat(6);
    const d = run(`<div><input id="u">${deep}</div>`);
    expect(ids(d.password)).toEqual(["p"]);
    expect(d.username).toEqual([]);
  });

  it("prefers a field that declares autocomplete username or email over a nearer one", () => {
    const d = run(
      '<form><input id="a" autocomplete="email"><input id="b" name="x"><input id="p" type="password"></form>',
    );
    expect(ids(d.username)).toEqual(["a"]);
  });

  it("takes the nearest text field before the password when nothing declares itself", () => {
    const d = run('<form><input id="a"><input id="b"><input id="p" type="password"></form>');
    expect(ids(d.username)).toEqual(["b"]);
  });

  it("treats autocomplete current-password on a text input as a password", () => {
    const d = run('<form><input id="u"><input id="p" type="text" autocomplete="current-password"></form>');
    expect(ids(d.password)).toEqual(["p"]);
  });

  it("never takes a search box for the username", () => {
    const d = run(
      '<form><input id="s" type="search"><input id="q" name="q"><input id="r" role="search"><input id="p" type="password"></form>',
    );
    expect(d.username).toEqual([]);
    expect(ids(d.password)).toEqual(["p"]);
  });

  it("picks the login form over the site search and a newsletter field", () => {
    const d = run(
      '<form role="search"><input id="s" type="search"></form>' +
        '<form><input id="news" type="email"></form>' +
        '<form><input id="u"><input id="p" type="password"></form>',
    );
    expect(ids(d.username)).toEqual(["u"]);
    expect(ids(d.password)).toEqual(["p"]);
  });
});

describe("username-first pages", () => {
  it.each([
    ['<form><input id="u" autocomplete="username"></form>'],
    ['<form><input id="u" type="email"></form>'],
    ['<form><input id="u" name="account_id"></form>'],
    ['<form><input id="u" name="benutzername"></form>'],
  ])("%s has a lone username", (markup) => {
    const d = run(markup);
    expect(d.username).toHaveLength(1);
    expect(d.password).toEqual([]);
  });

  it("does not guess when there are several fields or no hint", () => {
    expect(run('<form><input id="a"><input id="b"></form>').username).toEqual([]);
    expect(run('<form><input id="a" name="color"></form>').username).toEqual([]);
  });
});

describe("sign-up and change forms", () => {
  it("calls two password fields new", () => {
    const d = run('<form><input id="u"><input id="p1" type="password"><input id="p2" type="password"></form>');
    expect(ids(d.newPassword)).toEqual(["p1", "p2"]);
    expect(d.password).toEqual([]);
    expect(ids(d.username)).toEqual(["u"]);
  });

  it("calls a field with autocomplete new-password new, and one named confirm", () => {
    expect(ids(run('<form><input id="p" type="password" autocomplete="new-password"></form>').newPassword)).toEqual(["p"]);
    expect(ids(run('<form><input id="p" type="password" name="password_confirm"></form>').newPassword)).toEqual(["p"]);
    expect(ids(run('<form><input id="p" type="password" name="neues_passwort"></form>').newPassword)).toEqual(["p"]);
  });

  it("keeps a leading current-password current in a change form", () => {
    const d = run(
      '<form><input id="old" type="password" autocomplete="current-password">' +
        '<input id="n1" type="password" autocomplete="new-password"><input id="n2" type="password"></form>',
    );
    expect(ids(d.password)).toEqual(["old"]);
    expect(ids(d.newPassword)).toEqual(["n1", "n2"]);
  });

  it("reads the passwordrules attribute from the page, not from detect", () => {
    const d = run('<form><input id="p" type="password" autocomplete="new-password" passwordrules="minlength: 12"></form>');
    expect(d.newPassword[0]?.getAttribute("passwordrules")).toBe("minlength: 12");
  });
});

describe("one-time codes", () => {
  it("finds autocomplete one-time-code", () => {
    const d = run('<form><input id="c" autocomplete="one-time-code"></form>');
    expect(ids(d.otp)).toEqual(["c"]);
    expect(d.username).toEqual([]);
  });

  it("finds a field named like a code when it also looks numeric", () => {
    expect(ids(run('<form><input id="c" name="totp_code" inputmode="numeric"></form>').otp)).toEqual(["c"]);
    expect(ids(run('<form><input id="c" placeholder="Verification code" maxlength="6"></form>').otp)).toEqual(["c"]);
    expect(ids(run('<form><input id="c" aria-label="Bestätigungscode" pattern="[0-9]*"></form>').otp)).toEqual(["c"]);
  });

  it("does not take a name alone", () => {
    expect(run('<form><input id="c" name="otp"></form>').otp).toEqual([]);
  });

  it("finds a code split into one-character boxes", () => {
    const boxes = Array.from({ length: 6 }, (_, i) => `<input id="d${String(i)}" maxlength="1" inputmode="numeric">`).join("");
    const d = run(`<form>${boxes}</form>`);
    expect(ids(d.otp)).toEqual(["d0", "d1", "d2", "d3", "d4", "d5"]);
    expect(d.username).toEqual([]);
    expect(countFields(d).otp).toBe(1);
  });

  it("does not take two or nine boxes for a code", () => {
    const many = (n: number): string => Array.from({ length: n }, () => '<input maxlength="1">').join("");
    expect(run(`<form>${many(2)}</form>`).otp).toEqual([]);
    expect(run(`<form>${many(9)}</form>`).otp).toEqual([]);
  });

  it("finds a code field next to a login", () => {
    const d = run('<form><input id="u"><input id="p" type="password"><input id="c" autocomplete="one-time-code"></form>');
    expect(ids(d.username)).toEqual(["u"]);
    expect(ids(d.otp)).toEqual(["c"]);
  });
});

describe("fields that are not candidates", () => {
  it("skips hidden, button-like, disabled and read-only inputs", () => {
    const d = run(
      '<form><input id="h" type="hidden" name="user"><input id="b" type="submit"><input id="c" type="checkbox">' +
        '<input id="d" disabled name="user"><input id="r" readonly name="user"><input id="p" type="password"></form>',
    );
    expect(d.username).toEqual([]);
    expect(ids(d.password)).toEqual(["p"]);
  });

  it("skips fields in a disabled fieldset", () => {
    expect(run('<form><fieldset disabled><input id="p" type="password"></fieldset></form>').password).toEqual([]);
  });

  it("skips fields the visibility check refuses", () => {
    const d = run(
      '<form><input id="u"><div data-hidden><input id="ghost" type="password"></div><input id="p" type="password"></form>',
    );
    expect(ids(d.password)).toEqual(["p"]);
  });

  it("finds nothing when every field is hidden", () => {
    const d = run('<div data-hidden><form><input id="u"><input id="p" type="password"></form></div>');
    expect(countFields(d)).toEqual({ username: 0, password: 0, newPassword: 0, otp: 0 });
  });

  it("is not fooled by a field named like a form property (DOM clobbering)", () => {
    const d = run('<form id="f"><input name="elements"><input name="type"><input id="u"><input id="p" type="password"></form>');
    expect(ids(d.password)).toEqual(["p"]);
    expect(d.username).toHaveLength(1);
  });
});

describe("shadow roots and focus", () => {
  it("looks into open shadow roots", () => {
    document.body.innerHTML = '<div id="host"></div>';
    const root = (document.querySelector("#host") as HTMLElement).attachShadow({ mode: "open" });
    root.innerHTML = '<form><input id="u"><input id="p" type="password"></form>';
    const d = detect(document, env);
    expect(ids(d.username)).toEqual(["u"]);
    expect(ids(d.password)).toEqual(["p"]);
  });

  it("does not look into closed shadow roots", () => {
    document.body.innerHTML = '<div id="host"></div>';
    (document.querySelector("#host") as HTMLElement).attachShadow({ mode: "closed" }).innerHTML =
      '<input id="p" type="password">';
    expect(detect(document, env).password).toEqual([]);
  });

  it("stops at depth eight", () => {
    document.body.innerHTML = '<div id="h0"></div>';
    let host = document.querySelector("#h0") as HTMLElement;
    for (let level = 1; level <= 9; level++) {
      const root = host.attachShadow({ mode: "open" });
      const next = document.createElement("div");
      root.append(next);
      host = next;
    }
    host.innerHTML = '<input id="deep" type="password">';
    expect(detect(document, env).password).toEqual([]);
  });

  it("chooses the form that has focus, and reports an active field", () => {
    document.body.innerHTML =
      '<form><input id="u1"><input id="p1" type="password"></form>' +
      '<form><input id="p2" type="password" autocomplete="new-password"><input id="p3" type="password"></form>';
    expect(ids(detect(document, env).password)).toEqual(["p1"]);
    (document.querySelector("#p3") as HTMLInputElement).focus();
    const d = detect(document, env);
    expect(ids(d.newPassword)).toEqual(["p2", "p3"]);
    expect(hasActiveField(d)).toBe(true);
  });

  it("does not report a focused field that is not one of the detected ones", () => {
    document.body.innerHTML = '<form><input id="p" type="password"></form><input id="other" name="color">';
    (document.querySelector("#other") as HTMLInputElement).focus();
    expect(hasActiveField(detect(document, env))).toBe(false);
  });
});
