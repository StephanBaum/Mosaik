/**
 * Gives the reveal its platform dependencies (ADR-0032): what the platform supports, the
 * clock, the reduced-motion setting and the one WebGL2 band. Tests inject their own.
 */
import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import type { Band } from "./band.ts";
import { defaultDeps, type RevealDeps, type RevealEnv } from "./env.ts";

export { detectSupport, type RevealDeps, type RevealEnv, type RevealSupport } from "./env.ts";

const RevealEnvContext = createContext<RevealEnv | null>(null);

interface Env extends RevealEnv {
  setDark(dark: boolean): void;
}

function createEnv(overrides: Partial<RevealDeps> | undefined, initialDark: boolean): Env {
  const deps: RevealDeps = { ...defaultDeps(), ...overrides };
  let dark = initialDark;
  let cached: { canvas: HTMLCanvasElement; band: Band | null } | null = null;
  return {
    deps,
    dark: () => dark,
    setDark(next) {
      dark = next;
    },
    band(canvas) {
      if (!deps.support.gl || canvas === null) {
        return null;
      }
      if (cached?.canvas !== canvas) {
        cached?.band?.dispose();
        cached = { canvas, band: deps.createBand(canvas) };
      }
      return cached.band;
    },
  };
}

export function RevealProvider({
  dark,
  deps,
  children,
}: {
  dark: boolean;
  deps?: Partial<RevealDeps>;
  children: ReactNode;
}) {
  const [env] = useState(() => createEnv(deps, dark));
  useEffect(() => {
    env.setDark(dark);
  }, [env, dark]);
  return <RevealEnvContext value={env}>{children}</RevealEnvContext>;
}

export function useRevealEnv(): RevealEnv {
  const env = useContext(RevealEnvContext);
  if (env === null) {
    throw new Error("useRevealEnv needs a RevealProvider");
  }
  return env;
}
