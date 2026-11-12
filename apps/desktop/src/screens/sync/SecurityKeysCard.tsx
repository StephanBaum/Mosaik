/**
 * The security keys of the sync account (M9, Mosaic server only): the list, Add and Remove.
 * A key is a second step at sign-in, next to or instead of an authenticator-app code. Adding or
 * confirming one runs in the system browser ([`SecurityKeyStep`]). A key without an
 * authenticator-app code as a backup is allowed, with a warning and a recommendation to keep
 * two methods; removing the last second step says that two-step verification then turns off.
 */
import { useEffect, useRef, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { UserText } from "../../components/UserText.tsx";
import { formatRelative } from "../../i18n/format.ts";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, WebauthnKeys, WebauthnKeyView } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { useNow } from "./hooks.ts";
import { AddDialog, RemoveDialog } from "./SecurityKeyDialogs.tsx";

/** The list, read again when `refreshKey` changes or `reload` is called. */
function useKeys(refreshKey: number) {
  const ipc = useIpc();
  const [keys, setKeys] = useState<WebauthnKeys | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [version, setVersion] = useState(0);
  useEffect(() => {
    let current = true;
    void ipc.call("sync_webauthn_list", null).then((result) => {
      if (!current) {
        return;
      }
      if (result.ok) {
        setKeys(result.value);
        setError(null);
      } else {
        setError(result.error);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, refreshKey, version]);
  return {
    keys,
    error,
    reload: () => {
      setVersion((n) => n + 1);
    },
  };
}

function KeyRow({ entry, onRemove }: { entry: WebauthnKeyView; onRemove: () => void }) {
  const { t, locale } = useI18n();
  const now = useNow();
  const when = (at: number) => formatRelative(locale, at, Math.max(now, at));
  return (
    <li className="row">
      <div className="row__main">
        <p className="row__value">
          <UserText>{entry.label}</UserText>
        </p>
        <p className="caption">
          {t("settings.sync.key.added", { when: when(entry.createdAt) })}
          {" · "}
          {entry.lastUsedAt === null
            ? t("settings.sync.key.neverUsed")
            : t("settings.sync.key.lastUsed", { when: when(entry.lastUsedAt) })}
        </p>
      </div>
      <Button
        variant="ghost"
        size="sm"
        aria-label={t("settings.sync.key.removeLabel", { name: entry.label })}
        onClick={onRemove}
      >
        {t("settings.sync.key.remove")}
      </Button>
    </li>
  );
}

function KeyDialogs({
  adding,
  removing,
  totp,
  count,
  onAdded,
  onRemoved,
  onCloseAdd,
  onCloseRemove,
  returnFocus,
}: {
  adding: boolean;
  removing: WebauthnKeyView | null;
  totp: boolean;
  count: number;
  onAdded: () => void;
  onRemoved: () => void;
  onCloseAdd: () => void;
  onCloseRemove: () => void;
  returnFocus: () => HTMLElement | null;
}) {
  return (
    <>
      {adding && (
        <AddDialog
          totp={totp}
          firstKey={count === 0}
          onDone={onAdded}
          onClose={onCloseAdd}
          returnFocus={returnFocus}
        />
      )}
      {removing !== null && (
        <RemoveDialog
          target={removing}
          lastFactor={count === 1 && !totp}
          totp={totp}
          onDone={onRemoved}
          onClose={onCloseRemove}
          returnFocus={returnFocus}
        />
      )}
    </>
  );
}

// eslint-disable-next-line max-lines-per-function -- the card: list, hints, add button and the two dialogs.
export function SecurityKeysCard({
  refreshKey,
  onChanged,
}: {
  /** The key count of the overview: the list reloads when it changes. */
  refreshKey: number;
  onChanged: () => void;
}) {
  const { t } = useI18n();
  const toast = useToast();
  const { keys, error, reload } = useKeys(refreshKey);
  const [adding, setAdding] = useState(false);
  const [removing, setRemoving] = useState<WebauthnKeyView | null>(null);
  const addButton = useRef<HTMLButtonElement>(null);
  const list = keys?.keys ?? [];
  const totp = keys?.totp ?? false;
  const changed = (message: string) => {
    toast(message);
    reload();
    onChanged();
  };
  const back = () => addButton.current;

  return (
    <div className="stack">
      <h3 className="subtitle">{t("settings.sync.key.title")}</h3>
      <p className="caption">{t("settings.sync.key.body")}</p>
      <ErrorAlert error={error} />
      {keys !== null && list.length === 0 && (
        <p className="body" role="status">
          {t("settings.sync.key.none")}
        </p>
      )}
      {list.length > 0 && (
        <ul className="rows" aria-label={t("settings.sync.key.title")}>
          {list.map((entry) => (
            <KeyRow
              key={entry.credentialRef}
              entry={entry}
              onRemove={() => {
                setRemoving(entry);
              }}
            />
          ))}
        </ul>
      )}
      {list.length === 1 && !totp && (
        <Callout tone="warning">
          <p className="body">{t("settings.sync.key.recommend")}</p>
        </Callout>
      )}
      <div className="actions">
        <Button
          ref={addButton}
          disabled={keys === null}
          onClick={() => {
            setAdding(true);
          }}
        >
          {t("settings.sync.key.add")}
        </Button>
      </div>
      <KeyDialogs
        adding={adding}
        removing={removing}
        totp={totp}
        count={list.length}
        onAdded={() => {
          changed(t("settings.sync.key.addedToast"));
        }}
        onRemoved={() => {
          changed(t("settings.sync.key.removedToast"));
        }}
        onCloseAdd={() => {
          setAdding(false);
        }}
        onCloseRemove={() => {
          setRemoving(null);
        }}
        returnFocus={back}
      />
    </div>
  );
}
