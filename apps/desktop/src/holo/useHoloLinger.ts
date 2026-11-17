/**
 * Keeps a holo button's foil until its press ripple has run out when the click itself hands
 * the primary role to another button (ADR-0014). Example: "New item" opens the editor, whose
 * Save becomes the primary; swapping the top bar button at once cut its ripple off mid-wave.
 * For that one ripple the screen briefly shows two foils. Cosmetic only.
 */
import { useCallback, useEffect, useState } from "react";
import { RIPPLE } from "./effects.ts";

const RIPPLE_MS = Math.ceil(RIPPLE.duration * 1000);
const REDUCED_QUERY = "(prefers-reduced-motion: reduce)";

function reducedMotion(): boolean {
  return typeof window.matchMedia === "function" && window.matchMedia(REDUCED_QUERY).matches;
}

export interface HoloLinger {
  /** Whether the button wears the foil now. */
  holo: boolean;
  /** Call from the button's click handler, before the click changes the primary role. */
  pressed: () => void;
}

export function useHoloLinger(primary: boolean): HoloLinger {
  const [lingering, setLingering] = useState(false);

  useEffect(() => {
    if (!lingering) {
      return undefined;
    }
    const timer = window.setTimeout(() => {
      setLingering(false);
    }, RIPPLE_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [lingering]);

  const pressed = useCallback(() => {
    if (primary && !reducedMotion()) {
      setLingering(true);
    }
  }, [primary]);

  return { holo: primary || lingering, pressed };
}
