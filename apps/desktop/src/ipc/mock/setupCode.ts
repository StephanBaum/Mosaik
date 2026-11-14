/**
 * Mock setup code for a phone: a fixed 29 x 29 matrix (three finder squares and a deterministic
 * fill, not a scannable code). The rules follow the backend: hosted accounts only, the master
 * password is checked and earns no re-authentication grant, the code lasts 60 s.
 */
import { fail, requireUnlocked, type MockHandlers } from "./state.ts";

const SIZE = 29;
const VISIBLE_MS = 60_000;

function finder(x: number, y: number, ox: number, oy: number): boolean | null {
  const dx = x - ox;
  const dy = y - oy;
  if (dx < 0 || dy < 0 || dx > 6 || dy > 6) {
    return null;
  }
  const edge = dx === 0 || dy === 0 || dx === 6 || dy === 6;
  const core = dx >= 2 && dx <= 4 && dy >= 2 && dy <= 4;
  return edge || core;
}

function mockModules(): boolean[][] {
  return Array.from({ length: SIZE }, (_, y) =>
    Array.from({ length: SIZE }, (_, x) => {
      const corner =
        finder(x, y, 0, 0) ?? finder(x, y, SIZE - 7, 0) ?? finder(x, y, 0, SIZE - 7);
      return corner ?? (x * 7 + y * 13 + x * y) % 3 === 0;
    }),
  );
}

export const setupCodeHandlers: MockHandlers = {
  setup_code_show: (state, request) => {
    requireUnlocked(state);
    if (state.sync.config?.kind !== "hosted") {
      fail("setupCodeNeedsServer");
    }
    if (request.password === "") {
      fail("emptyPassword");
    }
    if (request.password !== state.account?.password) {
      fail("wrongCredentials");
    }
    return { modules: mockModules(), size: SIZE, expiresAt: state.now() + VISIBLE_MS };
  },
};
