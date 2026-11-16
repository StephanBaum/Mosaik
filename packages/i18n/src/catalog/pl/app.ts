/**
 * Polish UI strings: app. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { app as en } from "../en/app.ts";

export const app = {
  "app.name": "Mosaic",
  "app.preAudit":
    "Oprogramowanie przed audytem: nie zostało jeszcze sprawdzone przez zewnętrznego audytora bezpieczeństwa.",
  "app.previewTag": "Podgląd",
  "app.previewBuild":
    "Wersja podglądowa, nie wydanie. Niepodpisana, przed audytem, tylko do wypróbowania.",
  "action.cancel": "Anuluj",
  "action.continue": "Dalej",
  "action.retry": "Spróbuj ponownie",
  "action.copy": "Kopiuj",
  "action.reveal": "Pokaż",
  "action.hide": "Ukryj",
  "action.lock": "Zablokuj",
  "action.details": "Szczegóły",
  "sync.offline":
    "Nie można połączyć się z Twoim serwerem synchronizacji. Zmiany są zapisane na tym urządzeniu i zostaną zsynchronizowane po odzyskaniu połączenia.",
  "app.loading": "Otwieranie Mosaic…",
  "app.skipToContent": "Przejdź do treści",
  "action.save": "Zapisz",
  "action.close": "Zamknij",
  "action.showPassword": "Pokaż hasło",
  "action.hidePassword": "Ukryj hasło",
  "action.back": "Wstecz",
  "action.confirm": "Potwierdź",
  "action.edit": "Edytuj",
  "action.delete": "Przenieś do kosza",
  "action.restore": "Przywróć",
  "action.purge": "Usuń na zawsze",
  "action.generate": "Generuj",
  "action.useThis": "Użyj tego",
  "action.done": "Gotowe",
  "nav.main": "Nawigacja główna",
  "nav.vault": "Sejf",
  "nav.security": "Bezpieczeństwo",
  "nav.generator": "Generator",
  "nav.settings": "Ustawienia",
  "nav.trash": "Kosz",
  "nav.breadcrumb": "Ścieżka nawigacji",
  "nav.securityCount":
    "{count, plural, one {# wynik do sprawdzenia} few {# wyniki do sprawdzenia} many {# wyników do sprawdzenia} other {# wyniku do sprawdzenia}}",
  "sync.status.local": "Zapisano na tym urządzeniu",
  "sync.status.upToDate": "Zsynchronizowano",
  "sync.status.attention": "Wymaga uwagi",
  "sync.status.syncing": "Synchronizowanie…",
  "sync.status.offline": "Offline",
  "sync.status.error": "Synchronizacja nie powiodła się",
  "action.moreActions": "Więcej działań",
  "action.regenerate": "Generuj ponownie",
} satisfies Record<keyof typeof en, string>;
