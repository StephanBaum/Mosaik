/**
 * The stage the screens sit on (ADR-0032). It keeps the layers of the current phase, runs the
 * liquid-glass unlock and lock reveals between them, and gives the vault a way to seal
 * itself before it locks. Layers are keyed children in a fixed slot order, so a layer that
 * survives a phase change keeps its React state.
 *
 * Lock design: the lock command goes out first. In the same task the live vault is painted
 * into a sealed snapshot (secrets excluded) and, in the same commit, unmounted. The closing
 * animation plays over the picture, with the real unlock screen live underneath.
 */
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  type ActionDispatch,
  type ReactNode,
  type RefObject,
} from "react";
import { useSecrets } from "../app/SecretsContext.tsx";
import { useLatest } from "../hooks/useLatest.ts";
import type { Band } from "./band.ts";
import {
  RevealControlContext,
  RevealLayerContext,
  type RevealControl,
  type RevealLayerState,
} from "./context.ts";
import { startLock, startUnlock, type RunDeps, type RunMode } from "./driver.ts";
import type { RevealEnv } from "./env.ts";
import { buildDisplay } from "./paint.ts";
import { markFromPointer, resolveOrigin, type PointerMark } from "./origin.ts";
import {
  initReveal,
  keyOf,
  revealReducer,
  type LockedSession,
  type Phase,
  type RevealEvent,
  type RevealState,
  type RevealTarget,
} from "./phase.ts";
import { useRevealEnv } from "./RevealProvider.tsx";
import { sceneFor } from "./scene.ts";
import { releaseSealed, type SealedDisplay } from "./seal.ts";
import { BAND_PIXEL_BUDGET, SEAL_TIMEOUT_MS } from "./timing.ts";

export type { RevealTarget } from "./phase.ts";

const ACTIVE: RevealLayerState = { active: true };
const INACTIVE: RevealLayerState = { active: false };
/** The idle callback gives up after this long, so the warm-up still happens on a busy machine. */
const WARM_UP_FALLBACK_MS = 300;

interface Refs {
  root: RefObject<HTMLDivElement | null>;
  unlock: RefObject<HTMLDivElement | null>;
  vault: RefObject<HTMLDivElement | null>;
  sealed: RefObject<HTMLDivElement | null>;
  band: RefObject<HTMLCanvasElement | null>;
}

type Dispatch = ActionDispatch<[event: RevealEvent]>;

const driverDeps = (env: RevealEnv): RunDeps => ({
  raf: (cb) => env.deps.raf(cb),
  caf: (id) => {
    env.deps.caf(id);
  },
  now: () => env.deps.now(),
  dark: () => env.dark(),
});

/** Seals the vault for the lock animation. Called from event handlers only. */
function useSealControl(args: {
  phase: Phase;
  env: RevealEnv;
  refs: Refs;
  markRef: RefObject<PointerMark | null>;
  dispatch: Dispatch;
}): RevealControl {
  const { phase, env, refs, markRef, dispatch } = args;
  const secrets = useSecrets();
  const latestPhase = useLatest(phase);
  const sealVault = useCallback((): boolean => {
    const vault = refs.vault.current;
    const root = refs.root.current;
    const now = latestPhase.current;
    if (!env.deps.support.animate || !(now === "idle-unlocked" || now === "unlocking") || !vault || !root) {
      return false;
    }
    secrets.clearAll();
    const origin = resolveOrigin({
      dir: "lock",
      mark: markRef.current,
      layer: vault,
      stageRect: root.getBoundingClientRect(),
      now: env.deps.now(),
    });
    markRef.current = null;
    const band = env.band(refs.band.current);
    const sealed = env.deps.prepareSeal({ vault, stage: root, origin, env, band });
    if (sealed === null) {
      return false;
    }
    dispatch({ type: "seal", sealed });
    return true;
  }, [env, refs, markRef, dispatch, secrets, latestPhase]);
  const abortSeal = useCallback(() => {
    dispatch({ type: "abortSeal" });
  }, [dispatch]);
  return useMemo(() => ({ sealVault, abortSeal }), [sealVault, abortSeal]);
}

function modeFor(env: RevealEnv, band: Band | null): RunMode {
  if (env.deps.reduced()) {
    return "fade";
  }
  return band !== null && !band.lost ? "glass" : "lens";
}

interface RunContext {
  env: RevealEnv;
  refs: Refs;
  markRef: RefObject<PointerMark | null>;
  dispatch: Dispatch;
  run: number;
}

function runUnlock({ env, refs, markRef, dispatch, run }: RunContext): (() => void) | undefined {
  const { root, vault, unlock, band: bandRef } = refs;
  if (!root.current || !vault.current || !unlock.current) {
    return undefined;
  }
  const [stage, vaultEl] = [root.current, vault.current];
  const stageRect = stage.getBoundingClientRect();
  const origin = resolveOrigin({
    dir: "unlock",
    mark: markRef.current,
    layer: unlock.current,
    stageRect,
    now: env.deps.now(),
  });
  markRef.current = null;
  // Focus moves when the reveal starts, so nothing changes when it ends.
  vaultEl.querySelector<HTMLElement>("main")?.focus({ preventScroll: true });
  const band = env.band(bandRef.current);
  const mode = modeFor(env, band);
  const scene = sceneFor(stageRect, origin, env.deps.dpr(), BAND_PIXEL_BUDGET);
  stage.setAttribute("data-mode", mode);
  const stop = startUnlock(
    {
      vault: vaultEl,
      scene,
      mode,
      band,
      bandEl: bandRef.current,
      build: (paint) => buildDisplay(vaultEl, stage, scene.S, { groups: true, paint }),
    },
    driverDeps(env),
    () => {
      dispatch({ type: "done", run });
    },
  );
  return () => {
    stop();
    stage.removeAttribute("data-mode");
  };
}

function runLock(sealed: SealedDisplay | null, { env, refs, dispatch, run }: RunContext): (() => void) | undefined {
  const { root, sealed: hostRef, band: bandRef } = refs;
  if (!root.current || !hostRef.current || sealed === null) {
    return undefined;
  }
  const stage = root.current;
  stage.setAttribute("data-mode", sealed.mode);
  const stop = startLock(
    {
      host: hostRef.current,
      sealed,
      band: sealed.mode === "glass" ? env.band(bandRef.current) : null,
      bandEl: bandRef.current,
    },
    driverDeps(env),
    () => {
      dispatch({ type: "done", run });
    },
  );
  return () => {
    stop();
    stage.removeAttribute("data-mode");
  };
}

/** Starts the unlock or lock run when its phase begins, and cancels it when the phase ends. */
function useRunEffect(s: RevealState, ctx: Omit<RunContext, "run">): void {
  const { phase, run, sealed } = s;
  const { env, refs, markRef, dispatch } = ctx;
  useLayoutEffect(() => {
    const context = { env, refs, markRef, dispatch, run };
    if (phase === "unlocking") {
      return runUnlock(context);
    }
    return phase === "locking" ? runLock(sealed, context) : undefined;
  }, [phase, run, sealed, env, refs, markRef, dispatch]);
}

/** Puts the sealed picture into its host, and frees its canvases when it goes. */
function useSealedPicture(s: RevealState, refs: Refs): void {
  const { sealed } = s;
  useLayoutEffect(() => {
    const host = refs.sealed.current;
    if (sealed === null || !host) {
      return undefined;
    }
    host.replaceChildren(sealed.cv);
    return () => {
      sealed.cv.remove();
    };
  }, [sealed, refs]);
  useEffect(
    () => () => {
      if (sealed !== null) {
        releaseSealed(sealed);
      }
    },
    [sealed],
  );
}

/** A sealed vault waits for the locked session; if it never comes, the vault is put back. */
function useSealTimeout(phase: Phase, dispatch: Dispatch): void {
  useEffect(() => {
    if (phase !== "sealing") {
      return undefined;
    }
    const id = setTimeout(() => {
      dispatch({ type: "abortSeal" });
    }, SEAL_TIMEOUT_MS);
    return () => {
      clearTimeout(id);
    };
  }, [phase, dispatch]);
}

/** ANGLE compiles lazily at the first draw: draw the band once while the locked screen idles (L950-955). */
function useWarmUp(phase: Phase, env: RevealEnv, refs: Refs): void {
  useEffect(() => {
    if (phase !== "idle-locked" || !env.deps.support.gl) {
      return undefined;
    }
    const warm = () => {
      env.band(refs.band.current)?.warmUp();
    };
    if ("requestIdleCallback" in window) {
      const id = window.requestIdleCallback(warm);
      return () => {
        window.cancelIdleCallback(id);
      };
    }
    const id = setTimeout(warm, WARM_UP_FALLBACK_MS);
    return () => {
      clearTimeout(id);
    };
  }, [phase, env, refs]);
}

function Layer({
  name,
  active,
  elRef,
  children,
}: {
  name: "unlock" | "vault" | "other";
  active: boolean;
  elRef?: RefObject<HTMLDivElement | null>;
  children: ReactNode;
}) {
  return (
    <div
      ref={elRef}
      className="reveal__layer"
      data-reveal-layer={name}
      inert={!active}
      aria-hidden={active ? undefined : true}
    >
      <RevealLayerContext value={active ? ACTIVE : INACTIVE}>{children}</RevealLayerContext>
    </div>
  );
}

interface LayersProps {
  s: RevealState;
  unlockRef: RefObject<HTMLDivElement | null>;
  vaultRef: RefObject<HTMLDivElement | null>;
  sealedRef: RefObject<HTMLDivElement | null>;
  bandRef: RefObject<HTMLCanvasElement | null>;
  gl: boolean;
  renderUnlock: (session: LockedSession) => ReactNode;
  renderVault: () => ReactNode;
  other: ReactNode;
}

function Layers({ s, unlockRef, vaultRef, sealedRef, bandRef, gl, renderUnlock, renderVault, other }: LayersProps) {
  const { phase, locked, sealed } = s;
  const unlockOn = locked !== null && (phase === "idle-locked" || phase === "unlocking" || phase === "locking");
  const vaultOn = phase === "idle-unlocked" || phase === "unlocking";
  const sealedOn = (phase === "sealing" || phase === "locking") && sealed !== null;
  return (
    <>
      {phase === "other" && (
        <Layer key="other" name="other" active>
          {other}
        </Layer>
      )}
      {unlockOn && (
        <Layer key="unlock" name="unlock" active={phase !== "unlocking"} elRef={unlockRef}>
          {renderUnlock(locked)}
        </Layer>
      )}
      {vaultOn && (
        <Layer key="vault" name="vault" active elRef={vaultRef}>
          {renderVault()}
        </Layer>
      )}
      {sealedOn && (
        <div
          key="sealed"
          ref={sealedRef}
          className="reveal__sealed"
          data-reveal-layer="sealed"
          aria-hidden="true"
        />
      )}
      {gl && <canvas key="band" ref={bandRef} className="reveal__band" aria-hidden="true" />}
    </>
  );
}

export function RevealStage({
  target,
  renderUnlock,
  renderVault,
  other,
}: {
  target: RevealTarget;
  renderUnlock: (session: LockedSession) => ReactNode;
  renderVault: () => ReactNode;
  other: ReactNode;
}) {
  const env = useRevealEnv();
  const [s, dispatch] = useReducer(revealReducer, target, initReveal);
  // Store the target in the machine when it changes (React's "adjust state while rendering").
  if (keyOf(target) !== s.key || (target.kind === "locked" && target.session !== s.locked)) {
    dispatch({ type: "target", target, animate: env.deps.support.animate });
  }
  const rootRef = useRef<HTMLDivElement>(null);
  const unlockRef = useRef<HTMLDivElement>(null);
  const vaultRef = useRef<HTMLDivElement>(null);
  const sealedRef = useRef<HTMLDivElement>(null);
  const bandRef = useRef<HTMLCanvasElement>(null);
  const markRef = useRef<PointerMark | null>(null);
  const refs = useMemo<Refs>(
    () => ({ root: rootRef, unlock: unlockRef, vault: vaultRef, sealed: sealedRef, band: bandRef }),
    [],
  );
  const control = useSealControl({ phase: s.phase, env, refs, markRef, dispatch });
  useRunEffect(s, { env, refs, markRef, dispatch });
  useSealedPicture(s, refs);
  useSealTimeout(s.phase, dispatch);
  useWarmUp(s.phase, env, refs);

  return (
    <RevealControlContext value={control}>
      <div
        ref={rootRef}
        className="reveal"
        data-phase={s.phase}
        onPointerDownCapture={(event) => {
          const next = markFromPointer(event, event.currentTarget, env.deps.now());
          if (next !== null) {
            markRef.current = next;
          }
        }}
        onKeyDownCapture={() => {
          markRef.current = null;
        }}
      >
        <Layers
          s={s}
          unlockRef={unlockRef}
          vaultRef={vaultRef}
          sealedRef={sealedRef}
          bandRef={bandRef}
          gl={env.deps.support.gl}
          renderUnlock={renderUnlock}
          renderVault={renderVault}
          other={other}
        />
      </div>
    </RevealControlContext>
  );
}
