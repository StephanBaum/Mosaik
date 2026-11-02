/**
 * Gives `HoloButton`s the shared renderer. Without a provider (unit tests, isolated
 * renders) buttons get a no-op renderer and keep the static fallback fill.
 */
import { createContext, useContext, useEffect, useMemo, useRef, type ReactNode } from "react";
import { createHoloRenderer, type HoloRenderer, type RegisterOptions } from "./renderer.ts";

export interface HoloApi {
  register(button: HTMLButtonElement, foil: HTMLCanvasElement, options?: RegisterOptions): () => void;
  /** Paints a static decorative foil (ADR-0027) into `canvas`. */
  registerSurface(canvas: HTMLCanvasElement): () => void;
}

const FALLBACK: HoloApi = { register: () => () => undefined, registerSurface: () => () => undefined };

const HoloContext = createContext<HoloApi>(FALLBACK);

export function useHolo(): HoloApi {
  return useContext(HoloContext);
}

/** Creates the renderer on the first registered button and lives as long as the app. */
export function HoloProvider({ dark, children }: { dark: boolean; children: ReactNode }) {
  const renderer = useRef<HoloRenderer | null>(null);
  const darkRef = useRef(dark);

  const api = useMemo<HoloApi>(() => {
    const shared = (): HoloRenderer => {
      if (renderer.current === null) {
        renderer.current = createHoloRenderer();
        renderer.current.setDark(darkRef.current);
      }
      return renderer.current;
    };
    return {
      register: (button, foil, options) => shared().register(button, foil, options),
      registerSurface: (canvas) => shared().registerSurface(canvas),
    };
  }, []);

  useEffect(() => {
    darkRef.current = dark;
    renderer.current?.setDark(dark);
  }, [dark]);

  useEffect(
    () => () => {
      renderer.current?.dispose();
      renderer.current = null;
    },
    [],
  );

  return <HoloContext value={api}>{children}</HoloContext>;
}
