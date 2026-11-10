/**
 * The change-form heuristic (docs/EXTENSION_SECURITY.md, "password change"). Pure: it reads the
 * DOM it is given and changes nothing. It decides, before any secret is requested, whether the
 * page holds a password change form and which fields are the current and the new password. It
 * refuses to guess: more or fewer fields than a change form has, fields that cannot be told apart
 * and a password-like field that is in neither class all make the form `ambiguous`, and then
 * nothing is requested and nothing is filled.
 *
 * The form is the one the user works in (the one with focus), else the one that holds a
 * new-password field; two such forms are `ambiguous`. In that form:
 * - *new* fields: `autocomplete` token `new-password`; or, when no password field carries an
 *   `autocomplete` token at all, exactly three password fields (current, new, confirmation); or a
 *   name, id or label that says new or confirm while the other password field says current or old;
 * - *current* field: `autocomplete` token `current-password`, or the one remaining password
 *   field of a form whose new fields came from `autocomplete`;
 * - `currentAndNew`: exactly one current and one or two new fields; `newOnly`: one or two new
 *   fields and no other password field;
 * - eligible fields: `type=password`, or `type=text` with an explicit `new-password` or
 *   `current-password` token (a show-password toggle) that is the only field of its role and does
 *   not look like a user name (the sign-in heuristic's test); enabled, not read-only, and visible
 *   to the user. Any other text field with such a token makes the form `ambiguous`. The user name
 *   field is never touched.
 */
import type { ChangeFormShape } from "../shared/messages.ts";
import { collectInputs, deepActiveElement, looksLikeUsername } from "./detect.ts";
import { domEnv, isFillable, type VisibilityEnv } from "./visibility.ts";

export interface ChangeClassification {
  shape: ChangeFormShape;
  /** The current-password field (one) of a `currentAndNew` form. */
  current: HTMLInputElement[];
  /** The new-password fields: the new password and, if there is one, its confirmation. */
  next: HTMLInputElement[];
}

const NONE: ChangeClassification = { shape: "none", current: [], next: [] };
const AMBIGUOUS: ChangeClassification = { shape: "ambiguous", current: [], next: [] };

/** Says "new" or "confirm" (English and German), on word starts of the normalized text. */
const NEW_RE = /(^|\s)(new|neu\w*|confirm\w*|repeat|retype|re ?enter|wiederhol\w*|best[aä]tig\w*)/;
/** Says "current" or "old" (English and German). */
const CURRENT_RE = /(^|\s)(current|old|aktuell\w*|alt|altes|bisherig\w*|vorherig\w*)(\s|$)/;
/** A text field that is named like a password: never silently skipped. */
const PASSWORD_LIKE_RE = /pass|pwd|kennwort/i;

const SKIP_TYPES = new Set(["hidden", "submit", "button", "checkbox", "radio", "file", "image", "reset", "range", "color"]);

const attr = (el: Element, name: string): string => el.getAttribute(name) ?? "";

function tokensOf(el: Element): string[] {
  return attr(el, "autocomplete").toLowerCase().split(/\s+/).filter(Boolean);
}

/** The text a person reads next to the field: attributes and label, lower-cased, words split. */
function textOf(el: HTMLInputElement): string {
  const labels = Array.from(el.labels ?? [], (label) => label.textContent).join(" ");
  const raw = [attr(el, "name"), attr(el, "id"), attr(el, "aria-label"), attr(el, "placeholder"), labels].join(" ");
  return raw
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/[_\-.[\]]+/g, " ")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

interface Candidate {
  el: HTMLInputElement;
  tokens: string[];
  /** `type=password`, or `type=text` with an explicit password token. */
  passwordType: boolean;
}

/** The inputs of a form that are, or might pass for, a password field. */
function passwordFields(inputs: readonly HTMLInputElement[]): Candidate[] {
  const out: Candidate[] = [];
  for (const el of inputs) {
    const type = el.type.toLowerCase();
    if (SKIP_TYPES.has(type)) {
      continue;
    }
    const tokens = tokensOf(el);
    const explicit = tokens.includes("new-password") || tokens.includes("current-password");
    if (type === "password") {
      out.push({ el, tokens, passwordType: true });
    } else if (type === "text" && explicit) {
      out.push({ el, tokens, passwordType: true });
    } else if (type !== "password" && PASSWORD_LIKE_RE.test(`${attr(el, "name")} ${attr(el, "id")}`)) {
      // A field that is named like a password but is not one the heuristic may fill.
      out.push({ el, tokens, passwordType: false });
    }
  }
  return out;
}

function hasNewToken(c: Candidate): boolean {
  return c.tokens.includes("new-password");
}

function hasCurrentToken(c: Candidate): boolean {
  return c.tokens.includes("current-password");
}

/** Fields that share a form (or, without one, a tree), in document order. */
function groupInputs(inputs: readonly HTMLInputElement[]): HTMLInputElement[][] {
  const groups = new Map<object, HTMLInputElement[]>();
  for (const input of inputs) {
    const key: object = input.form ?? input.getRootNode();
    const group = groups.get(key);
    if (group === undefined) {
      groups.set(key, [input]);
    } else {
      group.push(input);
    }
  }
  return [...groups.values()];
}

/**
 * Whether a group shows a sign of being a change form: a new-password token, a "new" or "confirm"
 * label, or more than one password field (a sign-in form has one).
 */
function holdsNewField(group: readonly HTMLInputElement[]): boolean {
  const fields = passwordFields(group);
  return fields.length >= 2 || fields.some((c) => hasNewToken(c) || (c.passwordType && NEW_RE.test(textOf(c.el))));
}

/** The form the user works in: the one with focus, else the one that holds a new-password field. */
function chooseForm(
  groups: readonly HTMLInputElement[][],
  active: Element | null,
): HTMLInputElement[] | "ambiguous" | null {
  const focused = groups.find((group) => group.some((el) => el === active) && passwordFields(group).length > 0);
  if (focused !== undefined) {
    return focused;
  }
  const holding = groups.filter(holdsNewField);
  if (holding.length > 1) {
    return "ambiguous";
  }
  return holding[0] ?? null;
}

function eligible(el: HTMLInputElement, env: VisibilityEnv): boolean {
  return !el.matches(":disabled") && !el.readOnly && isFillable(el, env);
}

interface Roles {
  current: Candidate[];
  next: Candidate[];
}

/** The roles by `autocomplete` token; the one remaining field is the current password. */
function rolesByToken(fields: readonly Candidate[]): Roles | null {
  if (fields.some((c) => hasNewToken(c) && hasCurrentToken(c))) {
    return null;
  }
  const next = fields.filter(hasNewToken);
  let current = fields.filter(hasCurrentToken);
  const rest = fields.filter((c) => !hasNewToken(c) && !hasCurrentToken(c));
  if (next.length > 0 && current.length === 0 && rest.length === 1) {
    current = rest;
  } else if (rest.length > 0) {
    return null;
  }
  return { current, next };
}

/** The roles by what the fields say (their names and labels). */
function rolesByText(fields: readonly Candidate[]): Roles | null {
  const next: Candidate[] = [];
  const current: Candidate[] = [];
  for (const field of fields) {
    const text = textOf(field.el);
    const isNew = NEW_RE.test(text);
    const isCurrent = CURRENT_RE.test(text);
    if (isNew === isCurrent) {
      // Both, or neither: the field cannot be placed.
      return null;
    }
    (isNew ? next : current).push(field);
  }
  return { current, next };
}

function rolesOf(fields: readonly Candidate[]): Roles | null {
  if (fields.some((c) => hasNewToken(c) || hasCurrentToken(c))) {
    return rolesByToken(fields);
  }
  const [first, second, third] = fields;
  if (fields.length === 3 && first !== undefined && second !== undefined && third !== undefined) {
    return { current: [first], next: [second, third] };
  }
  return rolesByText(fields);
}

/** Classifies the change form of `root` (a `Document` or an open `ShadowRoot`). */
export function classifyChangeForm(root: Document | ShadowRoot, env: VisibilityEnv = domEnv): ChangeClassification {
  const inputs: HTMLInputElement[] = [];
  collectInputs(root, 0, inputs);
  const chosen = chooseForm(groupInputs(inputs), deepActiveElement(root));
  if (chosen === "ambiguous") {
    return AMBIGUOUS;
  }
  if (chosen === null) {
    return NONE;
  }
  const fields = passwordFields(chosen);
  const roles = rolesOf(fields);
  if (roles === null) {
    return fields.length >= 2 ? AMBIGUOUS : NONE;
  }
  return shapeOf(roles, fields, env);
}

/** The shape of a form whose fields have roles: a sign-in form is `none`, anything odd `ambiguous`. */
function shapeOf(roles: Roles, fields: readonly Candidate[], env: VisibilityEnv): ChangeClassification {
  const { current, next } = roles;
  if (next.length === 0) {
    // A sign-in form, not a change form.
    return fields.length >= 2 ? AMBIGUOUS : NONE;
  }
  const placed = current.length + next.length;
  const shapeOk = next.length <= 2 && current.length <= 1 && placed === fields.length;
  // A text field with a password token is a show-password toggle only when nothing else could be
  // the field of its role and it does not read as a user name.
  const textOk = (c: Candidate): boolean =>
    c.el.type.toLowerCase() === "password" ||
    ((current.includes(c) ? current : next).length === 1 && !looksLikeUsername(c.el));
  const allUsable = [...current, ...next].every((c) => c.passwordType && textOk(c) && eligible(c.el, env));
  if (!shapeOk || !allUsable) {
    return AMBIGUOUS;
  }
  return {
    shape: current.length === 1 ? "currentAndNew" : "newOnly",
    current: current.map((c) => c.el),
    next: next.map((c) => c.el),
  };
}

/** Whether two classifications name the same shape and the very same fields, in the same roles. */
export function sameClassification(a: ChangeClassification, b: ChangeClassification): boolean {
  const same = (x: readonly HTMLInputElement[], y: readonly HTMLInputElement[]): boolean =>
    x.length === y.length && x.every((el, index) => el === y[index]);
  return a.shape === b.shape && same(a.current, b.current) && same(a.next, b.next);
}
