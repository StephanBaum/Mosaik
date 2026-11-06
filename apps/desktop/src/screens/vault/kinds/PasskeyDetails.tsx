/**
 * What the detail shows of a passkey: the site, the account, a short credential ID, the algorithm
 * and the creation date. The private key never comes back to the page: it is not a secret field,
 * so there is no reveal or copy for it (ADR-0039); the only way out is the CXF export in
 * Settings (ADR-0041), which asks for the master password. A passkey is edited only through its
 * title, notes, tags and favorite; its fields come from the site that made it.
 */
import { Callout } from "../../../components/Callout.tsx";
import { UserText } from "../../../components/UserText.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { PasskeySummary } from "../../../ipc/contract/index.ts";

/** How much of the credential ID the row shows; the full ID is in the tooltip. */
const SHORT_ID_CHARS = 16;

function shortId(id: string): string {
  return id.length > SHORT_ID_CHARS ? `${id.slice(0, SHORT_ID_CHARS)}…` : id;
}

export function PasskeyDetails({ summary }: { summary: PasskeySummary }) {
  const { t } = useI18n();
  return (
    <>
      <div className="row">
        <div className="row__main">
          <span className="row__label">{t("item.passkey.rpId")}</span>
          <span className="row__value">{summary.rpId}</span>
        </div>
      </div>
      {summary.userName !== "" && (
        <div className="row">
          <div className="row__main">
            <span className="row__label">{t("item.passkey.userName")}</span>
            <span className="row__value">
              <UserText>{summary.userName}</UserText>
            </span>
          </div>
        </div>
      )}
      {summary.userDisplayName !== "" && (
        <div className="row">
          <div className="row__main">
            <span className="row__label">{t("item.passkey.displayName")}</span>
            <span className="row__value">
              <UserText>{summary.userDisplayName}</UserText>
            </span>
          </div>
        </div>
      )}
      <div className="row">
        <div className="row__main">
          <span className="row__label">{t("item.passkey.credentialId")}</span>
          {/* The group role lets the full ID, not the shortened text, name the value. */}
          <span
            className="row__value mono"
            role="group"
            aria-label={summary.credentialId}
            title={summary.credentialId}
          >
            {shortId(summary.credentialId)}
          </span>
        </div>
      </div>
      <div className="row">
        <div className="row__main">
          <span className="row__label">{t("item.passkey.algorithm")}</span>
          <span className="row__value">
            {summary.algorithm === "es256" ? "ES256" : t("item.passkey.algorithmOther")}
          </span>
        </div>
      </div>
      <div className="row">
        <div className="row__main">
          <span className="row__label">{t("item.passkey.created")}</span>
          <span className="row__value">
            {t("item.passkey.createdValue", { at: summary.createdAt })}
          </span>
        </div>
      </div>
      <Callout tone="info">{t("item.passkey.note")}</Callout>
    </>
  );
}
