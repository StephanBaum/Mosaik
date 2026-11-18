/**
 * Polish UI strings: insights. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { insights as en } from "../en/insights.ts";

export const insights = {
  "insights.title": "Bezpieczeństwo",
  "insights.intro":
    "Sprawdzanie odbywa się na tym urządzeniu. Ostatnio sprawdzono {checkedAt, date, medium} o {checkedAt, time, short}: {count, plural, one {# element} few {# elementy} many {# elementów} other {# elementu}}.",
  "insights.checkAgain": "Sprawdź ponownie",
  "insights.summary": "Podsumowanie",
  "insights.fixFirst": "Napraw je najpierw",
  "insights.showAll":
    "Pokaż wszystkie: {count, plural, one {# wynik} few {# wyniki} many {# wyników} other {# wyniku}}",
  "insights.empty":
    "Nie znaleziono problemów. Mosaic sprawdza ponownie za każdym razem, gdy otwierasz tę stronę.",
  "insights.kind.weak": "Słabe",
  "insights.kind.weak.description": "Łatwe do odgadnięcia",
  "insights.kind.reused": "Powtórzone",
  "insights.kind.reused.description":
    "{groups, plural, one {# grupa} few {# grupy} many {# grup} other {# grupy}}",
  "insights.kind.old": "Stare (ponad 1 rok)",
  "insights.kind.old.description": "Niezmieniane od ponad roku",
  "insights.kind.missingTotp": "Brak 2FA",
  "insights.kind.missingTotp.description": "Witryna to obsługuje, ale nie zostało to skonfigurowane",
  "insights.kind.insecureUrl": "Niezabezpieczony HTTP",
  "insights.kind.insecureUrl.description": "Wysyła hasło bez szyfrowania",
  "insights.kind.breached": "W wycieku",
  "insights.kind.breached.description": "Znalezione przez opcjonalne sprawdzanie wycieków",
  "insights.finding.breached":
    "To hasło występuje w znanym wycieku danych. Zmień je teraz, wszędzie, gdzie jest używane.",
  "insights.finding.weak": "Łatwe do odgadnięcia. Zmień je na wygenerowane hasło.",
  "insights.finding.reused":
    "Używane także przez {count, plural, one {# inny element} few {# inne elementy} many {# innych elementów} other {# innego elementu}}. Każda witryna powinna mieć własne hasło.",
  "insights.finding.old":
    "Niezmieniane od ponad roku. Zmień je, jeśli nadal używasz tego konta.",
  "insights.finding.missingTotp":
    "{site} oferuje kody dwuskładnikowe. Włącz je tam i zapisz kod w Mosaic.",
  "insights.finding.insecureUrl":
    "Używa zwykłego HTTP: {url}. Przełącz na https://, jeśli witryna to obsługuje.",
  "insights.severity.high": "Wysoka",
  "insights.severity.medium": "Średnia",
  "insights.severity.low": "Niska",
  "insights.action.changePassword": "Zmień hasło",
  "insights.action.setUpTotp": "Skonfiguruj 2FA",
  "insights.action.reviewUrl": "Sprawdź",
  "insights.attribution.twoFactor": "Dane 2FA: 2FA Directory (2fa.directory), licencja MIT.",
  "insights.counts":
    "{count, plural, =0 {Brak problemów} one {# problem} few {# problemy} many {# problemów} other {# problemu}}",
  "insights.documentation": "Jak to włączyć: {url}",
  "insights.actionFor": "{action}: {title}",
} satisfies Record<keyof typeof en, string>;
