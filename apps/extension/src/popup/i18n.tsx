/** UI strings: every text comes from `@mosaic/i18n` (UX_GUIDELINES §6). */
import {
  formatMessage,
  type Locale,
  type MessageKey,
  type MessageValues,
} from "@mosaic/i18n";
import { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";
import type { ErrorCode } from "../shared/errors.ts";

export interface I18n {
  locale: Locale;
  t: (key: MessageKey, values?: MessageValues) => string;
  /** What happened and what to do, for any error code the extension can show. */
  errorText: (code: ErrorCode) => string;
}

const I18nContext = createContext<I18n | null>(null);

export function I18nProvider({ locale, children }: { locale: Locale; children: ReactNode }) {
  const value = useMemo<I18n>(
    () => ({
      locale,
      t: (key, values) => formatMessage(locale, key, values),
      errorText: (code) => formatMessage(locale, `ext.error.${code}`),
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
