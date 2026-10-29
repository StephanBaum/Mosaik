/**
 * A registry of revealed secrets. Every component that shows a secret registers a
 * function that hides it; `clearAll` runs on lock, when the window loses focus and when
 * a reveal times out (UX_GUIDELINES §8).
 */
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, type ReactNode } from "react";
import { useWindowFocused } from "../hooks/useWindowFocused.ts";

interface Secrets {
  /** Registers a hide function; returns the unregister function. */
  register: (hide: () => void) => () => void;
  /** Hides everything that is currently revealed. */
  clearAll: () => void;
}

const SecretsContext = createContext<Secrets | null>(null);

export function SecretsProvider({ children }: { children: ReactNode }) {
  const hiders = useRef(new Set<() => void>());
  const clearAll = useCallback(() => {
    for (const hide of hiders.current) {
      hide();
    }
  }, []);
  const register = useCallback((hide: () => void) => {
    hiders.current.add(hide);
    return () => {
      hiders.current.delete(hide);
    };
  }, []);
  const focused = useWindowFocused();
  useEffect(() => {
    if (!focused) {
      clearAll();
    }
  }, [focused, clearAll]);
  const value = useMemo(() => ({ register, clearAll }), [register, clearAll]);
  return <SecretsContext value={value}>{children}</SecretsContext>;
}

export function useSecrets(): Secrets {
  const context = useContext(SecretsContext);
  if (context === null) {
    throw new Error("useSecrets needs a SecretsProvider");
  }
  return context;
}
