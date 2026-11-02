/**
 * A keyboard hint. Give a `shortcut` to take it from the shortcut table (so display and
 * matching never drift), or `children` for a fixed key such as Esc.
 */
import type { ReactNode } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { usePlatform } from "../platform/platform.ts";
import { format, SHORTCUTS, type ShortcutName } from "../platform/shortcuts.ts";

export function Kbd({
  shortcut,
  children,
  hidden = false,
  className,
}: {
  shortcut?: ShortcutName;
  children?: ReactNode;
  hidden?: boolean;
  className?: string;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  return (
    <kbd className={className === undefined ? "kbd" : `kbd ${className}`} aria-hidden={hidden ? "true" : undefined}>
      {shortcut === undefined ? children : format(SHORTCUTS[shortcut], platform, t)}
    </kbd>
  );
}
