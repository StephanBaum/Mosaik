/** Mock SSH agent: off, like the Rust stub. */
import type { MockHandlers } from "./state.ts";

export const sshHandlers: MockHandlers = {
  ssh_agent_status: () => ({ enabled: false, running: false, endpoint: null, keyCount: 0 }),
};
