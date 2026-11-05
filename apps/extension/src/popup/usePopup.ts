/** The popup's state: what the background reports for the front tab, kept fresh by its events. */
import { useCallback, useContext, useEffect, useState, createContext } from "react";
import { errorCodeOf, type ErrorCode } from "../shared/errors.ts";
import type { PopupState } from "../shared/messages.ts";
import type { PopupApi } from "./client.ts";
import type { PopupEnv } from "./env.ts";

export interface PopupModel {
  state: PopupState | null;
  /** Why the state could not be read at all (the background did not answer). */
  loadError: ErrorCode | null;
  refresh: () => Promise<void>;
}

type Setters = { setState: (state: PopupState) => void; setLoadError: (error: ErrorCode | null) => void };

/** Reads the state once; plain function so the effect below only subscribes. */
function load(api: PopupApi, set: Setters, isCurrent: () => boolean): Promise<void> {
  return api.request({ op: "getState" }).then(
    (state) => {
      if (isCurrent()) {
        set.setState(state);
        set.setLoadError(null);
      }
    },
    (error: unknown) => {
      if (isCurrent()) {
        set.setLoadError(errorCodeOf(error));
      }
    },
  );
}

export function usePopupState(api: PopupApi): PopupModel {
  const [state, setState] = useState<PopupState | null>(null);
  const [loadError, setLoadError] = useState<ErrorCode | null>(null);

  const refresh = useCallback(() => load(api, { setState, setLoadError }, () => true), [api]);

  useEffect(() => {
    let current = true;
    const set = { setState, setLoadError };
    void load(api, set, () => current);
    const stop = api.subscribe((event) => {
      if (event.event === "pairCode") {
        setState((was) =>
          was === null ? was : { ...was, pairCode: { code: event.code, expiresInSeconds: event.expiresInSeconds } },
        );
      } else {
        void load(api, set, () => current);
      }
    });
    return () => {
      current = false;
      stop();
    };
  }, [api]);

  return { state, loadError, refresh };
}

/** What every screen needs besides its props. */
export interface PopupServices {
  api: PopupApi;
  env: PopupEnv;
  refresh: () => Promise<void>;
}

const ServicesContext = createContext<PopupServices | null>(null);
export const ServicesProvider = ServicesContext.Provider;

export function useServices(): PopupServices {
  const services = useContext(ServicesContext);
  if (services === null) {
    throw new Error("useServices needs the popup's ServicesProvider");
  }
  return services;
}
