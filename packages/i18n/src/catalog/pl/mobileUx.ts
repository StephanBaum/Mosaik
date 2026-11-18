/**
 * Polish UI strings: mobileUx. Same keys as English (checked by the type).
 * ICU plural categories of Polish: one, few, many, other (CLDR; `other` is required).
 */
import type { mobileUx as en } from "../en/mobileUx.ts";

export const mobileUx = {
  "mobile.sync.warningsBanner": "Synchronizacja wykryła problem. Otwórz Ustawienia, aby go sprawdzić.",
  "mobile.provider.errorTitle": "Mosaic nie może kontynuować",
  "mobile.onboarding.details.keyWiped":
    "Mosaic usunął zeskanowany Klucz tajny po opuszczeniu aplikacji. Zeskanuj kod ponownie lub wpisz klucz.",
  "mobile.common.showField": "Pokaż: {field}",
  "mobile.common.hideField": "Ukryj: {field}",
  "mobile.error.storeInUse":
    "Mosaic jest zajęty w innym miejscu na tym telefonie. Chwilę poczekaj, a potem spróbuj ponownie.",
  "mobile.error.noAccount":
    "Mosaic nie jest jeszcze skonfigurowany na tym telefonie. Otwórz Mosaic, aby go skonfigurować.",
  "mobile.error.wrongCredentials":
    "Hasło główne lub Klucz tajny nie zadziałały. Sprawdź literówki i język klawiatury, a potem spróbuj ponownie.",
  "mobile.error.invalidInput":
    "Coś z wpisanych danych jest nieprawidłowe. Sprawdź wpisy i spróbuj ponownie.",
  "mobile.error.vaultUnavailable":
    "Tego sejfu nie można teraz otworzyć. Zsynchronizuj i spróbuj ponownie lub odblokuj Mosaic jeszcze raz.",
  "mobile.unlock.quickOptionTouchId": "Następnym razem odblokuj za pomocą Touch ID",
  "mobile.detail.readCode": "Odczytaj kod",
  "mobile.detail.labelValue": "{label}: {value}",
  "mobile.autofill.warningWithName": "Najpierw sprawdź adres | {name}",
  "mobile.autofill.linkEntry": "Powiąż {login} i wypełnij",
  "mobile.link.imported": "Zaimportowano dla tej aplikacji",
  "mobile.link.confirmTitle": "Powiązać {login} z {app}?",
  "mobile.link.confirmSubtitle": "ID aplikacji {appId} · Podpis {fingerprint}",
  "mobile.link.confirmFallbackHint": "Po powiązaniu dotknij pola ponownie, aby je wypełnić.",
  "mobile.link.importedNotVerified":
    "Zaimportowane logowanie. Witryna nie potwierdziła tej aplikacji.",
} satisfies Record<keyof typeof en, string>;
