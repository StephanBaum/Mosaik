/**
 * The new Emergency Kit of a rotation: saved through the native dialog (Rust renders the PDF from
 * the staged Secret Key, which never reaches the webview) and confirmed by typing its last group.
 * The backend learns nothing before this step is done.
 */
import { useEffect, useRef, useState, type SyntheticEvent } from "react";
import { Callout } from "../../../components/Callout.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { HoloButton } from "../../../components/HoloButton.tsx";
import { Button } from "../../../components/Button.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { IpcError } from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import { MASKED_KEY_PREFIX, kitLabels } from "../../onboarding/kit.ts";
import { StepHeading } from "../../sharing/StepHeading.tsx";

// eslint-disable-next-line max-lines-per-function -- save and verify are one two-part step.
export function NewKitStep({
  onBack,
  onVerified,
}: {
  onBack: () => void;
  onVerified: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [saved, setSaved] = useState(false);
  const [cancelled, setCancelled] = useState(false);
  const [lastGroup, setLastGroup] = useState("");
  const [mistyped, setMistyped] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);
  const form = useRef<HTMLFormElement>(null);

  // The save button is swapped for the verify form: focus goes to the field, not to the page.
  useEffect(() => {
    if (saved) {
      form.current?.querySelector("input")?.focus();
    }
  }, [saved]);

  async function save() {
    setBusy(true);
    setError(null);
    setCancelled(false);
    const result = await ipc.call("save_emergency_kit", { labels: kitLabels(t), pending: true, password: null });
    setBusy(false);
    if (!result.ok) {
      setError(result.error);
    } else if (result.value.kind === "saved") {
      setSaved(true);
    } else {
      setCancelled(true);
    }
  }

  async function verify(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const result = await ipc.call("verify_emergency_kit", { lastGroup, pending: true });
    setBusy(false);
    setLastGroup("");
    if (!result.ok) {
      setError(result.error);
    } else if (result.value.matches) {
      onVerified();
    } else {
      setMistyped(true);
    }
  }

  return (
    <div className="stack">
      <StepHeading>{t("rotation.kit.title")}</StepHeading>
      <p className="body muted">{t("rotation.kit.body")}</p>
      <div className="actions">
        {saved ? (
          <Button
            disabled={busy}
            onClick={() => {
              void save();
            }}
          >
            {t("rotation.kit.save")}
          </Button>
        ) : (
          <HoloButton
            disabled={busy}
            onClick={() => {
              void save();
            }}
          >
            {t("rotation.kit.save")}
          </HoloButton>
        )}
      </div>
      {cancelled && <p className="caption">{t("rotation.kit.cancelled")}</p>}
      {saved && (
        <form ref={form} className="stack" onSubmit={(event) => void verify(event)} noValidate>
          <Callout tone="info">{t("rotation.kit.saved")}</Callout>
          <p className="masked-key" aria-hidden="true">
            <span className="muted">{MASKED_KEY_PREFIX}</span>
            <span className="masked-key__slot">____</span>
          </p>
          <TextField
            label={t("rotation.kit.verify")}
            className="field__input--code"
            mono
            maxLength={4}
            autoCapitalize="characters"
            autoComplete="off"
            spellCheck={false}
            error={mistyped ? t("rotation.kit.mistyped") : undefined}
            value={lastGroup}
            onChange={(event) => {
              setLastGroup(event.target.value);
              setMistyped(false);
            }}
          />
          <ErrorAlert error={error} />
          <div className="dialog__actions">
            <Button variant="ghost" onClick={onBack}>
              {t("rotation.back")}
            </Button>
            <Button type="submit" disabled={busy || lastGroup === ""}>
              {t("rotation.kit.confirm")}
            </Button>
          </div>
        </form>
      )}
      {!saved && (
        <>
          <ErrorAlert error={error} />
          <div className="dialog__actions">
            <Button variant="ghost" onClick={onBack}>
              {t("rotation.back")}
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
