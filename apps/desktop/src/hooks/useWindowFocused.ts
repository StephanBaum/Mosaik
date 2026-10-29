import { useSyncExternalStore } from "react";

function subscribe(cb: () => void): () => void {
  window.addEventListener("focus", cb);
  window.addEventListener("blur", cb);
  document.addEventListener("visibilitychange", cb);
  return () => {
    window.removeEventListener("focus", cb);
    window.removeEventListener("blur", cb);
    document.removeEventListener("visibilitychange", cb);
  };
}

const getSnapshot = (): boolean => document.hasFocus() && !document.hidden;

export function useWindowFocused(): boolean {
  return useSyncExternalStore(subscribe, getSnapshot);
}
