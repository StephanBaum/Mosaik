/**
 * The review step: what a change touches and what happens to other devices, shared vaults,
 * emergency contacts, quick unlock and security keys. Plain lists, no colour-only meaning.
 */
import type { MessageKey } from "@mosaic/i18n";
import { Callout } from "../../../components/Callout.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import { UserText } from "../../../components/UserText.tsx";
import type { RekeyReviewView } from "../../../ipc/contract/index.ts";

export type CredentialKind = "password" | "secretKey" | "profile";

/** The facts of a password change, a Secret Key rotation or a protection upgrade. */
export function CredentialReview({ kind }: { kind: CredentialKind }) {
  const { t } = useI18n();
  return (
    <div className="stack">
      <h4 className="row__label">{t("rotation.review.changes")}</h4>
      <ul className="stack">
        <li className="body">{t(`rotation.review.${kind}.changes`)}</li>
        <li className="body">{t(`rotation.review.${kind}.devices`)}</li>
      </ul>
      <h4 className="row__label">{t("rotation.review.stays")}</h4>
      <ul className="stack">
        <li className="body">{t(`rotation.review.${kind}.stays`)}</li>
      </ul>
      {kind === "secretKey" && <Callout tone="warning">{t("rotation.review.secretKey.limit")}</Callout>}
    </div>
  );
}

function Points({ items }: { items: readonly (string | null)[] }) {
  return (
    <ul className="stack">
      {items.map((item) =>
        item === null ? null : (
          <li key={item} className="body">
            {item}
          </li>
        ),
      )}
    </ul>
  );
}

/** What a re-key will do to this account, from `rekey_review`. */
export function RekeyReviewPanel({ review }: { review: RekeyReviewView }) {
  const { t } = useI18n();
  const count = (key: MessageKey, n: number): string | null => (n > 0 ? t(key, { count: n }) : null);
  return (
    <div className="stack">
      <h4 className="row__label">{t("rotation.review.rekey.title")}</h4>
      <Points
        items={[
          t("rotation.review.rekey.keys"),
          count("rotation.review.rekey.vaults", review.ownedVaults),
          t("rotation.review.rekey.devices"),
          review.backend === "folder" ? t("rotation.review.rekey.folder") : null,
          review.quickUnlock ? t("rotation.review.rekey.quick") : null,
          count("rotation.review.rekey.hardwareKeys", review.hardwareKeys),
          count("rotation.review.rekey.hardwareVaults", review.hardwareVaults),
          count("rotation.review.rekey.attachments", review.unreachableAttachments),
        ]}
      />
      {review.sharedOwned.length > 0 && (
        <div className="stack">
          <p className="body">{t("rotation.review.rekey.shared", { count: review.sharedOwned.length })}</p>
          <ul className="stack">
            {review.sharedOwned.map((vault) => (
              <li key={vault.vaultId} className="caption">
                {vault.name === null ? t("sync.warning.unknownVault") : <UserText>{vault.name}</UserText>}
                {" · "}
                {vault.members}
              </li>
            ))}
          </ul>
          <Callout tone="warning">{t("rotation.review.rekey.sharedMembers")}</Callout>
        </div>
      )}
      <Points
        items={[
          count("rotation.review.rekey.memberVaults", review.memberVaults.length),
          count("rotation.review.rekey.grants", review.grants),
        ]}
      />
      <Callout tone="info">{t("rotation.review.rekey.limit")}</Callout>
    </div>
  );
}
