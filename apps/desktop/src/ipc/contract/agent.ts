/** Agent access contract: mirrors `src-tauri/src/contract/agent.rs` (ADR-0159, ADR-0160). */
import type { ItemId, VaultId } from "./common.ts";
import type { SecretRef } from "./items.ts";

export const agentInjectKinds = ["bearer", "header", "basic", "query"] as const;
export type AgentInjectKind = (typeof agentInjectKinds)[number];

export const agentBasicUserKinds = ["literal", "field", "secretIsUser"] as const;
export type AgentBasicUserKind = (typeof agentBasicUserKinds)[number];

export interface AgentBasicUserView {
  kind: AgentBasicUserKind;
  /** The literal or the text field key; `null` for `secretIsUser`. */
  value: string | null;
}

/** How a binding places the value. Only the members of `kind` are set. */
export interface AgentInjectView {
  kind: AgentInjectKind;
  headerName: string | null;
  headerPrefix: string | null;
  basicUser: AgentBasicUserView | null;
  queryName: string | null;
}

export interface AgentBindingSnapshot {
  origins: string[];
  inject: AgentInjectKind;
}

/** One binding of an item. Never a value. */
export interface AgentBindingView {
  elementId: string;
  /** A `field` or a `customField`. */
  field: SecretRef;
  origins: string[];
  inject: AgentInjectView;
  env: string | null;
  /** The user confirmed exactly this binding on this device. */
  confirmed: boolean;
  /** The last confirmed state; `null` if never confirmed here. */
  lastConfirmed: AgentBindingSnapshot | null;
  /** An agent could use it now. */
  usable: boolean;
}

export type AgentBindingChange =
  | { op: "add"; field: SecretRef; origins: string[]; inject: AgentInjectView; env: string | null }
  | {
      op: "update";
      elementId: string;
      field: SecretRef;
      origins: string[];
      inject: AgentInjectView;
      env: string | null;
    }
  | { op: "remove"; elementId: string };

export interface AgentBindingConfirmRequest {
  vaultId: VaultId;
  itemId: ItemId;
  elementId: string;
}
