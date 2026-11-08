/** Build-time message formatting for the static pages. No ICU code reaches the client. */
import { formatMessage, type Locale, type MessageKey, type MessageValues } from "@mosaic/i18n";
import { pathOf, type RouteId } from "./routes.ts";

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

/** What every page component gets: the formatter, the locale and links to the other pages. */
export interface Ctx {
  t: Translate;
  locale: Locale;
  /** The path of a page in this locale, with an optional `#fragment`. */
  href: (id: RouteId, hash?: string) => string;
}

export function context(locale: Locale): Ctx {
  return {
    t: translator(locale),
    locale,
    href: (id, hash) => pathOf(id, locale) + (hash === undefined ? "" : `#${hash}`),
  };
}
