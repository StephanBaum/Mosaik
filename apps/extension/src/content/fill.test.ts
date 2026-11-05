import { afterEach, describe, expect, it, vi } from "vitest";
import type { Detection } from "./detect.ts";
import { fillCredentials, fillNewPasswords, setFieldValue } from "./fill.ts";

afterEach(() => {
  document.body.innerHTML = "";
});

function inputs(markup: string): Record<string, HTMLInputElement> {
  document.body.innerHTML = markup;
  return Object.fromEntries([...document.querySelectorAll("input")].map((el) => [el.id, el]));
}

/** React's input tracking: it wraps the element's own `value` and compares on every `input` event. */
function trackLikeReact(el: HTMLInputElement, onChange: (value: string) => void): void {
  const proto = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value");
  let tracked = "";
  Object.defineProperty(el, "value", {
    configurable: true,
    get: () => proto?.get?.call(el) as string,
    set: (next: string) => {
      tracked = next;
      proto?.set?.call(el, next);
    },
  });
  el.addEventListener("input", () => {
    const actual = proto?.get?.call(el) as string;
    if (actual !== tracked) {
      tracked = actual;
      onChange(actual);
    }
  });
}

const detection = (parts: Partial<Detection>): Detection => ({
  username: [],
  password: [],
  newPassword: [],
  otp: [],
  active: null,
  ...parts,
});

describe("setFieldValue", () => {
  it("sets the value, fires input then change, and leaves focus on the field", () => {
    const a = inputs('<input id="a">')["a"] as HTMLInputElement;
    const seen: string[] = [];
    for (const type of ["focus", "input", "change"]) {
      a.addEventListener(type, (event) => {
        seen.push(`${event.type}:${String(event.bubbles)}`);
      });
    }
    setFieldValue(a, "ada");
    expect(a.value).toBe("ada");
    expect(seen).toEqual(["focus:false", "input:true", "change:true"]);
    expect(document.activeElement).toBe(a);
  });

  it("fires a composed InputEvent with the replacement input type", () => {
    const a = inputs('<input id="a">')["a"] as HTMLInputElement;
    let event: InputEvent | undefined;
    a.addEventListener("input", (e) => {
      event = e;
    });
    setFieldValue(a, "x");
    expect(event).toBeInstanceOf(InputEvent);
    expect(event?.composed).toBe(true);
    expect(event?.inputType).toBe("insertReplacementText");
  });

  it("reaches a framework that tracks the value (React's onChange fires)", () => {
    const f = inputs('<input id="a"><input id="b">');
    const a = f["a"] as HTMLInputElement;
    const b = f["b"] as HTMLInputElement;
    const viaSetter = vi.fn();
    trackLikeReact(a, viaSetter);
    setFieldValue(a, "secret");
    expect(viaSetter).toHaveBeenCalledExactlyOnceWith("secret");

    // Control: assigning .value on the element is exactly what the tracker swallows.
    const viaAssignment = vi.fn();
    trackLikeReact(b, viaAssignment);
    b.value = "secret";
    b.dispatchEvent(new InputEvent("input", { bubbles: true }));
    expect(viaAssignment).not.toHaveBeenCalled();
  });
});

describe("fillCredentials", () => {
  it("fills username, password and code in that order and counts them", () => {
    const f = inputs('<input id="u"><input id="p" type="password"><input id="c">');
    const order: string[] = [];
    document.addEventListener("input", (e) => {
      order.push((e.target as HTMLElement).id);
    });
    const summary = fillCredentials(
      detection({
        username: [f["u"] as HTMLInputElement],
        password: [f["p"] as HTMLInputElement],
        otp: [f["c"] as HTMLInputElement],
      }),
      { username: "ada", password: "pw", otp: "123456" },
    );
    expect(summary).toEqual({ username: 1, password: 1, otp: 1, newPassword: 0 });
    expect(f["u"]?.value).toBe("ada");
    expect(f["p"]?.value).toBe("pw");
    expect(f["c"]?.value).toBe("123456");
    expect(order).toEqual(["u", "p", "c"]);
    expect(document.activeElement).toBe(f["c"]);
  });

  it("leaves out what the credential does not have", () => {
    const f = inputs('<input id="u" value="keep"><input id="p" type="password">');
    const summary = fillCredentials(
      detection({ username: [f["u"] as HTMLInputElement], password: [f["p"] as HTMLInputElement] }),
      { username: null, password: "pw", otp: null },
    );
    expect(summary).toEqual({ username: 0, password: 1, otp: 0, newPassword: 0 });
    expect(f["u"]?.value).toBe("keep");
  });

  it("puts one digit in each box of a split code and stops at a non-digit", () => {
    const f = inputs(Array.from({ length: 6 }, (_, i) => `<input id="d${String(i)}" maxlength="1">`).join(""));
    const boxes = Object.values(f);
    const full = fillCredentials(detection({ otp: boxes }), { username: null, password: null, otp: "493827" });
    expect(full.otp).toBe(6);
    expect(boxes.map((b) => b.value).join("")).toBe("493827");
    const partial = fillCredentials(detection({ otp: boxes }), { username: null, password: null, otp: "12ab56" });
    expect(partial.otp).toBe(2);
  });
});

describe("fillNewPasswords", () => {
  it("fills every new-password field and nothing else", () => {
    const f = inputs('<input id="u"><input id="p1" type="password"><input id="p2" type="password">');
    const summary = fillNewPasswords(
      detection({
        username: [f["u"] as HTMLInputElement],
        newPassword: [f["p1"] as HTMLInputElement, f["p2"] as HTMLInputElement],
      }),
      "generated",
    );
    expect(summary.newPassword).toBe(2);
    expect(f["p1"]?.value).toBe("generated");
    expect(f["p2"]?.value).toBe("generated");
    expect(f["u"]?.value).toBe("");
  });
});
