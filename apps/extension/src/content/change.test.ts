import { afterEach, describe, expect, it } from "vitest";
import { classifyChangeForm, sameClassification } from "./change.ts";
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
  return classifyChangeForm(document, env);
}

const ids = (list: readonly HTMLInputElement[]): string[] => list.map((el) => el.id);

afterEach(() => {
  document.body.innerHTML = "";
});

describe("autocomplete tokens", () => {
  it("takes current-password and new-password as they say", () => {
    const c = run(
      '<form><input id="u" autocomplete="username"><input id="old" type="password" autocomplete="current-password">' +
        '<input id="new" type="password" autocomplete="new-password"><input id="again" type="password" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("currentAndNew");
    expect(ids(c.current)).toEqual(["old"]);
    expect(ids(c.next)).toEqual(["new", "again"]);
  });

  it("calls a form with new-password fields only newOnly, and never touches the user name", () => {
    const c = run(
      '<form><input id="u" type="email" autocomplete="username"><input id="n" type="password" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("newOnly");
    expect(c.current).toEqual([]);
    expect(ids(c.next)).toEqual(["n"]);
  });

  it("takes the one remaining password field as the current one when the new ones come from autocomplete", () => {
    const c = run(
      '<form><input id="old" type="password"><input id="n1" type="password" autocomplete="new-password"><input id="n2" type="password" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("currentAndNew");
    expect(ids(c.current)).toEqual(["old"]);
  });

  it("is ambiguous when two fields remain without a token", () => {
    const c = run(
      '<form><input id="a" type="password"><input id="b" type="password"><input id="n" type="password" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });

  it("is ambiguous when one field has both tokens", () => {
    const c = run(
      '<form><input id="a" type="password" autocomplete="current-password new-password"><input id="n" type="password" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });

  it("is ambiguous with three new-password fields", () => {
    const c = run(
      '<form><input type="password" autocomplete="new-password"><input type="password" autocomplete="new-password"><input type="password" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });

  it("takes a show-password toggle (text with an explicit token) as a password field", () => {
    const c = run(
      '<form><input id="old" type="password" autocomplete="current-password"><input id="n" type="text" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("currentAndNew");
    expect(ids(c.next)).toEqual(["n"]);
  });

  it("refuses a user name field that declares new-password next to a real new field", () => {
    const c = run(
      '<form><input id="user" type="text" name="username" autocomplete="new-password"><input id="old" type="password" autocomplete="current-password">' +
        '<input id="new" type="password" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("ambiguous");
    expect(c.next).toEqual([]);
  });

  it("refuses a text toggle that reads like a user name or an e-mail address even when it is the only new field", () => {
    for (const toggle of [
      '<input id="n" type="text" name="login" autocomplete="new-password">',
      '<input id="n" type="text" autocomplete="new-password email">',
    ]) {
      const c = run(`<form><input id="old" type="password" autocomplete="current-password">${toggle}</form>`);
      expect(c.shape).toBe("ambiguous");
    }
  });

  it("refuses two text toggles for one role", () => {
    const c = run(
      '<form><input id="old" type="password" autocomplete="current-password"><input id="a" type="text" autocomplete="new-password"><input id="b" type="text" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });
});

describe("three fields without tokens", () => {
  it("reads them as current, new and confirmation", () => {
    const c = run('<form><input id="a" type="password"><input id="b" type="password"><input id="c" type="password"></form>');
    expect(c.shape).toBe("currentAndNew");
    expect(ids(c.current)).toEqual(["a"]);
    expect(ids(c.next)).toEqual(["b", "c"]);
  });

  it("is ambiguous when a fourth, hidden password field sits in the form", () => {
    const c = run(
      '<form><input id="a" type="password"><input id="b" type="password"><input id="c" type="password"><div data-hidden><input id="d" type="password"></div></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });

  it("is ambiguous when one of the three is hidden", () => {
    const c = run(
      '<form><input id="a" type="password"><input id="b" type="password"><div data-hidden><input id="c" type="password"></div></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });
});

describe("what the fields say", () => {
  it("reads English names", () => {
    const c = run(
      '<form><input id="a" type="password" name="old_password"><input id="b" type="password" name="newPassword"></form>',
    );
    expect(c.shape).toBe("currentAndNew");
    expect(ids(c.current)).toEqual(["a"]);
    expect(ids(c.next)).toEqual(["b"]);
  });

  it("reads German labels", () => {
    const c = run(
      '<form><label for="a">Aktuelles Passwort</label><input id="a" type="password">' +
        '<label for="b">Neues Passwort</label><input id="b" type="password">' +
        '<label for="c">Neues Passwort wiederholen</label><input id="c" type="password"></form>',
    );
    expect(c.shape).toBe("currentAndNew");
    expect(ids(c.current)).toEqual(["a"]);
    expect(ids(c.next)).toEqual(["b", "c"]);
  });

  it("reads a form that only asks for a new password and its repetition", () => {
    const c = run(
      '<form><input id="a" type="password" placeholder="New password"><input id="b" type="password" placeholder="Confirm new password"></form>',
    );
    expect(c.shape).toBe("newOnly");
    expect(ids(c.next)).toEqual(["a", "b"]);
  });

  it("is ambiguous for two unlabeled password fields", () => {
    const c = run('<form><input id="a" type="password"><input id="b" type="password"></form>');
    expect(c.shape).toBe("ambiguous");
  });

  it("is ambiguous when a field says both new and old", () => {
    const c = run('<form><input id="a" type="password" name="old_new"><input id="b" type="password" name="new"></form>');
    expect(c.shape).toBe("ambiguous");
  });
});

describe("what is not a change form", () => {
  it("finds nothing in a sign-in form", () => {
    expect(run('<form><input id="u"><input id="p" type="password" autocomplete="current-password"></form>').shape).toBe("none");
    expect(run('<form><input id="u"><input id="p" type="password"></form>').shape).toBe("none");
  });

  it("finds nothing in a page without a password field", () => {
    expect(run('<form><input id="q" type="search"></form>').shape).toBe("none");
  });

  it("is ambiguous when two forms hold new-password fields and neither has focus", () => {
    const c = run(
      '<form><input type="password" autocomplete="new-password"></form><form><input type="password" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });

  it("takes the form with focus when two forms hold new-password fields", () => {
    document.body.innerHTML =
      '<form id="f1"><input id="n1" type="password" autocomplete="new-password"></form><form id="f2"><input id="n2" type="password" autocomplete="new-password"></form>';
    document.getElementById("n2")?.focus();
    const c = classifyChangeForm(document, env);
    expect(c.shape).toBe("newOnly");
    expect(ids(c.next)).toEqual(["n2"]);
  });
});

describe("the wrong field", () => {
  it("is ambiguous for a new-password field that is a text field without a token", () => {
    const c = run(
      '<form><input id="old" type="password" autocomplete="current-password"><input id="new" type="text" name="new-password"></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });

  it("is ambiguous when the user name field is named like a new password", () => {
    const c = run(
      '<form><input id="u" type="text" name="new-password"><input id="old" type="password" autocomplete="current-password">' +
        '<input id="n" type="password" autocomplete="new-password"></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });

  it("is ambiguous for a read-only or disabled field", () => {
    expect(
      run('<form><input id="o" type="password" autocomplete="current-password" readonly><input id="n" type="password" autocomplete="new-password"></form>').shape,
    ).toBe("ambiguous");
    expect(
      run('<form><input id="o" type="password" autocomplete="current-password"><input id="n" type="password" autocomplete="new-password" disabled></form>').shape,
    ).toBe("ambiguous");
  });

  it("is ambiguous for a field the user cannot see", () => {
    const c = run(
      '<form><input id="o" type="password" autocomplete="current-password"><div data-hidden><input id="n" type="password" autocomplete="new-password"></div></form>',
    );
    expect(c.shape).toBe("ambiguous");
  });
});

describe("the fill-time comparison", () => {
  const form =
    '<form><input id="a" type="password" autocomplete="current-password"><input id="b" type="password" autocomplete="new-password"></form>';

  it("matches itself", () => {
    expect(sameClassification(run(form), classifyChangeForm(document, env))).toBe(true);
  });

  it("notices swapped autocomplete values", () => {
    const before = run(form);
    document.getElementById("a")?.setAttribute("autocomplete", "new-password");
    document.getElementById("b")?.setAttribute("autocomplete", "current-password");
    expect(sameClassification(before, classifyChangeForm(document, env))).toBe(false);
  });

  it("notices reordered fields in a form without tokens", () => {
    const before = run('<form><input id="a" type="password"><input id="b" type="password"><input id="c" type="password"></form>');
    const first = document.getElementById("a");
    const last = document.getElementById("c");
    last?.before(first as Node);
    document.getElementById("b")?.before(last as Node);
    expect(sameClassification(before, classifyChangeForm(document, env))).toBe(false);
  });

  it("notices a field that was swapped for another one", () => {
    const before = run(form);
    const replacement = document.createElement("input");
    replacement.id = "b";
    replacement.type = "password";
    replacement.setAttribute("autocomplete", "new-password");
    document.getElementById("b")?.replaceWith(replacement);
    expect(sameClassification(before, classifyChangeForm(document, env))).toBe(false);
  });
});
