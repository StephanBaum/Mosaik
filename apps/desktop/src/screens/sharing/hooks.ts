/** Hooks of the sharing screens: which backend sync uses, and how times and roles are shown. */
import { useCallback } from "react";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { formatDate, formatTime } from "../../i18n/format.ts";
import type { ShareRole } from "../../ipc/contract/index.ts";
import { useSyncOverview } from "../sync/hooks.ts";

export type BackendKind = "loading" | "hosted" | "folder" | "none";

/**
 * What sync is set up with: sharing and emergency access work only with the Mosaic server.
 * `totp` says whether the sync account has a second factor (a fresh login then needs a code).
 */
export function useBackendKind(): { kind: BackendKind; totp: boolean } {
  const { overview } = useSyncOverview();
  if (overview === null) {
    return { kind: "loading", totp: false };
  }
  const { config } = overview;
  if (config === null) {
    return { kind: "none", totp: false };
  }
  return config.kind === "hosted"
    ? { kind: "hosted", totp: config.totpEnabled }
    : { kind: "folder", totp: false };
}

/** A moment as "Oct 26, 2026, 2:15 PM". */
export function useFormatWhen(): (ms: number) => string {
  const { locale } = useI18n();
  return useCallback((ms) => `${formatDate(locale, ms)}, ${formatTime(locale, ms)}`, [locale]);
}

/** The name of a role. */
export function useRoleLabel(): (role: ShareRole) => string {
  const { t } = useI18n();
  return useCallback((role) => t(`sharing.role.${role}`), [t]);
}
