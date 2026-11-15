/** The file and preview steps of the import: detection result, password for KeePass, tick list. */
import { useState } from "react";
import { Callout } from "../../components/Callout.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { Select } from "../../components/Select.tsx";
import { TextField } from "../../components/TextField.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { ImportDetected, ImportPreview, VaultSummary } from "../../ipc/contract/index.ts";
import { NotComing } from "./NotComing.tsx";
import { PreviewList } from "./PreviewList.tsx";
import { isMismatch, type ImportSourceInfo } from "./sources.ts";

export function FileStep({
  source,
  fileName,
  detected,
  busy,
  onPreview,
}: {
  source: ImportSourceInfo;
  fileName: string;
  detected: ImportDetected;
  busy: boolean;
  onPreview: (password: string) => void;
}) {
  const { t } = useI18n();
  const [password, setPassword] = useState("");
  const keepass = detected.format === "keepass";
  return (
    <>
      <ul className="rows rows--closed">
        <li className="row">
          <span className="row__status">
            <Icon name="check" />
            <UserText>{t("import.flow.fileChosen", { name: fileName })}</UserText>
          </span>
        </li>
      </ul>
      {detected.source !== null && !isMismatch(source, detected) && (
        <p className="field__hint">
          {t("import.detect.detected", { source: t(`import.detected.${detected.source}`) })}
        </p>
      )}
      {detected.source !== null && isMismatch(source, detected) && (
        <Callout tone="info">
          {t("import.detect.mismatch", { source: t(`import.detected.${detected.source}`) })}
        </Callout>
      )}
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

export function PreviewStep({
  source,
  preview,
  vaults,
  onCommit,
}: {
  source: ImportSourceInfo;
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
      <NotComing preview={preview} source={source} />
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
