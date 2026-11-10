/**
 * The rotate-now queue (D13): the passwords worth changing first, with "Change password"
 * (the guided panel), "Snooze" (1, 7 or 30 days) and "Dismiss" (until the password changes).
 * Snoozed items are listed apart. Mosaic only guides: nothing here opens a page.
 */
import { useEffect, useRef, useState } from "react";
import { Avatar } from "../../components/Avatar.tsx";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { Icon, type IconName } from "../../components/icons.tsx";
import { MenuButton } from "../../components/Menu.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import {
  snoozeDays,
  type IpcError,
  type ItemRequest,
  type QueueEntry,
  type QueueReason,
} from "../../ipc/contract/index.ts";
import { RotationPanel } from "./RotationPanel.tsx";
import type { QueueState } from "./useRotationQueue.ts";

const REASON_ICON: Record<QueueReason, IconName> = {
  weak: "alert",
  reused: "copy",
  old: "clock",
  breached: "shieldAlert",
  unfinished: "restore",
};

/** What the row says about the item: its title, who and where, why it is listed, until when it sleeps. */
function RowSummary({ entry, title }: { entry: QueueEntry; title: string }) {
  const { t } = useI18n();
  const where = [entry.username, entry.host].filter((part) => part !== null).join(" · ");
  return (
    <div className="row__main">
      <span className="row__value">{title}</span>
      {where !== "" && <span className="row__label">{where}</span>}
      <span className="queue__reasons">
        {entry.reasons.map((reason) => (
          <span key={reason} className={`issue issue--${entry.severity}`}>
            <Icon name={REASON_ICON[reason]} size="sm" />
            <span className="visually-hidden">{t(`insights.severity.${entry.severity}`)}: </span>
            {t(`credentialRotation.queue.reason.${reason}`)}
          </span>
        ))}
      </span>
      {entry.snoozedUntil !== null && (
        <span className="caption">{t("credentialRotation.queue.snoozedUntil", { date: entry.snoozedUntil })}</span>
      )}
      {entry.kind === "manual" && <span className="caption">{t("credentialRotation.queue.manualHint")}</span>}
    </div>
  );
}

/** Snooze and dismiss: each answers with a toast and asks the queue to read again. */
function useRowCommands(entry: QueueEntry, onError: (error: IpcError | null) => void, onDone: () => void) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const item = { vaultId: entry.vaultId, itemId: entry.itemId };
  async function settle(result: { ok: true } | { ok: false; error: IpcError }, message: string) {
    if (result.ok) {
      onError(null);
      toast(message);
      onDone();
    } else {
      onError(result.error);
    }
    await Promise.resolve();
  }
  return {
    snooze: (days: (typeof snoozeDays)[number]) =>
      ipc.call("rotation_snooze", { ...item, days }).then((r) => settle(r, t("credentialRotation.queue.snoozed", { days }))),
    dismiss: () => ipc.call("rotation_dismiss", item).then((r) => settle(r, t("credentialRotation.queue.dismissed"))),
  };
}

function Row({
  entry,
  onGuide,
  onOpen,
  onError,
  onDone,
}: {
  entry: QueueEntry;
  onGuide: (entry: QueueEntry) => void;
  onOpen: (item: ItemRequest) => void;
  onError: (error: IpcError | null) => void;
  onDone: () => void;
}) {
  const { t } = useI18n();
  const commands = useRowCommands(entry, onError, onDone);
  const title = entry.title ?? t("vault.untitled");
  const guided = entry.kind === "guided";
  const actionKey = !guided
    ? "credentialRotation.queue.editItem"
    : entry.attempts.length > 0
      ? "credentialRotation.queue.continue"
      : "credentialRotation.queue.change";
  const action = t(actionKey);
  return (
    <li className="row">
      <Avatar title={title} />
      <RowSummary entry={entry} title={title} />
      <div className="queue__actions">
        {/* The accessible name starts with the visible text (WCAG 2.5.3) and names the item. */}
        <Button
          size="sm"
          aria-label={t("credentialRotation.queue.actionFor", { action, title })}
          onClick={() => {
            if (guided) {
              onGuide(entry);
            } else {
              onOpen({ vaultId: entry.vaultId, itemId: entry.itemId });
            }
          }}
        >
          {action}
        </Button>
        {entry.snoozedUntil === null && (
          <MenuButton
            label={t("credentialRotation.queue.snoozeFor", { title })}
            icon="clock"
            items={snoozeDays.map((days) => ({
              label: t("credentialRotation.queue.snoozeDays", { days }),
              onSelect: () => {
                void commands.snooze(days);
              },
            }))}
          />
        )}
        <Button
          size="sm"
          variant="ghost"
          aria-label={t("credentialRotation.queue.dismissFor", { title })}
          onClick={() => {
            void commands.dismiss();
          }}
        >
          {t("credentialRotation.queue.dismiss")}
        </Button>
      </div>
    </li>
  );
}

/**
 * The entry the dashboard asked for (`openFor`): its panel if Mosaic can guide it, else the item
 * itself. Derived while rendering; the item is opened once, in an effect.
 */
function useRequestedEntry(
  queue: QueueState["queue"],
  openFor: ItemRequest | null | undefined,
  onOpenItem: (item: ItemRequest) => void,
): QueueEntry | null {
  const entry =
    openFor === null || openFor === undefined
      ? undefined
      : queue?.entries.find((e) => e.itemId === openFor.itemId && e.vaultId === openFor.vaultId);
  const opened = useRef<ItemRequest | null>(null);
  const unguided = queue !== null && openFor !== null && openFor !== undefined && entry?.kind !== "guided";
  useEffect(() => {
    if (unguided && opened.current !== openFor) {
      opened.current = openFor;
      onOpenItem(openFor);
    }
  }, [unguided, openFor, onOpenItem]);
  return entry?.kind === "guided" ? entry : null;
}

/** The due entries, then the snoozed ones apart. */
function QueueLists({
  queue,
  onGuide,
  onOpen,
  onError,
  onDone,
}: {
  /** `null` while it is read. */
  queue: QueueState["queue"];
  onGuide: (entry: QueueEntry) => void;
  onOpen: (item: ItemRequest) => void;
  onError: (error: IpcError | null) => void;
  onDone: () => void;
}) {
  const { t } = useI18n();
  const entries = queue?.entries ?? [];
  const due = entries.filter((entry) => entry.snoozedUntil === null);
  const snoozed = entries.filter((entry) => entry.snoozedUntil !== null);
  const rows = (list: readonly QueueEntry[], label: string) => (
    <ol className="rows rows--closed" aria-label={label}>
      {list.map((entry) => (
        <Row
          key={`${entry.vaultId}/${entry.itemId}`}
          entry={entry}
          onGuide={onGuide}
          onOpen={onOpen}
          onError={onError}
          onDone={onDone}
        />
      ))}
    </ol>
  );
  return (
    <>
      {queue !== null && entries.length === 0 && <p className="lede">{t("credentialRotation.queue.empty")}</p>}
      {due.length > 0 && (
        <section className="security__section">
          <p className="caption">{t("credentialRotation.queue.count", { count: due.length })}</p>
          {rows(due, t("credentialRotation.queue.list"))}
        </section>
      )}
      {snoozed.length > 0 && (
        <section className="security__section">
          <h3 className="subtitle">{t("credentialRotation.queue.snoozedHeading")}</h3>
          {rows(snoozed, t("credentialRotation.queue.snoozedHeading"))}
        </section>
      )}
    </>
  );
}

/** Which panel is open: the one the user chose, or the one the dashboard asked for until it is closed. */
function usePanel(queue: QueueState["queue"], openFor: ItemRequest | null | undefined, onOpenItem: (item: ItemRequest) => void) {
  const [chosen, setChosen] = useState<QueueEntry | null>(null);
  const [closedFor, setClosedFor] = useState<ItemRequest | null>(null);
  const requested = useRequestedEntry(queue, openFor, onOpenItem);
  return {
    panel: chosen ?? (closedFor === openFor ? null : requested),
    choose: setChosen,
    close: () => {
      setChosen(null);
      setClosedFor(openFor ?? null);
    },
  };
}

export function RotationQueue({
  state,
  onOpenItem,
  openFor,
  onChanged,
}: {
  state: QueueState;
  onOpenItem: (item: ItemRequest) => void;
  /** Open the panel of this item as soon as the queue is read (the dashboard's "Change password"). */
  openFor?: ItemRequest | null;
  /** A change was committed: the dashboard reads its report again. */
  onChanged?: () => void;
}) {
  const { t } = useI18n();
  const { queue, error, reload } = state;
  const [rowError, setRowError] = useState<IpcError | null>(null);
  const { panel, choose, close } = usePanel(queue, openFor, onOpenItem);
  const done = () => {
    reload();
    onChanged?.();
  };

  return (
    <section className="security" aria-label={t("credentialRotation.queue.title")}>
      <div className="page__heading">
        <h2 className="subtitle">{t("credentialRotation.queue.title")}</h2>
        <p className="lede">{t("credentialRotation.queue.intro")}</p>
      </div>
      <ErrorAlert error={error ?? rowError} />
      <QueueLists queue={queue} onGuide={choose} onOpen={onOpenItem} onError={setRowError} onDone={done} />
      {panel !== null && (
        <RotationPanel
          entry={panel}
          onClose={() => {
            close();
            done();
          }}
        />
      )}
    </section>
  );
}
