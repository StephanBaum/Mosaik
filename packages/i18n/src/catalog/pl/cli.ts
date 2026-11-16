/**
 * Polish CLI messages. Placeholders are plain `{name}` only (ADR-0075).
 * `pnpm --filter @mosaic/i18n export:cli` writes them to `crates/cli/i18n/pl.json`.
 */
import type { cli as en } from "../en/cli.ts";

export const cli = {
  "cli.err.storeInUse":
    "Aplikacja Mosaic ma otwarty ten sejf. Zamknij ją albo nadaj CLI własne urządzenie poleceniem `mosaic setup own-store`.",
  "cli.err.noStore":
    "Nie znaleziono sejfu w {path}. Najpierw skonfiguruj Mosaic w aplikacji albo podaj --store.",
  "cli.err.noAccount": "Ten magazyn nie ma jeszcze konta.",
  "cli.err.wrongCredentials": "Błędne hasło główne lub Klucz tajny.",
  "cli.err.secretKeyInvalid":
    "Ten Klucz tajny jest nieprawidłowy. Porównaj go z Zestawem awaryjnym.",
  "cli.err.inputRequired": "Potrzebne są dane wejściowe, ale ustawiono --no-input.",
  "cli.err.promptTimeout": "Brak danych wejściowych przez {seconds} s.",
  "cli.err.notFound": "Nic nie znaleziono dla {ref}.",
  "cli.err.ambiguousRef":
    "Więcej niż jeden element pasuje do {ref}. Użyj dokładniejszego odwołania lub identyfikatora elementu.",
  "cli.err.invalidRef": "{ref} nie jest prawidłowym odwołaniem.",
  "cli.err.noSuchField": "Ten element nie ma pola {field}.",
  "cli.err.noOtp": "Ten element nie ma hasła jednorazowego.",
  "cli.err.revealRequired":
    "{field} jest tajne. Dodaj --reveal, aby je wypisać, lub --copy, aby umieścić je w schowku.",
  "cli.err.vaultUnavailable": "Ten sejf jest chroniony sprzętowo lub niedostępny tutaj.",
  "cli.err.rotationInProgress": "Najpierw dokończ zmianę klucza w aplikacji Mosaic.",
  "cli.err.secretInArgv":
    "{flag} umieściłoby sekret w wierszu poleceń. Użyj zamiast tego monitu, --password-stdin lub --password-fd.",
  "cli.err.unknownCommand": "Nieznane polecenie {name}. Uruchom `mosaic help`.",
  "cli.err.unknownOption": "Nieznana opcja {name}.",
  "cli.err.missingValue": "Opcja {name} wymaga wartości.",
  "cli.err.missingArgument": "Brak argumentu {name}.",
  "cli.err.unexpectedArgument": "Nieoczekiwany argument {value}.",
  "cli.err.invalidValue": "Nieprawidłowa wartość dla {name}.",
  "cli.err.nonUtf8": "Argumenty muszą być prawidłowym UTF-8.",
  "cli.err.fdUnsupported": "--password-fd nie jest dostępne w tym systemie.",
  "cli.err.sessionExpired": "Sesja odblokowania się zakończyła. Wpisz hasło główne.",
  "cli.err.sessionNotCached":
    "Nie udało się zapisać sesji odblokowania (magazyn kluczy systemu lub folder środowiska uruchomieniowego jest niedostępny). Każde polecenie będzie pytać o hasło główne.",
  "cli.err.twoFactorMethodUnsupported":
    "To konto loguje się kluczem bezpieczeństwa, którego wiersz poleceń nie obsługuje. Dodaj kod z aplikacji uwierzytelniającej w aplikacji na komputerze, a potem spróbuj ponownie.",
  "cli.err.envSecretIgnored":
    "{name} jest ustawione, ale zignorowane. Mosaic nigdy nie czyta sekretów ze zmiennych środowiskowych.",
  "cli.err.exportUnreadable": "Nie można odczytać pliku: błędne hasło lub uszkodzony plik.",
  "cli.err.importFailed": "Nie udało się zaimportować pliku.",
  "cli.err.importTooLarge": "Plik jest zbyt duży, aby go zaimportować.",
  "cli.err.syncNotConfigured": "Synchronizacja nie jest skonfigurowana dla tego magazynu.",
  "cli.err.syncHalted": "Synchronizacja jest zatrzymana. Uruchom `mosaic sync --status`.",
  "cli.err.syncFailed": "Synchronizacja nie powiodła się: {reason}.",
  "cli.err.syncSessionRevoked":
    "Klucze tego konta zostały odnowione na innym urządzeniu, więc ta sesja się zakończyła. Zaloguj się ponownie z nowym Kluczem tajnym z najnowszego Zestawu awaryjnego.",
  "cli.err.syncSessionPasswordChanged":
    "Hasło główne zostało zmienione na innym urządzeniu, więc ta sesja się zakończyła. Zaloguj się ponownie z nowym hasłem.",
  "cli.err.syncSessionDeviceRemoved":
    "To urządzenie zostało usunięte z konta na innym urządzeniu, więc ta sesja się zakończyła. Zaloguj się ponownie, aby dodać je z powrotem.",
  "cli.err.syncSessionOperator":
    "Operator serwera zakończył tę sesję. Zaloguj się ponownie, aby dalej synchronizować.",
  "cli.err.fileExists": "{path} już istnieje. Mosaic nie nadpisuje plików.",
  "cli.err.fileRead": "Nie można odczytać {path}.",
  "cli.err.fileWrite": "Nie można zapisać {path}.",
  "cli.err.passphraseMismatch": "Oba hasła się różnią.",
  "cli.err.passphraseWeak": "Użyj hasła o długości co najmniej {min} znaków.",
  "cli.err.badEnvName": "{name} nie jest prawidłową nazwą zmiennej środowiskowej.",
  "cli.err.spawnFailed": "Nie można uruchomić {command}.",
  "cli.err.noVaultChosen": "Istnieje kilka sejfów. Wybierz jeden opcją --vault.",
  "cli.err.notSupported": "{what} nie jest dostępne w tej wersji CLI.",
  "cli.err.storeExists": "{path} już istnieje.",
  "cli.err.internal": "Coś poszło nie tak.",
  "cli.prompt.password": "Hasło główne: ",
  "cli.prompt.secretKey": "Klucz tajny (z Zestawu awaryjnego): ",
  "cli.prompt.filePassword": "Hasło pliku: ",
  "cli.prompt.exportPassphrase": "Hasło pliku eksportu: ",
  "cli.prompt.exportPassphraseAgain": "Hasło ponownie: ",
  "cli.prompt.importPassphrase": "Hasło pliku eksportu: ",
  "cli.prompt.confirmImport": "Zaimportować elementy ({count}) do {vault}? [y/N] ",
  "cli.prompt.newPassword": "Nowe hasło główne (to ustawione gdzie indziej): ",
  "cli.prompt.twoFactor": "Kod weryfikacji dwuetapowej: ",
  "cli.out.recovered": "Odzyskano. To urządzenie używa teraz Twojego nowego hasła głównego.",
  "cli.prompt.acknowledge": "Wpisz słowo {word}, aby kontynuować: ",
  "cli.warn.revealToTerminal":
    "Tajne wartości są wypisywane w terminalu i pozostają w jego historii przewijania.",
  "cli.out.unlocked":
    "Odblokowano na {minutes} min. Gdy skończysz, uruchom `mosaic lock`.",
  "cli.out.locked": "Zablokowano. Sesję usunięto.",
  "cli.out.concealed": "••••••••",
  "cli.out.copied": "Skopiowano. Zostanie wyczyszczone za {seconds} s.",
  "cli.out.noItems": "Brak elementów.",
  "cli.out.ambiguousCandidate": "{vault} / {title} ({id})",
  "cli.out.attachment": "{name} ({size} B, {mime})",
  "cli.label.id": "ID",
  "cli.label.vault": "Sejf",
  "cli.label.type": "Typ",
  "cli.label.title": "Tytuł",
  "cli.label.otp": "Kod jednorazowy",
  "cli.label.attachments": "Załączniki",
  "cli.status.store": "Magazyn",
  "cli.status.profile": "Profil",
  "cli.status.account": "Konto",
  "cli.status.session": "Sesja",
  "cli.status.sessionNone": "zablokowano (brak sesji)",
  "cli.status.sessionUntil": "odblokowano do {time} (Unix ms)",
  "cli.status.sync": "Synchronizacja",
  "cli.status.syncOff": "nie skonfigurowano",
  "cli.status.syncOn": "{backend}, zatrzymana: {halted}, ustalenia: {findings}",
  "cli.import.preview": "Elementów do zaimportowania do {vault}: {count}. {summary}",
  "cli.import.duplicates": "Elementy wyglądające na duplikaty ({count}) zostaną pominięte.",
  "cli.import.warnings":
    "Wpisy, których nie udało się odczytać ({count}), zostały pominięte.",
  "cli.import.done":
    "Zaimportowano elementy: {imported} (pominięto: {skipped}, błędy: {failed}).",
  "cli.import.dryRun": "Próba: nic nie zaimportowano.",
  "cli.import.cancelled": "Anulowano. Nic nie zaimportowano.",
  "cli.export.done": "Wyeksportowano elementy: {items}, sejfy: {vaults}, do {path}.",
  "cli.export.attachmentWarn": "Elementy z załącznikami: {count}. Załączniki są dołączone.",
  "cli.export.attachmentsLeftOut":
    "Pliki załączników spoza eksportu: {count}. Użyj --include-attachments.",
  "cli.export.unavailableVaults": "Pominięto sejfy chronione sprzętowo: {count}.",
  "cli.sync.done": "Synchronizacja zakończona: odebrano {pulled}, wysłano {pushed}.",
  "cli.sync.finding": "Ustalenie: {kind}.",
  "cli.sync.clean": "Synchronizacja działa prawidłowo. Brak ustaleń.",
  "cli.sync.acknowledged": "Ustalenia wyczyszczono.",
  "cli.sync.acknowledgeWord": "acknowledge",
  "cli.setup.done": "To urządzenie jest skonfigurowane jako własny magazyn. Uruchom `mosaic sync`.",
  "cli.ssh.agentStatus": "Agent SSH działa w aplikacji Mosaic. CLI go nie uruchamia.",
  "cli.ssh.noKeys": "Brak kluczy SSH.",
  "cli.err.agentContext":
    "Wygląda to na sesję agenta AI. Wypisanie tutaj sekretu umieściłoby go w kontekście agenta. Użyj zamiast tego `mosaic agent mcp` albo dodaj --i-know, jeśli uruchamiasz to samodzielnie.",
  "cli.err.agentAlreadyPaired":
    "Połączenie o nazwie {name} już istnieje. Dodaj --replace, aby połączyć ponownie.",
  "cli.err.desktopNotRunning": "Aplikacja Mosaic nie jest uruchomiona.",
  "cli.err.agentAccessOff":
    "Dostęp agentów jest wyłączony. Włącz go w aplikacji Mosaic w Ustawieniach.",
  "cli.err.agentLocked": "Sejf jest zablokowany. Najpierw odblokuj aplikację Mosaic.",
  "cli.err.pairingDenied": "Połączenie odrzucono w aplikacji Mosaic.",
  "cli.err.pairingExpired": "Żądanie połączenia wygasło.",
  "cli.err.pairingLimit":
    "Aplikacja Mosaic ma maksymalną dozwoloną liczbę połączeń. Najpierw usuń jedno.",
  "cli.err.pairingNameTaken":
    "Połączenie o tej nazwie już istnieje: najpierw je usuń lub wybierz inną nazwę.",
  "cli.err.pairingBlocked": "Łączenie jest zablokowane na kilka minut po wielokrotnych odmowach.",
  "cli.err.notPaired":
    "Ten klient nie jest połączony. Uruchom `mosaic agent pair --name {name}`.",
  "cli.err.agentRefused": "Aplikacja Mosaic odrzuciła żądanie ({reason}).",
  "cli.err.agentIo": "Połączenie z aplikacją Mosaic nie powiodło się.",
  "cli.agent.pairCode": "Porównaj ten kod z kodem pokazanym przez Mosaic: {code}",
  "cli.agent.paired": "Połączono jako {name}.",
  "cli.agent.status":
    "Aplikacja: {desktop}. Sejf: {vault}. Dostęp agentów: {access}. Połączeni: {paired}.",
} satisfies Record<keyof typeof en, string>;
