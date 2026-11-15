/** The import step machine: what is chosen, detected, previewed and committed, and the calls between. */
import { useEffect, useState } from "react";
import { useEvents } from "../../app/EventsProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type {
  ImportDetected,
  ImportPreview,
  ImportResult,
  IpcError,
  IpcResult,
  VaultSummary,
} from "../../ipc/contract/index.ts";
import { formatFor, sourceById, type SourceId } from "./sources.ts";

export interface Chosen {
  source: SourceId;
  token: string;
  fileName: string;
  detected: ImportDetected;
}
export type Step =
  | { name: "source" }
  | { name: "steps"; source: SourceId }
  | ({ name: "file" } & Chosen)
  | ({ name: "preview"; preview: ImportPreview } & Chosen)
  | ({ name: "cleanup"; result: ImportResult } & Chosen);

/** Only the chosen file's fields (a step object also carries its own name and preview). */
function pickChosen({ source, token, fileName, detected }: Chosen): Chosen {
  return { source, token, fileName, detected };
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

export interface ImportFlowState {
  step: Step;
  setStep: (step: Step) => void;
  vaults: VaultSummary[];
  busy: boolean;
  error: IpcError | null;
  setError: (error: IpcError | null) => void;
  choose: (source: SourceId) => Promise<void>;
  preview: (file: Chosen, password: string) => Promise<void>;
  commit: (file: Chosen, previewId: string, targetVaultId: string, skip: number[]) => Promise<void>;
  cancel: () => Promise<void>;
}

export function useImportFlow(onClose: () => void): ImportFlowState {
  const ipc = useIpc();
  const { importDrop, clearImportDrop } = useEvents();
  const [step, setStep] = useState<Step>({ name: "source" });
  const vaults = useVaults();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  /** The detection answer for a file: on to the file step, or an error and stay on the steps. */
  function settle(source: SourceId, token: string, fileName: string, detected: IpcResult<ImportDetected>) {
    if (!detected.ok) {
      setError(detected.error);
    } else if (formatFor(sourceById(source), detected.value) === null) {
      setError({ code: "importFormatInvalid" });
    } else {
      setError(null);
      setStep({ name: "file", source, token, fileName, detected: detected.value });
    }
  }
  async function accept(source: SourceId, token: string, fileName: string) {
    settle(source, token, fileName, await ipc.call("import_detect", { fileToken: token }));
  }
  async function choose(source: SourceId) {
    setError(null);
    const result = await ipc.call("import_choose_file", null);
    if (!result.ok) {
      setError(result.error);
    } else if (result.value.kind === "chosen") {
      await accept(source, result.value.fileToken, result.value.fileName);
    }
  }

  // A file dropped on the window while the steps are showing is taken like a chosen one.
  const stepsSource = step.name === "steps" ? step.source : null;
  useEffect(() => {
    if (stepsSource === null || importDrop === null) {
      return;
    }
    clearImportDrop();
    const { fileToken, fileName } = importDrop;
    void ipc.call("import_detect", { fileToken }).then((detected) => {
      settle(stepsSource, fileToken, fileName, detected);
    });
  }, [ipc, stepsSource, importDrop, clearImportDrop]);

  async function preview(file: Chosen, password: string) {
    const format = formatFor(sourceById(file.source), file.detected);
    if (format === null) {
      return;
    }
    setError(null);
    setBusy(true);
    const result = await ipc.call("import_preview", {
      fileToken: file.token,
      format,
      password: format === "keepass" ? password : null,
    });
    setBusy(false);
    if (result.ok) {
      setStep({ name: "preview", ...pickChosen(file), preview: result.value });
    } else {
      setError(result.error);
    }
  }

  async function commit(file: Chosen, previewId: string, targetVaultId: string, skip: number[]) {
    setError(null);
    const result = await ipc.call("import_commit", { previewId, targetVaultId, skip });
    if (result.ok) {
      setStep({ name: "cleanup", ...pickChosen(file), result: result.value });
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

  return { step, setStep, vaults, busy, error, setError, choose, preview, commit, cancel };
}
