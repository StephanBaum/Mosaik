/**
 * The Security page (D13) with its two views: the dashboard ("Overview") and the rotate-now
 * queue ("Rotate now"). The dashboard's "Change password" opens the queue's guided panel for
 * that item. The queue is read once here, so the tab can show how many passwords are due.
 */
import { useId, useState, type KeyboardEvent } from "react";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { ItemRequest } from "../../ipc/contract/index.ts";
import { SecurityDashboard } from "../SecurityDashboard.tsx";
import { RotationQueue } from "./RotationQueue.tsx";
import { dueCount, useRotationQueue } from "./useRotationQueue.ts";

type Tab = "overview" | "rotate";
const TABS: readonly Tab[] = ["overview", "rotate"];

export function SecurityPage({
  onOpenItem,
  initialTab = "overview",
}: {
  onOpenItem: (item: ItemRequest) => void;
  initialTab?: Tab;
}) {
  const { t } = useI18n();
  const id = useId();
  const [tab, setTab] = useState<Tab>(initialTab);
  const [openFor, setOpenFor] = useState<ItemRequest | null>(null);
  const [round, setRound] = useState(0);
  const state = useRotationQueue();
  const due = dueCount(state.queue);

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
      event.preventDefault();
      const next = TABS[(TABS.indexOf(tab) + 1) % TABS.length] ?? "overview";
      setTab(next);
      document.getElementById(`${id}-${next}`)?.focus();
    }
  }

  return (
    <div className="security-page">
      <div className="segmented" role="tablist" aria-label={t("credentialRotation.tablist")} onKeyDown={onKeyDown}>
        {TABS.map((each) => (
          <button
            key={each}
            id={`${id}-${each}`}
            type="button"
            role="tab"
            className="segmented__tab"
            aria-selected={each === tab}
            aria-controls={`${id}-panel`}
            aria-label={
              each === "rotate" && due > 0 ? t("credentialRotation.tab.rotateCount", { count: due }) : undefined
            }
            tabIndex={each === tab ? 0 : -1}
            onClick={() => {
              setOpenFor(null);
              setTab(each);
            }}
          >
            {t(`credentialRotation.tab.${each}`)}
            {each === "rotate" && due > 0 && (
              <span className="tab-badge tabular" aria-hidden="true">
                {due}
              </span>
            )}
          </button>
        ))}
      </div>
      <div role="tabpanel" id={`${id}-panel`} aria-labelledby={`${id}-${tab}`}>
        {tab === "overview" ? (
          <SecurityDashboard
            onOpenItem={onOpenItem}
            refreshKey={round}
            breach={state.queue?.breach ?? null}
            onBreachChanged={state.reload}
            onChangePassword={(item) => {
              setOpenFor(item);
              setTab("rotate");
            }}
          />
        ) : (
          <RotationQueue
            state={state}
            onOpenItem={onOpenItem}
            openFor={openFor}
            onChanged={() => {
              setRound((n) => n + 1);
            }}
          />
        )}
      </div>
    </div>
  );
}
