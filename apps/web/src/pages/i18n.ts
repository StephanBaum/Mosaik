/** Build-time message formatting for the static pages. No ICU code reaches the client. */
import { formatMessage, type Locale, type MessageKey, type MessageValues } from "@mosaic/i18n";

export type WebKey = Extract<MessageKey, `web.${string}`>;

export type Translate = (key: WebKey, values?: MessageValues) => string;

/** A formatter for `locale` that fills the working product name and domain into every message. */
export function translator(locale: Locale): Translate {
  const base = {
    name: formatMessage(locale, "web.brand.name"),
    domain: formatMessage(locale, "web.site.domain"),
  };
  return (key, values) => formatMessage(locale, key, { ...base, ...values });
}

/** Public path of a locale's home page. */
export function homePath(locale: Locale): string {
  return locale === "en" ? "/" : `/${locale}/`;
}
