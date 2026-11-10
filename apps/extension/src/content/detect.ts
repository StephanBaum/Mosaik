/**
 * Finds the login fields of a page. Pure: it reads the DOM it is given (a `Document` or a
 * `ShadowRoot`, walking open shadow roots to depth 8) and changes nothing.
 *
 * Attributes are read with `getAttribute`, never as named properties of a form, so a page
 * cannot answer for a field by naming an element `elements` or `type` (DOM clobbering).
 */
import { domEnv, isFillable, type VisibilityEnv } from "./visibility.ts";

export const MAX_SHADOW_DEPTH = 8;
const MAX_NEARBY_LEVELS = 5;
const SEGMENT_MIN = 4;
const SEGMENT_MAX = 8;

export interface Detection {
  /** Username (or email) fields: at most one. */
  username: HTMLInputElement[];
  /** Current-password fields. */
  password: HTMLInputElement[];
  /** New-password fields (sign-up, change form). */
  newPassword: HTMLInputElement[];
  /** One code field, or the digit boxes of a segmented code in order. */
  otp: HTMLInputElement[];
  /** The element with focus, looking into open shadow roots. */
  active: Element | null;
}

type Kind = "password" | "otp" | "text" | "search" | "other";

interface Field {
  el: HTMLInputElement;
  kind: Kind;
  tokens: string[];
  type: string;
}

const SKIP_TYPES = new Set([
  "hidden",
  "submit",
  "button",
  "checkbox",
  "radio",
  "file",
  "image",
  "reset",
  "range",
  "color",
]);
const TEXT_TYPES = new Set(["text", "email", "tel", "url", "number"]);
const SEARCH_RE = /search|query|^q$/i;
const NEW_RE = /new|confirm|repeat|retype|neu|wiederhol|bestätig/i;
const USERNAME_RE = /user|email|login|account|identifier|benutzer/i;
const OTP_RE =
  /otp|totp|one.?time|2fa|mfa|verification.?code|auth.?code|security.?code|einmal|bestätigungscode/i;

const attr = (el: Element, name: string): string => el.getAttribute(name) ?? "";

function tokensOf(el: Element): string[] {
  return attr(el, "autocomplete").toLowerCase().split(/\s+/).filter(Boolean);
}

function nameOrId(el: Element): string {
  return `${attr(el, "name")} ${attr(el, "id")}`;
}

function looksNumeric(el: Element): boolean {
  const maxLength = Number.parseInt(attr(el, "maxlength"), 10);
  const pattern = attr(el, "pattern");
  return (
    attr(el, "inputmode").toLowerCase() === "numeric" ||
    (maxLength >= 6 && maxLength <= 8) ||
    /^(\[0-9\]|\\d)/.test(pattern)
  );
}

function looksLikeOtp(el: Element): boolean {
  const text = `${nameOrId(el)} ${attr(el, "aria-label")} ${attr(el, "placeholder")}`;
  return OTP_RE.test(text) && looksNumeric(el);
}

function classify(el: HTMLInputElement, type: string, tokens: string[]): Kind {
  if (tokens.includes("one-time-code") || looksLikeOtp(el)) {
    return "otp";
  }
  if (type === "password" || tokens.includes("current-password") || tokens.includes("new-password")) {
    return "password";
  }
  const searchy = [attr(el, "name"), attr(el, "id")].some((value) => SEARCH_RE.test(value));
  if (type === "search" || attr(el, "role") === "search" || searchy) {
    return "search";
  }
  return TEXT_TYPES.has(type) ? "text" : "other";
}

function describeField(el: HTMLInputElement): Field | null {
  const type = el.type.toLowerCase();
  if (SKIP_TYPES.has(type) || el.matches(":disabled") || el.readOnly) {
    return null;
  }
  const tokens = tokensOf(el);
  return { el, kind: classify(el, type, tokens), tokens, type };
}

/** Every `input` in `root` and its open shadow roots, with the tree each one lives in. */
export function collectInputs(root: Document | ShadowRoot, depth: number, out: HTMLInputElement[]): void {
  out.push(...root.querySelectorAll("input"));
  if (depth >= MAX_SHADOW_DEPTH) {
    return;
  }
  for (const host of root.querySelectorAll("*")) {
    if (host.shadowRoot !== null) {
      collectInputs(host.shadowRoot, depth + 1, out);
    }
  }
}

/** The element with focus, looking into open shadow roots. */
export function deepActiveElement(root: Document | ShadowRoot): Element | null {
  let active = root.activeElement;
  for (let depth = 0; depth < MAX_SHADOW_DEPTH && active?.shadowRoot; depth++) {
    const inner: Element | null = active.shadowRoot.activeElement;
    if (inner === null) {
      break;
    }
    active = inner;
  }
  return active;
}

/** Fields that share a form (or, without one, a tree), in document order. */
function groupFields(inputs: readonly HTMLInputElement[], env: VisibilityEnv): Field[][] {
  const groups = new Map<object, Field[]>();
  for (const input of inputs) {
    const field = describeField(input);
    if (field === null || !isFillable(input, env)) {
      continue;
    }
    const key: object = input.form ?? input.getRootNode();
    const group = groups.get(key);
    if (group === undefined) {
      groups.set(key, [field]);
    } else {
      group.push(field);
    }
  }
  return [...groups.values()];
}

/** 4 to 8 adjacent one-character boxes are one code split into digits. */
function findSegments(fields: readonly Field[]): HTMLInputElement[] {
  let run: HTMLInputElement[] = [];
  const flush = (): HTMLInputElement[] | null =>
    run.length >= SEGMENT_MIN && run.length <= SEGMENT_MAX ? run : null;
  for (const field of fields) {
    const single = attr(field.el, "maxlength") === "1" && field.kind !== "password";
    if (single) {
      run.push(field.el);
      continue;
    }
    const found = flush();
    if (found !== null) {
      return found;
    }
    run = [];
  }
  return flush() ?? [];
}

function isNewPassword(field: Field, passwords: readonly Field[]): boolean {
  if (field.tokens.includes("current-password")) {
    return false;
  }
  return (
    field.tokens.includes("new-password") || passwords.length >= 2 || NEW_RE.test(nameOrId(field.el))
  );
}

/** Whether `candidate` sits within five ancestor levels of `anchor`'s own container. */
function isNearby(anchor: Element, candidate: Element): boolean {
  let node: Element | null = anchor.parentElement;
  for (let level = 0; node !== null && level < MAX_NEARBY_LEVELS; level++) {
    if (node.contains(candidate)) {
      return true;
    }
    node = node.parentElement;
  }
  return false;
}

function pickUsername(fields: readonly Field[], passwords: readonly Field[]): HTMLInputElement[] {
  const first = passwords[0];
  if (first === undefined) {
    return [];
  }
  const formless = first.el.form === null;
  const candidates = fields.filter(
    (f) => f.kind === "text" && f.type !== "number" && (!formless || isNearby(first.el, f.el)),
  );
  const declared = candidates.find((f) => f.tokens.includes("username") || f.tokens.includes("email"));
  if (declared !== undefined) {
    return [declared.el];
  }
  const index = fields.indexOf(first);
  const before = candidates.filter((f) => fields.indexOf(f) < index);
  const nearest = before.at(-1);
  return nearest === undefined ? [] : [nearest.el];
}

/** Whether a field says it holds a user name or an e-mail address (also used by the change-form heuristic). */
export function looksLikeUsername(el: HTMLInputElement): boolean {
  const tokens = tokensOf(el);
  return tokens.includes("username") || tokens.includes("email") || el.type.toLowerCase() === "email" || USERNAME_RE.test(nameOrId(el));
}

/** A lone field on a page that asks for the username first. */
function pickUsernameFirst(fields: readonly Field[]): HTMLInputElement[] {
  const texts = fields.filter((f) => f.kind === "text" && f.type !== "number");
  const only = texts.length === 1 ? texts[0] : undefined;
  if (only === undefined) {
    return [];
  }
  const declared = only.tokens.includes("username") || only.tokens.includes("email");
  return declared || only.type === "email" || USERNAME_RE.test(nameOrId(only.el)) ? [only.el] : [];
}

function analyze(fields: readonly Field[], active: Element | null): Detection {
  const segments = findSegments(fields);
  const rest = fields.filter((f) => !segments.includes(f.el));
  const passwords = rest.filter((f) => f.kind === "password");
  const codes = rest.filter((f) => f.kind === "otp");
  const isNew = (f: Field): boolean => isNewPassword(f, passwords);
  return {
    username:
      passwords.length > 0 ? pickUsername(rest, passwords) : pickUsernameFirst(rest.filter((f) => f.kind !== "otp")),
    password: passwords.filter((f) => !isNew(f)).map((f) => f.el),
    newPassword: passwords.filter(isNew).map((f) => f.el),
    otp: segments.length > 0 ? segments : codes.slice(0, 1).map((f) => f.el),
    active,
  };
}

const has = (d: Detection, key: "username" | "password" | "newPassword" | "otp"): boolean =>
  d[key].length > 0;

const found = (d: Detection | undefined): d is Detection =>
  d !== undefined && (d.username.length > 0 || d.password.length > 0 || d.newPassword.length > 0 || d.otp.length > 0);

/** The form the user is working in: the one with focus, else login, username-only, code, sign-up. */
function choose(candidates: readonly Detection[], groups: readonly Field[][], active: Element | null): Detection {
  const focused = candidates[groups.findIndex((fields) => fields.some((f) => f.el === active))];
  const picked =
    (found(focused) ? focused : undefined) ??
    candidates.find((d) => has(d, "password")) ??
    candidates.find((d) => has(d, "username")) ??
    candidates.find((d) => has(d, "otp")) ??
    candidates.find((d) => has(d, "newPassword"));
  return picked ?? { username: [], password: [], newPassword: [], otp: [], active };
}

function detectInputs(inputs: readonly HTMLInputElement[], root: Document | ShadowRoot, env: VisibilityEnv): Detection {
  const active = deepActiveElement(root);
  const groups = groupFields(inputs, env);
  const candidates = groups.map((fields) => analyze(fields, active));
  return choose(candidates, groups, active);
}

/** Detects the login form of `root` (see docs/EXTENSION_SECURITY.md, "field detection"). */
export function detect(root: Document | ShadowRoot, env: VisibilityEnv = domEnv): Detection {
  const inputs: HTMLInputElement[] = [];
  collectInputs(root, 0, inputs);
  return detectInputs(inputs, root, env);
}

/**
 * Like `detect`, but only the fields inside `scope` (the form that was submitted): what the
 * save capture reads, so another form on the page cannot answer for it.
 */
export function detectWithin(root: Document | ShadowRoot, scope: Element, env: VisibilityEnv = domEnv): Detection {
  const inputs: HTMLInputElement[] = [];
  collectInputs(root, 0, inputs);
  return detectInputs(
    inputs.filter((input) => scope.contains(input)),
    root,
    env,
  );
}

export interface FieldCounts {
  username: number;
  password: number;
  newPassword: number;
  otp: number;
}

export function countFields(d: Detection): FieldCounts {
  return {
    username: d.username.length,
    password: d.password.length,
    newPassword: d.newPassword.length,
    otp: d.otp.length > 0 ? 1 : 0,
  };
}

/** Whether focus is on one of the detected fields. */
export function hasActiveField(d: Detection): boolean {
  const { active } = d;
  return (
    active !== null &&
    [...d.username, ...d.password, ...d.newPassword, ...d.otp].some((el) => el === active)
  );
}
