/**
 * The files of an item (ADR-0034): name, size, when they were added, and Preview, Save as and
 * Remove. A file never reaches the webview; the buttons ask Rust, which opens the native dialogs
 * and does all file I/O. There are no thumbnails.
 */
import { useId, useRef, useState } from "react";
import { Button } from "../../../components/Button.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { Icon } from "../../../components/icons.tsx";
import { IconButton } from "../../../components/IconButton.tsx";
import { UserText } from "../../../components/UserText.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type {
  AttachmentEntry,
  ItemDetail,
  ItemRequest,
} from "../../../ipc/contract/index.ts";
import { formatSize, iconFor } from "./format.ts";
import { PreviewDialog } from "./PreviewDialog.tsx";
import { useFiles, type Files } from "./useFiles.ts";

function RemoveDialog({
  entry,
  onKeep,
  onRemove,
}: {
  entry: AttachmentEntry;
  onKeep: () => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();
  return (
    <Dialog title={t("files.removeConfirm.title", { name: entry.fileName })} onClose={onKeep}>
      <p className="lede">{t("files.removeConfirm.body")}</p>
      <div className="dialog__actions">
        <Button autoFocus onClick={onKeep}>
          {t("files.removeConfirm.keep")}
        </Button>
        <Button variant="dangerSolid" onClick={onRemove}>
          {t("files.removeConfirm.confirm")}
        </Button>
      </div>
    </Dialog>
  );
}

function FileRow({
  entry,
  editable,
  files,
  onPreview,
  onRemove,
}: {
  entry: AttachmentEntry;
  editable: boolean;
  files: Files;
  onPreview: () => void;
  onRemove: () => void;
}) {
  const { t, locale } = useI18n();
  const name = entry.fileName;
  const detail =
    entry.addedAt === null ? null : t("files.addedAt", { at: new Date(entry.addedAt) });
  return (
    <li className="row files__row">
      <span className="files__icon" aria-hidden="true">
        <Icon name={iconFor(entry.mime)} />
      </span>
      <div className="row__main">
        <span className="row__value">
          <UserText>{name}</UserText>
        </span>
        <span className="row__label">
          {[formatSize(entry.size, t, locale), entry.available ? detail : t("files.notDownloaded")]
            .filter((part) => part !== null)
            .join(" · ")}
        </span>
      </div>
      <div className="files__actions">
        {entry.previewable && (
          <IconButton icon="eye" label={t("files.preview", { name })} onClick={onPreview} />
        )}
        <IconButton
          icon="download"
          label={t("files.saveAs", { name })}
          disabled={!entry.available}
          onClick={() => {
            files.saveAs(entry);
          }}
        />
        {editable && <IconButton icon="trash" label={t("files.remove", { name })} onClick={onRemove} />}
      </div>
    </li>
  );
}

/** The rows, or the empty line, and the hint that only images have a preview. */
function FilesList({
  entries,
  editable,
  files,
  onPreview,
  onRemove,
}: {
  entries: readonly AttachmentEntry[];
  editable: boolean;
  files: Files;
  onPreview: (entry: AttachmentEntry) => void;
  onRemove: (entry: AttachmentEntry) => void;
}) {
  const { t } = useI18n();
  if (entries.length === 0) {
    return <p className="files__note">{t("files.empty")}</p>;
  }
  return (
    <>
      <ul className="rows files__list">
        {entries.map((entry) => (
          <FileRow
            key={entry.attachmentId}
            entry={entry}
            editable={editable}
            files={files}
            onPreview={() => {
              onPreview(entry);
            }}
            onRemove={() => {
              onRemove(entry);
            }}
          />
        ))}
      </ul>
      {entries.some((entry) => entry.available && !entry.previewable) && (
        <p className="files__note">{t("files.previewOnlyImages")}</p>
      )}
    </>
  );
}

export function FilesSection({
  detail,
  item,
  editable,
  autoFocusAdd,
  note,
  onChanged,
}: {
  detail: ItemDetail;
  item: ItemRequest;
  editable: boolean;
  autoFocusAdd: boolean;
  /** A caption under the heading (the editor says that files are saved at once). */
  note?: string | undefined;
  onChanged: (detail: ItemDetail) => void;
}) {
  const { t } = useI18n();
  const headingId = useId();
  const files = useFiles(item, onChanged);
  const addButton = useRef<HTMLButtonElement>(null);
  const [previewing, setPreviewing] = useState<AttachmentEntry | null>(null);
  const [removing, setRemoving] = useState<AttachmentEntry | null>(null);
  if (!editable && detail.attachments.length === 0) {
    return null;
  }
  return (
    <section className="files" aria-labelledby={headingId}>
      <h3 id={headingId} className="files__title">
        {t("files.title")}
      </h3>
      {note !== undefined && <p className="files__note">{note}</p>}
      <FilesList
        entries={detail.attachments}
        editable={editable}
        files={files}
        onPreview={setPreviewing}
        onRemove={setRemoving}
      />
      {editable && (
        <div>
          <Button
            ref={addButton}
            autoFocus={autoFocusAdd}
            aria-busy={files.adding}
            disabled={files.adding}
            onClick={files.add}
          >
            {t("files.add")}
          </Button>
        </div>
      )}
      <ErrorAlert error={files.error} />
      {previewing !== null && (
        <PreviewDialog
          item={item}
          attachment={previewing}
          onClose={() => {
            setPreviewing(null);
          }}
          onSaveAs={() => {
            files.saveAs(previewing);
          }}
        />
      )}
      {removing !== null && (
        <RemoveDialog
          entry={removing}
          onKeep={() => {
            setRemoving(null);
          }}
          onRemove={() => {
            void files.remove(removing).then(() => {
              setRemoving(null);
              addButton.current?.focus();
            });
          }}
        />
      )}
    </section>
  );
}
