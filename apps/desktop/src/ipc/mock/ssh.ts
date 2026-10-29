/** Mock SSH agent: on/off, a status with the sample endpoint, and one approval request. */
import type { SshAgentStatus } from "../contract/index.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

const ENDPOINT = "/run/user/1000/mosaic/agent.sock";
const APPROVAL_DELAY_MS = 500;
const APPROVAL_TTL_MS = 25_000;

function statusOf(state: MockState): SshAgentStatus {
  const running = state.sshEnabled && state.unlocked;
  return {
    enabled: state.sshEnabled,
    running,
    endpoint: running ? ENDPOINT : null,
    keyCount: running ? 1 : 0,
    skipped: [],
    error: null,
  };
}

/** Called after an unlock: with `?sshRequest=1` the agent asks for one approval. */
export function startAgent(state: MockState): void {
  if (!state.sshEnabled && !state.sshRequest) {
    return;
  }
  state.sshEnabled = true;
  if (!state.sshRequest) {
    return;
  }
  state.sshRequest = false;
  setTimeout(() => {
    const requestedAt = state.now();
    state.emit({
      type: "sshApprovalRequested",
      request: {
        requestId: "ssh-1",
        vaultId: state.vaults[0]?.vaultId ?? "",
        itemId: state.items[0]?.detail.summary.itemId ?? "",
        keyTitle: "Deploy key",
        fingerprint: "SHA256:uNiVztksCsDhcc0u9e8BujQXVUpKZIDTMczCvj3tD2s",
        client: { processName: "git", pid: 4242 },
        requestedAt,
        expiresAt: requestedAt + APPROVAL_TTL_MS,
        canRemember: true,
      },
    });
  }, APPROVAL_DELAY_MS);
}

export const sshHandlers: MockHandlers = {
  ssh_agent_status: (state) => statusOf(state),

  ssh_agent_set_enabled: (state, request) => {
    if (request.enabled) {
      requireUnlocked(state);
    }
    state.sshEnabled = request.enabled;
    return statusOf(state);
  },

  ssh_respond: (state, request) => {
    if (request.requestId !== "ssh-1") {
      fail("sshRequestExpired");
    }
    state.emit({ type: "sshApprovalClosed", requestId: request.requestId, reason: "answered" });
    return null;
  },
};
