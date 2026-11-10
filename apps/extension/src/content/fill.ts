/**
 * Writes values into fields the way a person typing would, so frameworks that track input
 * (React's value tracker, Vue's v-model) see the change. Nothing here is stored: values are
 * parameters and locals, and the caller drops its references right after.
 */
import type { FillSummary } from "../shared/messages.ts";
import type { ChangeClassification } from "./change.ts";
import type { Detection } from "./detect.ts";

/** The DOM's own `value` setter. Called in the extension's isolated world, it skips any setter the page installed on the element. */
function nativeSetter(): ((this: HTMLInputElement, value: string) => void) | undefined {
  // eslint-disable-next-line @typescript-eslint/unbound-method -- called with an explicit `this` below
  return Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
}

/** Focuses `el`, sets its value and fires `input` then `change`. */
export function setFieldValue(el: HTMLInputElement, value: string): void {
  el.focus();
  const setter = nativeSetter();
  if (setter === undefined) {
    el.value = value;
  } else {
    setter.call(el, value);
  }
  el.dispatchEvent(
    new InputEvent("input", { bubbles: true, composed: true, inputType: "insertReplacementText" }),
  );
  el.dispatchEvent(new Event("change", { bubbles: true }));
}

export interface Credentials {
  username: string | null;
  password: string | null;
  otp: string | null;
}

/** One digit per box; stops at the shorter of the code and the boxes. Other characters are not filled. */
function fillSegments(boxes: readonly HTMLInputElement[], code: string): number {
  let filled = 0;
  for (const [index, box] of boxes.entries()) {
    const digit = code[index];
    if (digit === undefined || !/^\d$/.test(digit)) {
      break;
    }
    setFieldValue(box, digit);
    filled++;
  }
  return filled;
}

function fillOtp(boxes: readonly HTMLInputElement[], code: string): number {
  const [only] = boxes;
  if (boxes.length === 1 && only !== undefined) {
    setFieldValue(only, code);
    return 1;
  }
  return fillSegments(boxes, code);
}

/** Fills username, then password, then the code, in that order; leaves focus on the last field. */
export function fillCredentials(detection: Detection, credentials: Credentials): FillSummary {
  const summary: FillSummary = { username: 0, password: 0, otp: 0, newPassword: 0 };
  const [username] = detection.username;
  if (credentials.username !== null && username !== undefined) {
    setFieldValue(username, credentials.username);
    summary.username = 1;
  }
  if (credentials.password !== null) {
    for (const field of detection.password) {
      setFieldValue(field, credentials.password);
      summary.password++;
    }
  }
  if (credentials.otp !== null) {
    summary.otp = fillOtp(detection.otp, credentials.otp);
  }
  return summary;
}

/** Fills every new-password field (a sign-up or change form) with a generated password. */
export function fillNewPasswords(detection: Detection, password: string): FillSummary {
  for (const field of detection.newPassword) {
    setFieldValue(field, password);
  }
  return { username: 0, password: 0, otp: 0, newPassword: detection.newPassword.length };
}

/**
 * Fills a change form: the current password into the current field, the new password into the new
 * and the confirmation field. The user name is never touched and the form is never submitted.
 */
export function fillChangeFields(
  form: ChangeClassification,
  values: { current: string | null; next: string },
): FillSummary {
  const summary: FillSummary = { username: 0, password: 0, otp: 0, newPassword: 0 };
  const [current] = form.current;
  if (values.current !== null && current !== undefined) {
    setFieldValue(current, values.current);
    summary.password = 1;
  }
  for (const field of form.next) {
    setFieldValue(field, values.next);
    summary.newPassword++;
  }
  return summary;
}
