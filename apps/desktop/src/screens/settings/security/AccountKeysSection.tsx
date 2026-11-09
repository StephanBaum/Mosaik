/**
 * Settings > Security > "Password and keys" (M7): change the master password, rotate the Secret
 * Key, upgrade the protection and re-key the account. Each opens its wizard.
 */
import { useState } from "react";
import { Button } from "../../../components/Button.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import { SettingsSection } from "../../SettingsSection.tsx";
import { ChangePasswordDialog } from "./ChangePasswordDialog.tsx";
import type { WizardKind } from "./CredentialsStep.tsx";
import { focusQuickUnlock, useBackendOverview } from "./hooks.ts";
import { IgnoredNotices } from "./IgnoredNotices.tsx";
import { RotationWizard } from "./RotationWizard.tsx";

const ACTIONS: readonly WizardKind[] = ["password", "secretKey", "profile", "rekey"];

export function AccountKeysSection() {
  const { t } = useI18n();
  const overview = useBackendOverview();
  const [open, setOpen] = useState<WizardKind | null>(null);
  const hasBackend = overview !== null && overview.config !== null;

  function close(focusQuick: boolean) {
    setOpen(null);
    if (focusQuick) {
      focusQuickUnlock();
    }
  }

  return (
    <>
      <SettingsSection title={t("rotation.section.title")} body={t("rotation.section.body")}>
      {ACTIONS.map((kind) => {
        const blocked = kind === "rekey" && overview !== null && !hasBackend;
        return (
          <div key={kind} className="setting-row">
            <div className="setting-row__label">
              <p className="body">{t(`rotation.action.${kind}`)}</p>
              <p className="caption">{t(`rotation.action.${kind}.body`)}</p>
              {blocked && <p className="field__hint">{t("rotation.action.rekey.needsSync")}</p>}
            </div>
            <Button
              disabled={blocked}
              aria-label={t(`rotation.action.${kind}`)}
              onClick={() => {
                setOpen(kind);
              }}
            >
              {t(`rotation.action.${kind}.button`)}
            </Button>
          </div>
        );
      })}
      {open === "password" && <ChangePasswordDialog onClose={close} />}
      {open !== null && open !== "password" && <RotationWizard kind={open} onClose={close} />}
    </SettingsSection>
    <IgnoredNotices />
    </>
  );
}
