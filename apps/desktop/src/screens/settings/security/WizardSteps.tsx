/** The review, confirmation and done steps of the rotation wizard. */
import { Button } from "../../../components/Button.tsx";
import { Callout } from "../../../components/Callout.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { HoloButton } from "../../../components/HoloButton.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { IpcError, RekeyReviewView, RotationDone } from "../../../ipc/contract/index.ts";
import { StepHeading } from "../../sharing/StepHeading.tsx";
import type { WizardKind } from "./CredentialsStep.tsx";
import { CredentialReview, RekeyReviewPanel } from "./ReviewLists.tsx";

/** The review of a rekey before the verb: loads `rekey_review` and shows what it touches. */
export function ConfirmStep({
  kind,
  review,
  reviewError,
  busy,
  onBack,
  onRun,
}: {
  kind: "secretKey" | "rekey";
  review: RekeyReviewView | null;
  reviewError: IpcError | null;
  busy: boolean;
  onBack: () => void;
  onRun: () => void;
}) {
  const { t } = useI18n();
  const rekey = kind === "rekey";
  const ready = !rekey || review !== null;
  return (
    <div className="stack">
      <StepHeading>{t(`rotation.confirm.title.${kind}`)}</StepHeading>
      {rekey ? (
        review === null ? (
          reviewError === null ? (
            <p className="body muted" role="status">
              {t("rotation.review.loading")}
            </p>
          ) : (
            <ErrorAlert error={reviewError} />
          )
        ) : (
          <RekeyReviewPanel review={review} />
        )
      ) : null}
      <p className="body">{t(rekey ? "rotation.confirm.body.rekey" : "rotation.confirm.body.secretKey")}</p>
      <div className="dialog__actions">
        <Button variant="ghost" onClick={onBack}>
          {t("rotation.back")}
        </Button>
        {rekey ? (
          <Button variant="dangerSolid" disabled={busy || !ready} onClick={onRun}>
            {t("rotation.confirm.verb.rekey")}
          </Button>
        ) : (
          <HoloButton disabled={busy} onClick={onRun}>
            {t("rotation.confirm.verb.secretKey")}
          </HoloButton>
        )}
      </div>
    </div>
  );
}

export function ReviewStep({
  kind,
  hasBackend,
  onCancel,
  onNext,
  onSwitch,
}: {
  kind: WizardKind;
  hasBackend: boolean;
  onCancel: () => void;
  onNext: () => void;
  onSwitch: (kind: WizardKind) => void;
}) {
  const { t } = useI18n();
  return (
    <div className="stack">
      <StepHeading>{t(`rotation.review.title.${kind}`)}</StepHeading>
      {kind === "rekey" ? (
        <>
          <p className="body">{t("rotation.review.rekey.intro")}</p>
          {!hasBackend && <Callout tone="warning">{t("rotation.review.rekey.unavailable")}</Callout>}
        </>
      ) : (
        <CredentialReview kind={kind} />
      )}
      {kind === "secretKey" && (
        <div>
          <Button
            variant="ghost"
            onClick={() => {
              onSwitch("rekey");
            }}
          >
            {t("rotation.review.secretKey.compromised")}
          </Button>
        </div>
      )}
      <div className="dialog__actions">
        {kind === "rekey" ? (
          <Button
            onClick={() => {
              onSwitch("secretKey");
            }}
          >
            {t("rotation.review.rekey.no")}
          </Button>
        ) : (
          <Button variant="ghost" onClick={onCancel}>
            {t("action.cancel")}
          </Button>
        )}
        <HoloButton disabled={kind === "rekey" && !hasBackend} onClick={onNext}>
          {kind === "rekey" ? t("rotation.review.rekey.yes") : t("rotation.next")}
        </HoloButton>
      </div>
    </div>
  );
}

/** The attachments the re-seal could not read, by item and file name. */
function UnreachableFiles({ done }: { done: RotationDone }) {
  const { t } = useI18n();
  const more = done.unreachableAttachments - done.unreachable.length;
  return (
    <Callout tone="warning">
      <p className="body">{t("rotation.done.unreachable", { count: done.unreachableAttachments })}</p>
      <p className="body">{t("rotation.done.unreachable.note")}</p>
      <ul className="stack">
        {done.unreachable.map((file, index) => (
          // The same file can appear twice (two items), so the position is part of the key.
          <li key={`${String(index)}-${file.fileName ?? ""}`} className="body">
            {t("rotation.done.unreachable.file", {
              item: file.itemTitle ?? t("rotation.done.unreachable.unknownItem"),
              file: file.fileName ?? t("rotation.done.unreachable.unknownFile"),
            })}
          </li>
        ))}
      </ul>
      {more > 0 && <p className="caption">{t("rotation.done.unreachable.more", { count: more })}</p>}
    </Callout>
  );
}

export function DoneStep({
  kind,
  done,
  quickWas,
  onClose,
}: {
  kind: WizardKind;
  done: RotationDone;
  /** Quick unlock was set up before the change (a password change cannot say what it deleted). */
  quickWas: boolean;
  onClose: (focusQuick: boolean) => void;
}) {
  const { t } = useI18n();
  const quick = done.quickUnlockRemoved || (kind !== "rekey" && quickWas);
  return (
    <div className="stack">
      <StepHeading>{t(`rotation.done.title.${kind}`)}</StepHeading>
      <p className="body">{t(`rotation.done.body.${kind}`)}</p>
      {kind === "rekey" && <Callout tone="warning">{t("rotation.done.members")}</Callout>}
      {quick && <p className="body">{t("rotation.done.quick.body")}</p>}
      {done.hardwareKeysRemoved > 0 && (
        <p className="body">{t("rotation.done.hardware.body", { count: done.hardwareKeysRemoved })}</p>
      )}
      {done.unreachableAttachments > 0 && <UnreachableFiles done={done} />}
      {!done.finished && <Callout tone="info">{t("rotation.done.unfinished")}</Callout>}
      <div className="dialog__actions">
        {(quick || done.hardwareKeysRemoved > 0) && (
          <Button
            onClick={() => {
              onClose(true);
            }}
          >
            {done.hardwareKeysRemoved > 0 && !quick ? t("rotation.done.hardware") : t("rotation.done.quick")}
          </Button>
        )}
        <HoloButton
          data-autofocus=""
          onClick={() => {
            onClose(false);
          }}
        >
          {t("rotation.close")}
        </HoloButton>
      </div>
    </div>
  );
}

