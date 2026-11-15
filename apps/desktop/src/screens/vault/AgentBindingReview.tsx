/**
 * The agent access of an item in the detail pane (ADR-0159, builder check B1): each binding with
 * its sites and how the value is sent. A binding that arrived by sync, or changed on another
 * device, is not usable here until the user confirms it: the banner shows what changed against
 * the last confirmed state and a plain "Confirm for this device" button. Nothing here shows a
 * value.
 */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n, type I18n } from "../../i18n/I18nProvider.tsx";
import type {
  AgentBindingView,
  IpcError,
  ItemDetail as Detail,
  ItemRequest,
} from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";

/** The label of the bound field: the field's name, or the custom field's label. */
function fieldLabel(binding: AgentBindingView, detail: Detail, t: I18n["t"]): string {
  const { field } = binding;
  if (field.kind === "field") {
    return t(`item.field.${field.field}`);
  }
  return field.kind === "customField"
    ? (detail.customFields.find((each) => each.elementId === field.elementId)?.label ?? field.elementId)
    : "";
}

/** What changed against the last confirmed state, in sentences. */
function Difference({ binding }: { binding: AgentBindingView }) {
  const { t } = useI18n();
  const last = binding.lastConfirmed;
  if (last === null) {
    return <p>{t("agent.review.never")}</p>;
  }
  const added = binding.origins.filter((site) => !last.origins.includes(site));
  const removed = last.origins.filter((site) => !binding.origins.includes(site));
  return (
    <>
      {added.length === 0 && removed.length === 0 && <p>{t("agent.review.same")}</p>}
      {added.length > 0 && <p translate="no">{t("agent.review.added", { sites: added.join(", ") })}</p>}
      {removed.length > 0 && <p translate="no">{t("agent.review.removed", { sites: removed.join(", ") })}</p>}
      {last.inject !== binding.inject.kind && (
        <p>
          {t("agent.review.injectChanged", {
            before: t(`agent.editor.inject.${last.inject}`),
            after: t(`agent.editor.inject.${binding.inject.kind}`),
          })}
        </p>
      )}
    </>
  );
}

function Review({
  binding,
  item,
  onDetail,
}: {
  binding: AgentBindingView;
  item: ItemRequest;
  onDetail: (detail: Detail) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [error, setError] = useState<IpcError | null>(null);
  return (
    <div className="agent-review" role="group" aria-label={t("agent.review.title")}>
      <p>
        <strong>{t("agent.review.title")}</strong>
      </p>
      <Difference binding={binding} />
      <ErrorAlert error={error} />
      <div>
        <Button
          onClick={() => {
            void ipc.call("agent_binding_confirm", { ...item, elementId: binding.elementId }).then((result) => {
              if (result.ok) {
                setError(null);
                onDetail(result.value);
              } else {
                setError(result.error);
              }
            });
          }}
        >
          {t("agent.review.confirm")}
        </Button>
      </div>
    </div>
  );
}

export function AgentBindingReview({
  detail,
  item,
  trash,
  onDetail,
}: {
  detail: Detail;
  item: ItemRequest;
  trash: boolean;
  onDetail: (detail: Detail) => void;
}) {
  const { t } = useI18n();
  if (detail.agentBindings.length === 0) {
    return null;
  }
  return (
    <section className="detail__agent" aria-label={t("agent.detail.title")}>
      <h3 className="row__label">{t("agent.detail.title")}</h3>
      <ul className="rows">
        {detail.agentBindings.map((binding) => (
          <li key={binding.elementId} className="row">
            <div className="row__main">
              <p>{fieldLabel(binding, detail, t)}</p>
              <p className="caption" translate="no">
                {binding.origins.join(", ")}
              </p>
              <p className="caption">{t(`agent.access.inject.${binding.inject.kind}`)}</p>
              {binding.confirmed && binding.usable && <p className="caption">{t("agent.detail.usable")}</p>}
              {!binding.confirmed && !trash && <Review binding={binding} item={item} onDetail={onDetail} />}
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
