/** A keyboard hint taken from the shortcut table, so display and matching never drift. */
import { useI18n } from "../i18n/I18nProvider.tsx";
import { usePlatform } from "../platform/platform.ts";
import { format, SHORTCUTS, type ShortcutName } from "../platform/shortcuts.ts";

export function Kbd({ shortcut, hidden = false }: { shortcut: ShortcutName; hidden?: boolean }) {
  const { t } = useI18n();
  const platform = usePlatform();
  return (
    <kbd className="kbd" aria-hidden={hidden ? "true" : undefined}>
      {format(SHORTCUTS[shortcut], platform, t)}
    </kbd>
  );
}
