/** "Remind me to change this password": never, or every 30, 90, 180 or 365 days (M7b). No request is made for it. */
import { Select } from "../../components/Select.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { policyDays, type PolicyDays } from "../../ipc/contract/index.ts";

export function RotationPolicySelect({
  value,
  disabled,
  onChange,
}: {
  value: PolicyDays | null;
  disabled: boolean;
  onChange: (days: PolicyDays | null) => void;
}) {
  const { t } = useI18n();
  const options = [
    { value: "", label: t("credentialRotation.policy.none") },
    ...policyDays.map((days) => ({
      value: String(days),
      label: t("credentialRotation.policy.every", { days }),
    })),
  ];
  return (
    <div>
      <Select
        label={t("credentialRotation.policy.label")}
        value={value === null ? "" : String(value)}
        options={options}
        disabled={disabled}
        onChange={(event) => {
          const days = policyDays.find((each) => String(each) === event.target.value);
          onChange(days ?? null);
        }}
      />
      <p className="field__hint">{t("credentialRotation.policy.help")}</p>
    </div>
  );
}
