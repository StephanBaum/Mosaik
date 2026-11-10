/**
 * An open password change on an item (M7b): "Password change started on {date}. The new password
 * is kept here until you confirm." with It worked, It didn't work, Reveal new and Copy new. The
 * item's own password does not change until "It worked".
 */
import { Button } from "../../components/Button.tsx";
import { IconButton } from "../../components/IconButton.tsx";
import { SecretField } from "../../components/SecretField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { ChangeUrlView, ItemRequest, RotationAttemptView, RotationOutcomeKey } from "../../ipc/contract/index.ts";
import { useCopyAttempt } from "../rotation/useCopyAttempt.ts";

export function PendingRotationBanner({
  item,
  attempt,
  url,
  busy,
  onClose,
}: {
  item: ItemRequest;
  attempt: RotationAttemptView;
  url: ChangeUrlView | null;
  busy: boolean;
  onClose: (attemptId: string, outcome: RotationOutcomeKey) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const copies = useCopyAttempt();
  const request = { ...item, attemptId: attempt.attemptId };
  return (
    <section className="rotation-banner" aria-label={t("credentialRotation.banner.title", { date: attempt.startedAt })}>
      <h3 className="rotation-banner__title">{t("credentialRotation.banner.title", { date: attempt.startedAt })}</h3>
      <p className="caption">{t(`credentialRotation.banner.${attempt.stage}`)}</p>
      <SecretField
        label={t("credentialRotation.banner.newPassword")}
        reveal={() => ipc.call("rotation_reveal", request)}
        actions={
          <IconButton
            icon="copy"
            label={t("credentialRotation.banner.copy")}
            onClick={() => {
              void copies.password(request);
            }}
          />
        }
      />
      <div className="actions">
        <Button
          disabled={busy}
          onClick={() => {
            onClose(attempt.attemptId, "succeeded");
          }}
        >
          {t("credentialRotation.result.worked")}
        </Button>
        <Button
          disabled={busy}
          onClick={() => {
            onClose(attempt.attemptId, "failed");
          }}
        >
          {t("credentialRotation.result.failed")}
        </Button>
        {url !== null && (
          <Button
            variant="ghost"
            onClick={() => {
              void copies.link(url.url);
            }}
          >
            {t("credentialRotation.banner.copyLink")}
          </Button>
        )}
      </div>
    </section>
  );
}
