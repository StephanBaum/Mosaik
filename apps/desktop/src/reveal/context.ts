/** The hooks the rest of the app uses to talk to the reveal. No JSX, so it stays import-cheap. */
import { createContext, useContext } from "react";

export interface RevealControl {
  /**
   * Locking from the app: paints the vault into a sealed snapshot and unmounts it. Call it
   * from an event handler, right after sending the lock command. `false` means no animation
   * (unsupported, nothing to seal, or the vault is not showing): the swap is instant.
   */
  sealVault(): boolean;
  /** The lock failed or the locked session never came: put the vault back. */
  abortSeal(): void;
}

export const RevealControlContext = createContext<RevealControl>({
  sealVault: () => false,
  abortSeal: () => undefined,
});

export function useReveal(): RevealControl {
  return useContext(RevealControlContext);
}

export interface RevealLayerState {
  /** The layer is the one the user is in (not the inert layer of a running reveal). */
  active: boolean;
}

export const RevealLayerContext = createContext<RevealLayerState>({ active: true });

export function useRevealLayer(): RevealLayerState {
  return useContext(RevealLayerContext);
}

/**
 * Only the active layer carries the `main` landmark id (`#main`, the target of the skip
 * link), so the page never has two while a reveal runs.
 */
export function useLandmarkId(id: string): string | undefined {
  return useContext(RevealLayerContext).active ? id : undefined;
}
