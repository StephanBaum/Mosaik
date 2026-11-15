/**
 * Mock agent access (ADR-0159): the setting, the paired clients, grants, the activity log, the
 * pairing and access dialogs, and the bindings of items. No value exists here: the mock keeps
 * names, destinations and states, like the real contract. `?agent=on` starts with agent access
 * on, one paired client, one grant and a few log entries; `&agent=pair` and `&agent=access` also
 * open the pairing or the access dialog 500 ms after the app subscribes (the 1 s Allow guard is
 * enforced here too, like in Rust).
 */
import type {
  AgentAccessRequest,
  AgentAccessStatus,
  AgentGrantView,
  AgentLogView,
  AgentPairingRequest,
  AgentPairingView,
} from "../contract/index.ts";
import { note, viewOf } from "./agentBindings.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

const REQUEST_DELAY_MS = 500;
const DIALOG_TTL_MS = 120_000;
const ALLOW_GUARD_MS = 1000;
const PAIR_REQUEST_ID = "a1b2c3d4e5f60718293a4b5c6d7e8f90";
const ACCESS_REQUEST_ID = "0f9e8d7c6b5a49382716f5e4d3c2b1a0";
const CLIENT_ID = "0123456789abcdef0123456789abcdef";
const MAX_LOG_LIMIT = 200;

export type AgentVariant = "on" | "pair" | "access";

export interface MockAgent {
  /** The setting `agentAccessEnabled`. */
  enabled: boolean;
  pairings: AgentPairingView[];
  grants: AgentGrantView[];
  /** Newest first. */
  log: AgentLogView[];
  cliPath: string | null;
  inFlight: number;
  /** Open a dialog after the first subscription (`&agent=pair|access`). */
  openOnStart: "pair" | "access" | null;
  pairing: AgentPairingRequest | null;
  access: AgentAccessRequest | null;
  nextGrantId: number;
}

export function defaultAgent(): MockAgent {
  return {
    enabled: false,
    pairings: [],
    grants: [],
    log: [],
    cliPath: "/usr/local/bin/mosaic",
    inFlight: 0,
    openOnStart: null,
    pairing: null,
    access: null,
    nextGrantId: 1,
  };
}

/** The state `?agent=` starts from. */
export function seedAgent(variant: AgentVariant | undefined, now: number): MockAgent {
  const agent = defaultAgent();
  if (variant === undefined) {
    return agent;
  }
  agent.enabled = true;
  agent.pairings = [
    {
      pairingId: CLIENT_ID,
      name: "Claude Code",
      pairedAt: now - 2 * 86_400_000,
      lastUsedAt: now - 3_600_000,
    },
  ];
  agent.grants = [
    {
      grantId: 1,
      pairingId: CLIENT_ID,
      pairingName: "Claude Code",
      itemTitle: "Example API",
      vaultName: "Personal",
      fieldLabel: "api_key.secret",
      state: "until",
      expiresAt: now + 600_000,
    },
  ];
  agent.nextGrantId = 2;
  agent.log = [
    {
      atMs: now - 3_600_000,
      kind: "request",
      pairingId: CLIENT_ID,
      pairingName: "Claude Code",
      itemId: null,
      itemTitle: "Example API",
      fieldLabel: "api_key.secret",
      method: "GET",
      origin: "https://api.example.com",
      path: "/v1/invoices",
      status: 200,
      bytes: 2048,
      masked: 0,
      code: null,
      grant: null,
      origins: null,
      inject: null,
    },
    {
      atMs: now - 3_700_000,
      kind: "refused",
      pairingId: CLIENT_ID,
      pairingName: "Claude Code",
      itemId: null,
      itemTitle: "Example API",
      fieldLabel: "api_key.secret",
      method: "POST",
      origin: "https://other.example.net",
      path: "/steal",
      status: null,
      bytes: null,
      masked: null,
      code: "destinationNotAllowed",
      grant: null,
      origins: null,
      inject: null,
    },
  ];
  agent.openOnStart = variant === "pair" || variant === "access" ? variant : null;
  return agent;
}

function statusOf(state: MockState): AgentAccessStatus {
  const agent = state.agent;
  return {
    enabled: agent.enabled,
    endpoint: agent.enabled ? "running" : "stopped",
    pairings: agent.pairings,
    grants: state.unlocked ? agent.grants : [],
    inFlight: agent.inFlight,
    cliPath: agent.cliPath,
  };
}

function pairingRequest(state: MockState): AgentPairingRequest {
  const requestedAt = state.now();
  return {
    requestId: PAIR_REQUEST_ID,
    name: "Cursor",
    code: "K7M2QX",
    requestedAt,
    expiresAt: requestedAt + DIALOG_TTL_MS,
  };
}

function accessRequest(state: MockState): AgentAccessRequest {
  const requestedAt = state.now();
  return {
    requestId: ACCESS_REQUEST_ID,
    pairingName: "Claude Code",
    itemTitle: "Example API",
    vaultName: state.vaults[0]?.name ?? "Personal",
    fieldLabel: "api_key.secret",
    origins: ["https://api.example.com"],
    inject: "bearer",
    reason: "List the open invoices",
    waiting: 0,
    requestedAt,
    expiresAt: requestedAt + DIALOG_TTL_MS,
  };
}

/** Called after the app subscribes: opens the dialog the URL asked for, once, if unlocked. */
export function startAgent(state: MockState): void {
  const agent = state.agent;
  const open = agent.openOnStart;
  if (open === null) {
    return;
  }
  agent.openOnStart = null;
  setTimeout(() => {
    if (!state.unlocked) {
      agent.openOnStart = open;
      return;
    }
    if (open === "pair") {
      agent.pairing = pairingRequest(state);
      state.emit({ type: "agentPairingRequested", request: agent.pairing });
    } else {
      agent.access = accessRequest(state);
      state.emit({ type: "agentAccessRequested", request: agent.access });
    }
  }, REQUEST_DELAY_MS);
}

function revoke(state: MockState, grantId: number): void {
  const agent = state.agent;
  const grant = agent.grants.find((each) => each.grantId === grantId);
  if (grant === undefined) {
    return;
  }
  agent.grants = agent.grants.filter((each) => each.grantId !== grantId);
  note(state, {
    kind: "revoked",
    pairingId: grant.pairingId,
    pairingName: grant.pairingName,
    itemTitle: grant.itemTitle,
    fieldLabel: grant.fieldLabel,
  });
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
    note(state, {
      kind: "bindingConfirmed",
      itemTitle: item.detail.summary.title,
      origins: binding.origins,
      inject: binding.inject.kind,
    });
    return viewOf(state, item);
  },

  agent_access_status: (state) => statusOf(state),

  agent_access_set_enabled: (state, request) => {
    if (request.enabled) {
      requireUnlocked(state);
    } else {
      state.agent.grants = [];
      note(state, { kind: "stoppedAll" });
    }
    state.agent.enabled = request.enabled;
    state.emit({ type: "agentStateChanged" });
    return statusOf(state);
  },

  agent_pairing_respond: (state, request) => {
    requireUnlocked(state);
    const agent = state.agent;
    const open = agent.pairing;
    if (open === null || open.requestId !== request.requestId) {
      fail("agentRequestExpired");
    }
    if (request.decision === "allow") {
      if (state.now() - open.requestedAt < ALLOW_GUARD_MS) {
        fail("agentAllowTooSoon");
      }
      const pairing: AgentPairingView = {
        pairingId: "fedcba9876543210fedcba9876543210",
        name: open.name,
        pairedAt: state.now(),
        lastUsedAt: null,
      };
      agent.pairings = [...agent.pairings, pairing];
      note(state, { kind: "paired", pairingId: pairing.pairingId, pairingName: pairing.name });
      state.emit({ type: "agentStateChanged" });
    }
    agent.pairing = null;
    state.emit({ type: "agentPairingClosed", requestId: open.requestId, reason: "answered" });
    return null;
  },

  agent_pairing_remove: (state, request) => {
    const agent = state.agent;
    const pairing = agent.pairings.find((each) => each.pairingId === request.pairingId);
    if (pairing === undefined) {
      fail("agentPairingNotFound");
    }
    agent.pairings = agent.pairings.filter((each) => each.pairingId !== request.pairingId);
    agent.grants = agent.grants.filter((each) => each.pairingId !== request.pairingId);
    note(state, { kind: "pairingRemoved", pairingId: pairing.pairingId, pairingName: pairing.name });
    state.emit({ type: "agentStateChanged" });
    return statusOf(state);
  },

  agent_access_respond: (state, request) => {
    requireUnlocked(state);
    const agent = state.agent;
    const open = agent.access;
    if (open === null || open.requestId !== request.requestId) {
      fail("agentRequestExpired");
    }
    if (request.decision !== "deny") {
      if (state.now() - open.requestedAt < ALLOW_GUARD_MS) {
        fail("agentAllowTooSoon");
      }
      const state15 = request.decision === "allow15m" || request.decision === "allow1h";
      const minutes = request.decision === "allow1h" ? 60 : 15;
      agent.grants = [
        ...agent.grants,
        {
          grantId: agent.nextGrantId++,
          pairingId: CLIENT_ID,
          pairingName: open.pairingName,
          itemTitle: open.itemTitle,
          vaultName: open.vaultName,
          fieldLabel: open.fieldLabel,
          state: state15 ? "until" : request.decision === "allowOnce" ? "once" : "untilLock",
          expiresAt: state15 ? state.now() + minutes * 60_000 : null,
        },
      ];
      note(state, { kind: "accessGranted", pairingName: open.pairingName, itemTitle: open.itemTitle });
      state.emit({ type: "agentStateChanged" });
    } else {
      note(state, { kind: "accessDenied", pairingName: open.pairingName, itemTitle: open.itemTitle });
    }
    agent.access = null;
    state.emit({ type: "agentAccessClosed", requestId: open.requestId, reason: "answered" });
    return null;
  },

  agent_grant_revoke: (state, request) => {
    revoke(state, request.grantId);
    state.emit({ type: "agentStateChanged" });
    return statusOf(state);
  },

  agent_stop_all: (state) => {
    state.agent.grants = [];
    note(state, { kind: "stoppedAll" });
    state.emit({ type: "agentStateChanged" });
    return statusOf(state);
  },

  agent_log: (state, request) => {
    requireUnlocked(state);
    const limit = Math.min(Math.max(request.limit, 1), MAX_LOG_LIMIT);
    const before = request.beforeMs ?? Number.POSITIVE_INFINITY;
    const entries = state.agent.log
      .filter((entry) => entry.atMs < before)
      .filter((entry) => request.pairingId === null || entry.pairingId === request.pairingId)
      .filter((entry) => request.itemId === null || entry.itemId === request.itemId)
      .slice(0, limit);
    return { entries, skipped: 0 };
  },
};
