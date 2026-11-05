/**
 * What the opt-in save capture reads from a submitted form (pure: it takes the form's
 * container and reads the DOM it is given). Only fields of that container count, so another
 * form on the page cannot answer for it, and only a password of 1 to 1024 characters is taken.
 */
import { detectWithin } from "../content/detect.ts";
import type { VisibilityEnv } from "../content/visibility.ts";
import type { CapturedMessage } from "../shared/messages.ts";

export const MAX_USERNAME = 256;
export const MAX_PASSWORD = 1024;
/** At most one capture per this many milliseconds. */
export const MIN_GAP_MS = 2000;
const MAX_CONTAINER_LEVELS = 5;
const BUTTON_SELECTOR = 'button, input[type="submit"], input[type="button"], [role="button"]';
const PASSWORD_SELECTOR = 'input[type="password"], input[autocomplete*="password"]';

export type Extracted = Pick<CapturedMessage, "username" | "password" | "newPassword">;

const usable = (value: string | undefined): value is string =>
  value !== undefined && value.length >= 1 && value.length <= MAX_PASSWORD;

/**
 * The passwords of a login (one), a sign-up (a new password and its confirmation, equal) or a
 * change form (the current one, then the new one twice). Null when nothing usable was typed.
 */
export function passwordsOf(
  current: readonly string[],
  fresh: readonly string[],
): Pick<Extracted, "password" | "newPassword"> | null {
  let old = current[0];
  let news = fresh;
  if (current.length === 0 && fresh.length >= 3) {
    // A change form whose fields carry no autocomplete hint: the first one is the current password.
    old = fresh[0];
    news = fresh.slice(1);
  }
  if (news.length === 0) {
    return usable(old) ? { password: old, newPassword: null } : null;
  }
  const first = news[0];
  if (!usable(first) || !news.every((value) => value === first)) {
    return null;
  }
  if (old === undefined) {
    return { password: first, newPassword: null };
  }
  return usable(old) ? { password: old, newPassword: first } : null;
}

export function extract(scope: Element, env?: VisibilityEnv): Extracted | null {
  const root = scope.getRootNode();
  if (!(root instanceof Document || root instanceof ShadowRoot)) {
    return null;
  }
  const found = env === undefined ? detectWithin(root, scope) : detectWithin(root, scope, env);
  const passwords = passwordsOf(
    found.password.map((el) => el.value),
    found.newPassword.map((el) => el.value),
  );
  if (passwords === null) {
    return null;
  }
  const typed = found.username[0]?.value.trim() ?? "";
  return { username: typed === "" || typed.length > MAX_USERNAME ? null : typed, ...passwords };
}

/** The form around `start`, or the nearest ancestor (five levels) that holds a password field. */
export function containerOf(start: Element): Element | null {
  const form = start.closest("form");
  if (form !== null) {
    return form;
  }
  let node: Element | null = start.parentElement;
  // Never the whole page: with other forms on it, that would answer for the wrong one.
  for (let level = 0; node !== null && node !== document.body && level < MAX_CONTAINER_LEVELS; level++) {
    if (node.querySelector(PASSWORD_SELECTOR) !== null) {
      return node;
    }
    node = node.parentElement;
  }
  return null;
}

/** The control a click landed on, if it is one that submits (button, submit/button input, role=button). */
export function submitControl(target: EventTarget | undefined): Element | null {
  return target instanceof Element ? target.closest(BUTTON_SELECTOR) : null;
}

const NOT_TEXT = new Set(["button", "submit", "reset", "checkbox", "radio", "file", "image", "range", "color", "hidden"]);

/** A box the user types into: Enter in one of a form's boxes submits that form. */
export function isTextEntry(target: EventTarget | undefined): target is HTMLInputElement {
  return target instanceof HTMLInputElement && !NOT_TEXT.has(target.type);
}

/** At most one capture per `MIN_GAP_MS`: a click on a submit button and the Enter that follows count once. */
export class Throttle {
  #last = Number.NEGATIVE_INFINITY;
  readonly #now: () => number;

  constructor(now: () => number) {
    this.#now = now;
  }

  allow(): boolean {
    const now = this.#now();
    if (now - this.#last < MIN_GAP_MS) {
      return false;
    }
    this.#last = now;
    return true;
  }
}
