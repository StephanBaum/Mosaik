/**
 * Agent access in the item editor's form (ADR-0159): one row per field the user marked for
 * agents, how it is built from the stored bindings and the binding changes it sends. The origin
 * check here is a mirror for display only (a canonical form and a warning); Rust decides what is
 * stored and refuses what it does not accept (`agentOriginInvalid`).
 */
import type {
  AgentBindingChange,
  AgentBindingView,
  AgentInjectView,
  ErrorCode,
  ItemDetail,
  SecretRef,
} from "../../ipc/contract/index.ts";

/** The most destinations one binding may list (the same limit as in Rust). */
export const MAX_AGENT_ORIGINS = 16;

export interface AgentRow {
  /** The marking is on. A row that is off and has no `elementId` is not sent. */
  enabled: boolean;
  /** The stored binding this row edits (`null` for one added in this editor). */
  elementId: string | null;
  /** The sites as typed. */
  origins: string[];
  inject: AgentInjectView;
  /** The variable name as typed; empty means none. */
  env: string;
}

/** Rows by the key of their field (`agentKey`). */
export type AgentRows = Record<string, AgentRow>;

export const BEARER: AgentInjectView = {
  kind: "bearer",
  headerName: null,
  headerPrefix: null,
  basicUser: null,
  queryName: null,
};

/** The identity of a bindable field: `field:<key>` or `custom:<element id>`. */
export function agentKey(field: SecretRef): string {
  if (field.kind === "field") {
    return `field:${field.field}`;
  }
  return field.kind === "customField" ? `custom:${field.elementId}` : `history:${String(field.index)}`;
}

export function newAgentRow(): AgentRow {
  return { enabled: true, elementId: null, origins: [""], inject: BEARER, env: "" };
}

function rowOf(binding: AgentBindingView): AgentRow {
  return {
    enabled: true,
    elementId: binding.elementId,
    origins: binding.origins,
    inject: binding.inject,
    env: binding.env ?? "",
  };
}

/** The rows of the bindings the item has now. */
export function initialAgentRows(editing: ItemDetail | null): AgentRows {
  const rows: AgentRows = {};
  for (const binding of editing?.agentBindings ?? []) {
    rows[agentKey(binding.field)] = rowOf(binding);
  }
  return rows;
}

function same(a: AgentRow, b: AgentRow): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

function specOf(row: AgentRow) {
  return {
    // Sent in the canonical form where the mirror knows it; Rust canonicalizes and decides.
    origins: row.origins
      .map((site) => viewSite(site).canonical ?? site.trim())
      .filter((site) => site !== ""),
    inject: row.inject,
    env: row.env.trim() === "" ? null : row.env.trim(),
  };
}

/**
 * The binding changes of the draft. `fields` resolves a key to its field (a secret field
 * of the kind or a custom field of the item); a stored binding whose field is gone is removed.
 */
export function agentChanges(
  form: AgentRows,
  initial: AgentRows,
  fieldOf: (key: string) => SecretRef | null,
): AgentBindingChange[] {
  const changes: AgentBindingChange[] = [];
  for (const [key, row] of Object.entries(form)) {
    const field = fieldOf(key);
    if (field === null) {
      continue;
    }
    const before = initial[key];
    if (!row.enabled) {
      if (row.elementId !== null) {
        changes.push({ op: "remove", elementId: row.elementId });
      }
    } else if (row.elementId === null) {
      changes.push({ op: "add", field, ...specOf(row) });
    } else if (before === undefined || !same(row, before)) {
      changes.push({ op: "update", elementId: row.elementId, field, ...specOf(row) });
    }
  }
  for (const [key, before] of Object.entries(initial)) {
    if (!(key in form) && before.elementId !== null) {
      changes.push({ op: "remove", elementId: before.elementId });
    }
  }
  return changes;
}

/** The errors a save can answer for the agent section (shown there, not at the bottom). */
export const AGENT_SAVE_ERRORS: readonly ErrorCode[] = [
  "agentOriginInvalid",
  "agentBindingInvalid",
  "agentValueUnusable",
  "agentVaultShared",
  "agentAccessOff",
];

export interface SiteView {
  /** The canonical origin Rust will store, if the text is a usable address. */
  canonical: string | null;
  /** The address has characters from another script or punycode: it may pass for another site. */
  lookalike: boolean;
}

/** A display mirror of `AgentOrigin::from_input`: https only, the site and nothing after it. */
export function viewSite(input: string): SiteView {
  const text = input.trim();
  if (text === "") {
    return { canonical: null, lookalike: false };
  }
  let url: URL;
  try {
    url = new URL(text.includes("://") ? text : `https://${text}`);
  } catch {
    return { canonical: null, lookalike: false };
  }
  const plain =
    url.protocol === "https:" &&
    url.username === "" &&
    url.password === "" &&
    url.pathname === "/" &&
    url.search === "" &&
    url.hash === "";
  // eslint-disable-next-line no-control-regex -- only ASCII is a plain host.
  const nonAscii = /[^\u0000-\u007f]/.test(text);
  return {
    canonical: plain ? url.origin : null,
    lookalike: nonAscii || url.hostname.split(".").some((label) => label.startsWith("xn--")),
  };
}
