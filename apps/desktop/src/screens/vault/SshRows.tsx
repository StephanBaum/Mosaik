/**
 * What the detail shows of an SSH key: the public key (copyable), the fingerprint, the
 * algorithm, whether a passphrase protects it, and whether the agent can use it. The private
 * key never comes back to the page: there is no reveal, copy or export for it.
 */
import { Callout } from "../../components/Callout.tsx";
import { Chip } from "../../components/Chip.tsx";
import { CopyButton } from "../../components/CopyButton.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { ItemRequest, SshKeySummary } from "../../ipc/contract/index.ts";

export function SshRows({ summary, item }: { summary: SshKeySummary; item: ItemRequest }) {
  const { t } = useI18n();
  const publicLabel = t("item.field.sshPublicKey");
  return (
    <>
      {summary.publicKey !== null && (
        <div className="row">
          <div className="row__main">
            <span className="row__label">{publicLabel}</span>
            <span className="row__value mono">{summary.publicKey}</span>
          </div>
          <CopyButton
            source={{ kind: "item", ...item, field: { kind: "sshPublicKey" } }}
            field={publicLabel}
          />
        </div>
      )}
      <div className="row">
        <div className="row__main">
          <span className="row__label">{t("item.field.sshFingerprint")}</span>
          <span className="row__value mono">{summary.fingerprint ?? ""}</span>
        </div>
        {summary.algorithm !== null && <Chip>{summary.algorithm}</Chip>}
        {summary.encrypted && <Chip>{t("item.ssh.encrypted")}</Chip>}
      </div>
      {!summary.agentSupported && (
        <Callout tone="info">
          {t("item.ssh.agentUnsupported", { algorithm: summary.algorithm ?? "SSH" })}
        </Callout>
      )}
    </>
  );
}
