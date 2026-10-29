/**
 * Import (D05): choose the format and the file (Rust opens the dialog and reads the file),
 * preview with duplicate hints, pick the target vault, commit. The export's contents never
 * reach this component: only titles, counts and warnings.
 */
import { useEffect, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Select } from "../../components/Select.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import {
  importFormats,
  type ImportFormat,
  type ImportPreview,
  type ImportResult,
  type IpcError,
  type VaultSummary,
} from "../../ipc/contract/index.ts";
import { PreviewList } from "./PreviewList.tsx";

type Step =
  | { name: "format" }
  | { name: "file"; token: string; fileName: string }
  | { name: "preview"; preview: ImportPreview }
  | { name: "result"; result: ImportResult };

function FormatStep({
  format,
  onFormat,
  onChoose,
}: {
  format: ImportFormat;
  onFormat: (format: ImportFormat) => void;
  onChoose: () => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <Select
        label={t("import.flow.formatStep")}
        value={format}
        options={importFormats.map((each) => ({ value: each, label: t(`import.format.${each}`) }))}
        onChange={(event) => {
          onFormat(importFormats.find((each) => each === event.target.value) ?? "enpass");
        }}
      />
      <HoloButton onClick={onChoose}>{t("import.flow.chooseFile")}</HoloButton>
    </>
  );
}

function FileStep({
  fileName,
  keepass,
  busy,
  onPreview,
}: {
  fileName: string;
  keepass: boolean;
  busy: boolean;
  onPreview: (password: string) => void;
}) {
  const { t } = useI18n();
  const [password, setPassword] = useState("");
  return (
    <>
      <p className="body">{t("import.flow.fileChosen", { name: fileName })}</p>
      {keepass && (
        <TextField
          label={t("import.flow.password")}
          type="password"
          autoComplete="off"
          value={password}
          onChange={(event) => {
            setPassword(event.target.value);
          }}
        />
      )}
      <p className="status" role="status">
        {busy ? t("import.flow.previewBusy") : ""}
      </p>
      <HoloButton
        disabled={busy}
        onClick={() => {
          onPreview(password);
          setPassword("");
        }}
      >
        {t("import.flow.preview")}
      </HoloButton>
    </>
  );
}

function PreviewStep({
  preview,
  vaults,
  onCommit,
}: {
  preview: ImportPreview;
  vaults: readonly VaultSummary[];
  onCommit: (targetVaultId: string, skip: number[]) => void;
}) {
  const { t } = useI18n();
  // Entries that look like an existing item, or repeat one in the file, start unticked.
  const [selected, setSelected] = useState<ReadonlySet<number>>(
    () =>
      new Set(
        preview.entries
          .filter((entry) => entry.duplicateOf === null && entry.duplicateInFile === null)
          .map((entry) => entry.index),
      ),
  );
  const [target, setTarget] = useState(vaults[0]?.vaultId ?? "");
  return (
    <>
      <PreviewList preview={preview} selected={selected} onChange={setSelected} />
      <Select
        label={t("import.flow.target")}
        value={target}
        options={vaults.map((vault) => ({ value: vault.vaultId, label: vault.name ?? t("vault.unnamed") }))}
        onChange={(event) => {
          setTarget(event.target.value);
        }}
      />
      {selected.size === 0 && <p className="field__hint">{t("import.flow.noneSelected")}</p>}
      <HoloButton
        disabled={selected.size === 0 || target === ""}
        onClick={() => {
          onCommit(
            target,
            preview.entries.filter((e) => !selected.has(e.index)).map((e) => e.index),
          );
        }}
      >
        {t("import.flow.commit", { count: selected.size })}
      </HoloButton>
    </>
  );
}

function ResultStep({ result, onDone }: { result: ImportResult; onDone: () => void }) {
  const { t } = useI18n();
  return (
    <>
      <p className="body" role="status">
        {t("import.result", { imported: result.imported, skipped: result.skipped })}{" "}
        {t("import.flow.resultFailed", { failed: result.failed })}
      </p>
      <p className="body muted">{t("import.flow.deleteReminder")}</p>
      <HoloButton onClick={onDone}>{t("action.done")}</HoloButton>
    </>
  );
}

function useVaults(): VaultSummary[] {
  const ipc = useIpc();
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  useEffect(() => {
    void ipc.call("list_vaults", null).then((result) => {
      if (result.ok) {
        setVaults(result.value);
      }
    });
  }, [ipc]);
  return vaults;
}

export function ImportFlow({ onClose, embedded = false }: { onClose: () => void; embedded?: boolean }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [format, setFormat] = useState<ImportFormat>("enpass");
  const [step, setStep] = useState<Step>({ name: "format" });
  const vaults = useVaults();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function choose() {
    setError(null);
    const result = await ipc.call("import_choose_file", { format });
    if (!result.ok) {
      setError(result.error);
    } else if (result.value.kind === "chosen") {
      setStep({ name: "file", token: result.value.fileToken, fileName: result.value.fileName });
    }
  }

  async function preview(token: string, password: string) {
    setError(null);
    setBusy(true);
    const result = await ipc.call("import_preview", {
      fileToken: token,
      format,
      password: format === "keepass" ? password : null,
    });
    setBusy(false);
    if (result.ok) {
      setStep({ name: "preview", preview: result.value });
    } else {
      setError(result.error);
    }
  }

  async function commit(previewId: string, targetVaultId: string, skip: number[]) {
    setError(null);
    const result = await ipc.call("import_commit", { previewId, targetVaultId, skip });
    if (result.ok) {
      setStep({ name: "result", result: result.value });
    } else {
      setError(result.error);
    }
  }

  async function cancel() {
    if (step.name === "preview") {
      await ipc.call("import_cancel", { previewId: step.preview.previewId });
    }
    onClose();
  }

  const body = (
    <div className={embedded ? "stack" : "panel panel--wide"}>
      <h1 className="title">{t("import.flow.title")}</h1>
      {step.name === "format" && (
        <FormatStep format={format} onFormat={setFormat} onChoose={() => void choose()} />
      )}
      {step.name === "file" && (
        <FileStep
          fileName={step.fileName}
          keepass={format === "keepass"}
          busy={busy}
          onPreview={(password) => void preview(step.token, password)}
        />
      )}
      {step.name === "preview" && (
        <PreviewStep
          preview={step.preview}
          vaults={vaults}
          onCommit={(target, skip) => void commit(step.preview.previewId, target, skip)}
        />
      )}
      {step.name === "result" && <ResultStep result={step.result} onDone={onClose} />}
      <ErrorAlert error={error} />
      {step.name !== "result" && <Button onClick={() => void cancel()}>{t("import.flow.cancel")}</Button>}
    </div>
  );
  return <Frame embedded={embedded}>{body}</Frame>;
}

function Frame({ embedded, children }: { embedded: boolean; children: React.ReactNode }) {
  const { t } = useI18n();
  return embedded ? (
    <section aria-label={t("import.flow.title")}>{children}</section>
  ) : (
    <main id="main" className="screen screen--centered">
      {children}
    </main>
  );
}
