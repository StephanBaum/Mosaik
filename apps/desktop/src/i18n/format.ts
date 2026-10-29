/** Locale-aware date, time, relative-time and size formatting through `Intl`. */
import type { Locale } from "@mosaic/i18n";

export function formatDate(locale: Locale, ms: number): string {
  return new Intl.DateTimeFormat(locale, { dateStyle: "medium" }).format(ms);
}

export function formatTime(locale: Locale, ms: number): string {
  return new Intl.DateTimeFormat(locale, { timeStyle: "short" }).format(ms);
}

type RelativeStep = readonly [limitSeconds: number, unit: Intl.RelativeTimeFormatUnit, size: number];

const DAY_S = 86_400;
const RELATIVE_STEPS: readonly RelativeStep[] = [
  [60, "second", 1],
  [3600, "minute", 60],
  [DAY_S, "hour", 3600],
  [30 * DAY_S, "day", DAY_S],
  [365 * DAY_S, "month", 30 * DAY_S],
  [Infinity, "year", 365 * DAY_S],
];

export function formatRelative(locale: Locale, ms: number, now: number): string {
  const diffSeconds = (ms - now) / 1000;
  const abs = Math.abs(diffSeconds);
  const step = RELATIVE_STEPS.find(([limit]) => abs < limit) ?? ([Infinity, "year", 365 * DAY_S] as const);
  const [, unit, size] = step;
  const value = Math.trunc(diffSeconds / size);
  return new Intl.RelativeTimeFormat(locale, { numeric: "auto" }).format(value === 0 ? 0 : value, unit);
}

export function formatBytes(locale: Locale, bytes: number): string {
  const [unit, divisor] =
    bytes < 1000
      ? (["byte", 1] as const)
      : bytes < 1e6
        ? (["kilobyte", 1e3] as const)
        : (["megabyte", 1e6] as const);
  return new Intl.NumberFormat(locale, { style: "unit", unit, maximumFractionDigits: 1 }).format(
    bytes / divisor,
  );
}
