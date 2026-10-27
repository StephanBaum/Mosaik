/** Mock security report: no findings until the insights track adds them. */
import { requireUnlocked, type MockHandlers } from "./state.ts";

export const insightHandlers: MockHandlers = {
  security_report: (state) => {
    requireUnlocked(state);
    return {
      generatedAt: state.now(),
      itemsChecked: state.items.length,
      counts: { weak: 0, reused: 0, old: 0, missingTotp: 0, insecureUrl: 0 },
      findings: [],
    };
  },
};
