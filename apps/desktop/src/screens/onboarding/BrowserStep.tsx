/**
 * Step 5 (D06): install the browser extension, optional. Pairing is confirmed on this side
 * (the pairing dialog opens over the step as usual, ADR-0036); the list of paired browsers
 * updates by itself. Store links appear once the listings exist (`STORE_URLS`).
 */
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { BrowserPairing } from "../../ipc/contract/index.ts";
import { BrowserPairingDialog } from "../BrowserPairingDialog.tsx";
import { useBrowserStatus } from "../useBrowserStatus.ts";
import { OnboardingShell } from "./OnboardingShell.tsx";

const STORES = ["chrome", "edge", "firefox"] as const;

/** Store listings of the extension. `null` until the listing exists (owner item, PROGRESS "Needs owner"). */
export const STORE_URLS: Readonly<Record<(typeof STORES)[number], string | null>> = {
  chrome: null,
  edge: null,
  firefox: null,
};

function StoreLinks() {
  const { t } = useI18n();
  const links = STORES.flatMap((store) => {
    const url = STORE_URLS[store];
    return url === null ? [] : [{ store, url }];
  });
  if (links.length === 0) {
    return <p className="field__hint">{t("onboarding.browser.notInStores")}</p>;
  }
  return (
    <ul className="store-links">
      {links.map(({ store, url }) => (
        <li key={store}>
          <a className="button button--secondary button--sm" href={url} target="_blank" rel="noreferrer">
            {t("onboarding.browser.getFor", { browser: t(`browser.hint.${store}`) })}
          </a>
        </li>
      ))}
    </ul>
  );
}

function PairedList({ pairings }: { pairings: readonly BrowserPairing[] }) {
  const { t } = useI18n();
  return (
    <section aria-label={t("onboarding.browser.connectedList")}>
      <div role="status">
        <ul className="rows">
        {pairings.length === 0 ? (
          <li className="row">
            <span className="row__main field__hint">{t("onboarding.browser.waiting")}</span>
          </li>
        ) : (
          pairings.map((pairing) => (
            <li key={pairing.pairingId} className="row">
              <span className="row__status">
                <Icon name="check" size="sm" />
                <UserText>{t("onboarding.browser.connected", { name: pairing.label })}</UserText>
              </span>
              <span className="caption">{t(`browser.family.${pairing.browser}`)}</span>
            </li>
          ))
        )}
        </ul>
      </div>
    </section>
  );
}

export function BrowserStep({ onDone }: { onDone: () => void }) {
  const { t } = useI18n();
  const [status] = useBrowserStatus();
  const paired = (status?.pairings.length ?? 0) > 0;
  return (
    <OnboardingShell step={5}>
      <div className="auth__main">
        <div className="auth__intro">
          <h1 className="display">{t("onboarding.browser.title")}</h1>
          <p className="lede">{t("onboarding.browser.body")}</p>
        </div>
        <StoreLinks />
        <ol className="steps" aria-label={t("onboarding.browser.steps")}>
          <li>{t("onboarding.browser.step1")}</li>
          <li>{t("onboarding.browser.step2")}</li>
          <li>{t("onboarding.browser.step3")}</li>
        </ol>
        <PairedList pairings={status?.pairings ?? []} />
        {paired && (
          <div className="done-note">
            <Icon name="check" />
            <div>
              <p className="done-note__title">{t("onboarding.done.title")}</p>
              <p className="caption">{t("onboarding.done.body")}</p>
            </div>
          </div>
        )}
        <div className="auth__actions">
          <HoloButton size="lg" block onClick={onDone}>
            {t(paired ? "onboarding.browser.done" : "onboarding.browser.skip")}
          </HoloButton>
        </div>
      </div>
      <BrowserPairingDialog />
    </OnboardingShell>
  );
}
