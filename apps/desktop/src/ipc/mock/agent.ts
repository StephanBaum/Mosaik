/**
 * Mock agent access (ADR-0159): the setting and the bindings of items. No value exists here:
 * the mock keeps names, destinations and states, like the real contract.
 */
import type {
  AgentBindingChange,
  AgentBindingView,
  ItemDetail,
} from "../contract/index.ts";
import { fail, requireUnlocked, type MockHandlers, type MockItem, type MockState } from "./state.ts";

export interface MockAgent {
  /** The setting `agentAccessEnabled`. */
  enabled: boolean;
}

export function defaultAgent(): MockAgent {
  return { enabled: false };
}

const MIN_VALUE_LENGTH = 12;

function canonicalOrigin(input: string): string {
  let url: URL;
  try {
    url = new URL(input.includes("://") ? input : `https://${input}`);
  } catch {
    return fail("agentOriginInvalid");
  }
  if (url.protocol !== "https:" || url.username !== "" || url.password !== "" || url.pathname !== "/" || url.search !== "" || url.hash !== "") {
    return fail("agentOriginInvalid");
  }
  return url.origin;
}

function fieldValue(item: MockItem, change: { field: AgentBindingView["field"] }): string | undefined {
  const field = change.field;
  if (field.kind === "field") {
    return item.secrets[field.field];
  }
  return field.kind === "customField" ? item.customSecrets[field.elementId] : undefined;
}

function checkChange(item: MockItem, change: Exclude<AgentBindingChange, { op: "remove" }>): string[] {
  if (change.field.kind === "history" || change.origins.length === 0 || change.origins.length > 16) {
    return fail("agentBindingInvalid");
  }
  const origins = change.origins.map(canonicalOrigin);
  const value = fieldValue(item, change);
  if (value === undefined || value.length < MIN_VALUE_LENGTH || /[\r\n\0]/.test(value)) {
    return fail("agentValueUnusable");
  }
  return origins;
}

/** Applies a draft's binding changes; a binding saved here is confirmed on this device. */
export function applyBindings(state: MockState, item: MockItem, changes: AgentBindingChange[]): void {
  if (changes.length === 0) {
    return;
  }
  if (!state.agent.enabled) {
    fail("agentAccessOff");
  }
  const bindings = item.detail.agentBindings;
  for (const change of changes) {
    if (change.op === "remove") {
      if (!bindings.some((each) => each.elementId === change.elementId)) {
        fail("agentBindingInvalid");
      }
      item.detail.agentBindings = item.detail.agentBindings.filter((each) => each.elementId !== change.elementId);
      continue;
    }
    const origins = checkChange(item, change);
    const view: AgentBindingView = {
      elementId: change.op === "update" ? change.elementId : `b${state.nextId()}`,
      field: change.field,
      origins,
      inject: change.inject,
      env: change.env,
      confirmed: true,
      lastConfirmed: null,
      usable: true,
    };
    if (change.op === "update") {
      if (!bindings.some((each) => each.elementId === change.elementId)) {
        fail("agentBindingInvalid");
      }
      item.detail.agentBindings = bindings.map((each) => (each.elementId === change.elementId ? view : each));
    } else {
      item.detail.agentBindings = [...bindings, view];
    }
  }
}

/** The item as the webview sees it: no binding while agent access is off. */
export function viewOf(state: MockState, item: MockItem): ItemDetail {
  return state.agent.enabled ? item.detail : { ...item.detail, agentBindings: [] };
}

export const agentHandlers: MockHandlers = {
  agent_binding_confirm: (state, request) => {
    requireUnlocked(state);
    if (!state.agent.enabled) {
      fail("agentAccessOff");
    }
    const item =
      state.items.find(
        (each) =>
          each.detail.summary.vaultId === request.vaultId && each.detail.summary.itemId === request.itemId,
      ) ?? fail("itemNotFound");
    const binding = item.detail.agentBindings.find((each) => each.elementId === request.elementId);
    if (binding === undefined) {
      fail("agentBindingInvalid");
    }
    item.detail.agentBindings = item.detail.agentBindings.map((each) =>
      each.elementId === request.elementId ? { ...each, confirmed: true, lastConfirmed: null, usable: true } : each,
    );
    return viewOf(state, item);
  },
};
