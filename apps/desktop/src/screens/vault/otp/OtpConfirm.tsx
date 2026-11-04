/**
 * The confirmation before anything is saved: service and account (editable), the settings,
 * and the live code the site usually asks for to finish setup (shown at once; hides on blur
 * and on lock like every code). Nothing is stored here: the editor saves the item.
 */
import { useState } from "react";
import { Button } from "../../../components/Button.tsx";
import { Callout } from "../../../components/Callout.tsx";
import { HoloButton } from "../../../components/HoloButton.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { ItemKind, OtpPending, VaultSummary } from "../../../ipc/contract/index.ts";
import { usePlatform } from "../../../platform/platform.ts";
import { TotpCode } from "../TotpCode.tsx";
import type { OtpSetupResult } from "./OtpSection.tsx";

type Target = OtpSetupResult["target"];

function hardwareVaults(vaults: readonly VaultSummary[]): VaultSummary[] {
  return vaults.filter((vault) => vault.protection === "hardwareKey");
}

function StoragePicker({
  vaults,
  target,
  onChange,
}: {
  vaults: readonly VaultSummary[];
  target: Target;
  onChange: (target: Target) => void;
}) {
  const { t } = useI18n();
  const nameOf = (vault: VaultSummary) => vault.name ?? t("vault.hardware.placeholderName");
  return (
    <fieldset className="otp-setup__storage">
      <legend className="field__label">{t("otp.storage.label")}</legend>
      <label className="otp-setup__choice">
        <input
          type="radio"
          name="otp-storage"
          checked={target.kind === "item"}
          onChange={() => {
            onChange({ kind: "item" });
          }}
        />
        <span>{t("otp.storage.item")}</span>
        <span className="field__hint">{t("otp.storage.itemTradeoff")}</span>
      </label>
      {hardwareVaults(vaults).map((vault) => (
        <label key={vault.vaultId} className="otp-setup__choice">
          <input
            type="radio"
            name="otp-storage"
            disabled={!vault.available}
            checked={target.kind === "hardwareVault" && target.vaultId === vault.vaultId}
            onChange={() => {
              onChange({ kind: "hardwareVault", vaultId: vault.vaultId });
            }}
          />
          <span>{t("otp.storage.hardware", { vault: nameOf(vault) })}</span>
          <span className="field__hint">
            {vault.available
              ? t("vault.hardware.tradeoff")
              : t("otp.storage.hardwareClosed", { vault: nameOf(vault) })}
          </span>
        </label>
      ))}
    </fieldset>
  );
}

function Summary({ pending }: { pending: OtpPending }) {
  const { t } = useI18n();
  return (
    <p className="otp-setup__details">
      <span>
        {t("otp.confirm.details", {
          scheme: pending.scheme,
          digits: pending.digits,
          period: pending.period ?? 0,
        })}
      </span>{" "}
      <span>{t("otp.confirm.algorithmValue", { algorithm: pending.algorithm })}</span>
    </p>
  );
}

export function OtpConfirm({
  pending,
  clipboardCleared,
  prefill,
  vaults,
  itemKind,
  onBack,
  onUse,
}: {
  pending: OtpPending;
  clipboardCleared: boolean;
  prefill: { issuer: string; account: string };
  vaults: readonly VaultSummary[];
  /** Only a login can keep its code in a hardware-protected vault. */
  itemKind: ItemKind;
  onBack: () => void;
  onUse: (result: OtpSetupResult) => void;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  const [issuer, setIssuer] = useState(pending.issuer ?? prefill.issuer);
  const [account, setAccount] = useState(pending.accountName ?? prefill.account);
  const [target, setTarget] = useState<Target>({ kind: "item" });
  const timeBased = pending.scheme !== "hotp";
  const choosable = itemKind === "login" && hardwareVaults(vaults).length > 0;
  const label = (value: string): string | null => (value.trim() === "" ? null : value.trim());

  return (
    <div className="stack">
      <h3 className="otp-setup__heading">{t("otp.confirm.title")}</h3>
      <TextField
        label={t("otp.confirm.issuer")}
        autoComplete="off"
        value={issuer}
        onChange={(event) => {
          setIssuer(event.target.value);
        }}
      />
      <TextField
        label={t("otp.confirm.account")}
        autoComplete="off"
        value={account}
        onChange={(event) => {
          setAccount(event.target.value);
        }}
      />
      <Summary pending={pending} />
      {pending.strength === "weak" && <Callout tone="warning">{t("otp.confirm.weak")}</Callout>}
      {timeBased ? (
        <div>
          <div className="rows">
            <TotpCode
              source={{ kind: "pending", pendingId: pending.pendingId }}
              initiallyShown
              label={t("otp.confirm.codeLabel")}
            />
          </div>
          <p className="field__hint">{t("otp.confirm.codeHint")}</p>
        </div>
      ) : (
        <Callout tone="info">{t("otp.confirm.hotpNote")}</Callout>
      )}
      {clipboardCleared && <p className="field__hint">{t("otp.clipboardCopies", { platform })}</p>}
      {choosable && <StoragePicker vaults={vaults} target={target} onChange={setTarget} />}
      <div className="dialog__actions">
        <Button onClick={onBack}>{t("otp.confirm.back")}</Button>
        <HoloButton
          onClick={() => {
            onUse({ pending, issuer: label(issuer), accountName: label(account), target });
          }}
        >
          {t("otp.confirm.use")}
        </HoloButton>
      </div>
    </div>
  );
}
