/**
 * A static, decorative foil accent (ADR-0027): the logo's fourth tile, the active nav bar,
 * completed setup progress and the empty-state tile. Painted once by the shared renderer,
 * never animated, hidden from assistive tech, and flat (the parent's fill shows) without
 * WebGL2, after a lost context, without a provider, or with reduced transparency.
 */
import { useEffect, useRef } from "react";
import { useHolo } from "../holo/HoloProvider.tsx";

export function HoloSurface({ className }: { className?: string }) {
  const holo = useHolo();
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (canvas.current === null) {
      return undefined;
    }
    return holo.registerSurface(canvas.current);
  }, [holo]);

  return (
    <canvas
      ref={canvas}
      className={className === undefined ? "holo-surface" : `holo-surface ${className}`}
      aria-hidden="true"
    />
  );
}
