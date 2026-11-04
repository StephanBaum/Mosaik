/**
 * The one-time code setup dialog (ADR-0031): scan (the click that opened it is the user
 * gesture), type the key, then confirm. It never writes; the item editor saves. It is
 * rendered into the document body so its form is not nested in the editor's.
 */
import { useEffect, useRef, type KeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Button } from "../../../components/Button.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { useIpcEvent } from "../../../hooks/useIpcEvent.ts";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { ItemKind, VaultSummary } from "../../../ipc/contract/index.ts";
import { usePlatform } from "../../../platform/platform.ts";
import { OtpConfirm } from "./OtpConfirm.tsx";
import { OtpManualForm } from "./OtpManualForm.tsx";
import type { OtpSetupResult, OtpStart } from "./OtpSection.tsx";
import { OtpSources } from "./OtpSources.tsx";
import { useOtpScan, type OtpScanApi } from "./useOtpScan.ts";

/** A paste into a text control is the control's own business. */
function isTextTarget(target: EventTarget): boolean {
  return (
    target instanceof HTMLElement && target.closest("input, textarea, [contenteditable]") !== null
  );
}

function Working({ image }: { image: boolean }) {
  const { t } = useI18n();
  return (
    <p className="status otp-setup__working" role="status" aria-busy="true">
      {t(image ? "otp.working.image" : "otp.working.screen")}
    </p>
  );
}

function Body({
  api,
  screenScan,
  prefill,
  vaults,
  itemKind,
  onClose,
  onDone,
}: {
  api: OtpScanApi;
  screenScan: boolean;
  prefill: { issuer: string; account: string };
  vaults: readonly VaultSummary[];
  itemKind: ItemKind;
  onClose: () => void;
  onDone: (result: OtpSetupResult) => void;
}): ReactNode {
  const { t } = useI18n();
  const { step } = api;
  if (step.kind === "working") {
    return (
      <>
        <Working image={step.source !== "screen"} />
        <div className="dialog__actions">
          <Button onClick={onClose}>{t("action.cancel")}</Button>
        </div>
      </>
    );
  }
  if (step.kind === "manual") {
    return <OtpManualForm prefill={prefill} onSubmit={api.parse} onCancel={onClose} />;
  }
  if (step.kind === "confirm") {
    return (
      <OtpConfirm
        pending={step.pending}
        clipboardCleared={step.clipboardCleared}
        prefill={prefill}
        vaults={vaults}
        itemKind={itemKind}
        onBack={api.choose}
        onUse={(result) => {
          api.handOver();
          onDone(result);
        }}
      />
    );
  }
  return (
    <div className="stack">
      <ErrorAlert error={step.error} />
      <OtpSources
        screenScan={screenScan}
        onPick={(start) => {
          if (start.kind === "scan") {
            void api.scan(start.source);
          } else {
            api.manual();
          }
        }}
      />
      <div className="dialog__actions">
        <Button onClick={onClose}>{t("action.cancel")}</Button>
      </div>
    </div>
  );
}

export function OtpSetupDialog({
  start,
  prefill,
  vaults,
  itemKind,
  screenScan,
  onDone,
  onClose,
}: {
  start: OtpStart;
  prefill: { issuer: string; account: string };
  vaults: readonly VaultSummary[];
  itemKind: ItemKind;
  screenScan: boolean;
  onDone: (result: OtpSetupResult) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  const api = useOtpScan(start, onClose);
  const { scan, discard } = api;
  const started = useRef(false);

  useEffect(() => {
    // Once, even if effects run twice in development: one click, one capture.
    if (!started.current && start.kind === "scan") {
      started.current = true;
      void scan(start.source);
    }
  }, [start, scan]);

  // A setup the editor did not take is forgotten when the dialog goes away (a lock
  // clears them in Rust anyway).
  useEffect(() => discard, [discard]);

  function close() {
    discard();
    onClose();
  }

  // Nothing of a setup survives a lock.
  useIpcEvent((event) => {
    if (event.type === "locked") {
      close();
    }
  });

  function onKeyDown(event: KeyboardEvent) {
    const modifier = platform === "mac" ? event.metaKey : event.ctrlKey;
    const paste = modifier && event.key.toLowerCase() === "v";
    if (paste && api.step.kind !== "manual" && !isTextTarget(event.target)) {
      // The webview's own paste would read the image; Rust reads it instead.
      event.preventDefault();
      void scan("clipboard");
    }
  }

  return createPortal(
    <Dialog title={t("otp.dialog.title")} onClose={close} size="md">
      <div className="otp-setup" onKeyDown={onKeyDown}>
        <Body
          api={api}
          screenScan={screenScan}
          prefill={prefill}
          vaults={vaults}
          itemKind={itemKind}
          onClose={close}
          onDone={onDone}
        />
      </div>
    </Dialog>,
    document.body,
  );
}
