/** E02 with E04, E05 and E07: what Mosaic can do on the front tab. */
import { useEffect, useMemo, useState } from "react";
import type { ErrorCode } from "../../shared/errors.ts";
import type { PopupState } from "../../shared/messages.ts";
import type { CopySource, LoginMatch } from "../../shared/protocol.ts";
import { useFrameGate } from "../components/FrameGate.tsx";
import { Icon } from "../components/Icon.tsx";
import { ItemRow, type RowItem } from "../components/ItemRow.tsx";
import { OtpCard } from "../components/OtpCard.tsx";
import { SaveCard } from "../components/SaveCard.tsx";
import { Search } from "../components/Search.tsx";
import { Warnings } from "../components/Warnings.tsx";
import { useI18n } from "../i18n.tsx";
import { useActions, type Actions } from "../useActions.ts";
import { useShortcuts } from "../useShortcuts.ts";
import { useServices } from "../usePopup.ts";

function toRow(match: LoginMatch): RowItem {
  return {
    vaultId: match.vaultId,
    itemId: match.itemId,
    title: match.title,
    username: match.username,
    hasOtp: match.hasOtp,
    fillable: true,
    blocked: match.blocked !== null,
    sameSite: match.matchedBy === "etld1",
  };
}

function useFillShortcut(): string | null {
  const { env } = useServices();
  const [shortcut, setShortcut] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    void env.fillShortcut().then((key) => {
      if (current) {
        setShortcut(key);
      }
    });
    return () => {
      current = false;
    };
  }, [env]);
  return shortcut;
}

/** A failed shortcut or menu fill, shown once when the popup opens and kept until it closes. */
function useShownError(error: ErrorCode | null): ErrorCode | null {
  const [shown, setShown] = useState<ErrorCode | null>(error);
  if (error !== null && error !== shown) {
    setShown(error);
  }
  return shown;
}

type Gate = ReturnType<typeof useFrameGate>;

/** Everything that needs the user's attention before the list: errors, address warnings, a pending save, a code. */
function Notices({ state, gate, actions }: { state: PopupState; gate: Gate; actions: Actions }) {
  const { t, errorText } = useI18n();
  const shownError = useShownError(state.lastError);
  const next = state.nextOtp;
  return (
    <>
      {shownError !== null && (
        <p className="alert" role="alert">
          {errorText(shownError)}
        </p>
      )}
      {state.tab !== null && <Warnings warnings={state.warnings} host={state.tab.host} unicodeHost={state.tab.unicodeHost} />}
      {gate.blocked && (
        <p className="callout callout--info">
          <Icon name="info" />
          <span>{t("ext.frame.cross")}</span>
        </p>
      )}
      {state.pendingSave !== null && <SaveCard pending={state.pendingSave} />}
      {next !== null && (
        <OtpCard
          next={next}
          fillOff={gate.blocked}
          onFill={() => {
            gate.gate((confirm) => actions.fillOtp(next, confirm));
          }}
          onCopy={() => void actions.copy({ kind: "item", vaultId: next.vaultId, itemId: next.itemId, field: "totp" })}
        />
      )}
    </>
  );
}

function SiteLogins({
  tab,
  rows,
  fillOff,
  quiet,
  onFill,
  onCopy,
}: {
  tab: NonNullable<PopupState["tab"]>;
  rows: readonly RowItem[];
  fillOff: boolean;
  /** A save or code card holds the screen's one primary button, so no row gets the holo one. */
  quiet: boolean;
  onFill: (item: RowItem) => void;
  onCopy: (source: CopySource) => void;
}) {
  const { t } = useI18n();
  const shortcut = useFillShortcut();
  return (
    <section aria-labelledby="for-site" className="section">
      <div className="section__head">
        <h2 id="for-site" className="title">
          {t("ext.logins.forSite", { host: tab.unicodeHost ?? tab.host })}
        </h2>
        {rows.length > 0 && <span className="caption">{t("ext.logins.count", { count: rows.length })}</span>}
      </div>
      {rows.length === 0 ? (
        <>
          <p className="muted">{t("ext.logins.none", { host: tab.host })}</p>
          <p className="caption">{t("ext.logins.noneHint")}</p>
        </>
      ) : (
        <ul className="items">
          {rows.map((row, index) => (
            <ItemRow
              key={`${row.vaultId}/${row.itemId}`}
              item={row}
              primary={index === 0 && !row.blocked && !quiet}
              fillOff={fillOff}
              onFill={() => {
                onFill(row);
              }}
              onCopy={onCopy}
            />
          ))}
        </ul>
      )}
      {shortcut !== null && rows.length > 0 && <p className="caption">{t("ext.logins.shortcut", { shortcut })}</p>}
    </section>
  );
}

export function LoginsView({ state }: { state: PopupState }) {
  const { t } = useI18n();
  const actions = useActions();
  const gate = useFrameGate(state);
  const tab = state.tab;
  const rows = useMemo(() => state.matches.map(toRow), [state.matches]);
  const fillRow = (item: Pick<RowItem, "vaultId" | "itemId">) => {
    gate.gate((confirm) => actions.fill({ vaultId: item.vaultId, itemId: item.itemId }, confirm));
  };
  const copy = (source: CopySource) => void actions.copy(source);
  useShortcuts({ items: rows.filter((row) => !row.blocked), fill: fillRow, copy: actions.copy, enabled: gate.dialog === null });

  return (
    <>
      {tab === null ? (
        <h1 className="muted heading-plain">{t("ext.logins.unsupported")}</h1>
      ) : (
        <h1 className="visually-hidden">{t("ext.nav.logins")}</h1>
      )}
      <Notices state={state} gate={gate} actions={actions} />
      {tab !== null && (
        <SiteLogins
          tab={tab}
          rows={rows}
          fillOff={gate.blocked}
          quiet={state.pendingSave !== null || state.nextOtp !== null}
          onFill={fillRow}
          onCopy={copy}
        />
      )}
      <Search fillOff={gate.blocked} onFill={fillRow} onCopy={copy} />
      {tab !== null && <p className="caption note">{t("ext.logins.frameNote", { host: tab.host })}</p>}
      {gate.dialog}
    </>
  );
}
