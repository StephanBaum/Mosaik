import { describe, expect, it } from "vitest";
import type { CreatedAccount } from "../../ipc/contract/index.ts";
import { flowReducer, initialFlow, type FlowAction, type FlowState } from "./flow.ts";

const SECRET_KEY = "M1-ABCDEF-GHJKLM-NPQRST-UVWXYZ-234567-ABCDEF-GHJKLM";

const account: CreatedAccount = {
  emergencyKit: {
    accountId: "019a2b3c-0000-7000-8000-000000000001",
    email: "me@example.com",
    secretKey: SECRET_KEY,
  },
  session: { state: "unlocked", accountId: "019a2b3c-0000-7000-8000-000000000001", email: "me@example.com", kitVerified: false },
};

function run(actions: readonly FlowAction[]): FlowState {
  return actions.reduce(flowReducer, initialFlow);
}

describe("onboarding flow", () => {
  it("keeps only the session and the kit after the account is created", () => {
    const state = run([{ type: "created", account }]);
    expect(state.session).toEqual(account.session);
    expect(state.kit).toEqual(account.emergencyKit);
    expect(state.step).toBe("kit");
  });

  it("drops the Secret Key once the kit is verified", () => {
    const state = run([{ type: "created", account }, { type: "kitSaved" }, { type: "verified" }]);
    expect(state.kit).toBeNull();
    expect(state.step).toBe("import");
    expect(JSON.stringify(state)).not.toContain(SECRET_KEY);
  });

  it("walks through import and done", () => {
    const base: FlowAction[] = [{ type: "created", account }, { type: "kitSaved" }, { type: "verified" }];
    expect(run([...base, { type: "skip" }]).step).toBe("done");
    const imported = run([...base, { type: "importStart" }]);
    expect(imported.step).toBe("importing");
    expect(flowReducer(imported, { type: "done" }).step).toBe("done");
  });
});
