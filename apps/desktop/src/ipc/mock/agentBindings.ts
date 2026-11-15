/**
 * Bindings of items in the mock (ADR-0159): saving one in the editor confirms it on this device,
 * and the detail shows none while agent access is off. Values never leave the mock's item.
 */
import type {
  AgentBindingChange,
  AgentBindingView,
  AgentLogView,
  ItemDetail,
} from "../contract/index.ts";
import { fail, type MockItem, type MockState } from "./state.ts";

const MIN_VALUE_LENGTH = 12;

export function note(state: MockState, entry: Partial<AgentLogView> & Pick<AgentLogView, "kind">): void {
  state.agent.log = [
    {
      atMs: state.now(),
      pairingId: null,
      pairingName: null,
      itemId: null,
      itemTitle: null,
      fieldLabel: null,
      method: null,
      origin: null,
      path: null,
      status: null,
      bytes: null,
      masked: null,
      code: null,
      grant: null,
      origins: null,
      inject: null,
      ...entry,
    },
    ...state.agent.log,
  ];
}

// ----- bindings of items -------------------------------------------------------------------

function canonicalOrigin(input: string): string {
  let url: URL;
  try {
    url = new URL(input.includes("://") ? input : `https://${input}`);
  } catch {
    return fail("agentOriginInvalid");
  }
  if (
    url.protocol !== "https:" ||
    url.username !== "" ||
    url.password !== "" ||
    url.pathname !== "/" ||
    url.search !== "" ||
    url.hash !== ""
  ) {
    return fail("agentOriginInvalid");
  }
  return url.origin;
}

function fieldValue(item: MockItem, field: AgentBindingView["field"]): string | undefined {
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
  const value = fieldValue(item, change.field);
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
  for (const change of changes) {
    const bindings = item.detail.agentBindings;
    if (change.op === "remove") {
      if (!bindings.some((each) => each.elementId === change.elementId)) {
        fail("agentBindingInvalid");
      }
      item.detail.agentBindings = bindings.filter((each) => each.elementId !== change.elementId);
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
    note(state, {
      kind: "bindingConfirmed",
      itemTitle: item.detail.summary.title,
      origins,
      inject: change.inject.kind,
    });
  }
}

/** The item as the webview sees it: no binding while agent access is off. */
export function viewOf(state: MockState, item: MockItem): ItemDetail {
  return state.agent.enabled ? item.detail : { ...item.detail, agentBindings: [] };
}

