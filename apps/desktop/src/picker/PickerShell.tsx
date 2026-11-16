/**
 * The page of a picker window: the state from Rust, the language and the theme it names, and
 * whatever the window shows for it. Nothing is rendered until the first state arrives.
 */
import { useEffect, type ReactNode } from "react";
import { I18nProvider, resolveLocale } from "../i18n/I18nProvider.tsx";
import type { PickerState } from "../ipc/contract/index.ts";
import { applyTheme } from "../theme/theme.ts";
import type { PickerIpc } from "./ipc.ts";
import { usePickerState } from "./usePickerState.ts";

export function PickerShell({
  ipc,
  children,
}: {
  ipc: PickerIpc;
  children: (state: PickerState) => ReactNode;
}) {
  const state = usePickerState(ipc);
  const theme = state?.theme;
  useEffect(() => {
    if (theme !== undefined) {
      applyTheme(theme);
    }
  }, [theme]);
  if (state === null) {
    return null;
  }
  return (
    <I18nProvider locale={resolveLocale(state.language, navigator.languages)}>
      {children(state)}
    </I18nProvider>
  );
}
