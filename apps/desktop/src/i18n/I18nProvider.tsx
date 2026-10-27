/** UI strings: every text comes from `@mosaic/i18n` (UX_GUIDELINES §6). */
import {
  formatMessage,
  negotiateLocale,
  type Locale,
  type MessageKey,
  type MessageValues,
} from "@mosaic/i18n";
import { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";
import type { IpcError, Language } from "../ipc/contract/index.ts";

export interface I18n {
  locale: Locale;
  t: (key: MessageKey, values?: MessageValues) => string;
  /** The message for a failed command: what happened and what to do next. */
  errorMessage: (error: IpcError) => string;
}

const I18nContext = createContext<I18n | null>(null);

/** The locale for a language setting; `system` follows the OS (English fallback). */
export function resolveLocale(language: Language, preferred: readonly string[]): Locale {
  return language === "system" ? negotiateLocale(preferred) : language;
}

export function I18nProvider({ locale, children }: { locale: Locale; children: ReactNode }) {
  const value = useMemo<I18n>(
    () => ({
      locale,
      t: (key, values) => formatMessage(locale, key, values),
      errorMessage: (error) => formatMessage(locale, `error.${error.code}`),
    }),
    [locale],
  );
  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);
  return <I18nContext value={value}>{children}</I18nContext>;
}

export function useI18n(): I18n {
  const context = useContext(I18nContext);
  if (context === null) {
    throw new Error("useI18n needs an I18nProvider");
  }
  return context;
}
