/**
 * "Agent access" under one secret field of the item editor (ADR-0159, D8): a switch that lets AI
 * agents use the value, the sites it may be sent to, and how it is sent. This is the only place
 * where destinations are added or changed: the approval dialogs have no such control. The
 * address check here only shows what Rust will store and warns about look-alikes; Rust decides.
 */
import { Button } from "../../../components/Button.tsx";
import { Callout } from "../../../components/Callout.tsx";
import { Icon } from "../../../components/icons.tsx";
import { IconButton } from "../../../components/IconButton.tsx";
import { Select, type Option } from "../../../components/Select.tsx";
import { Switch } from "../../../components/Switch.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import {
  agentBasicUserKinds,
  agentInjectKinds,
  type AgentBasicUserKind,
  type AgentInjectKind,
  type AgentInjectView,
} from "../../../ipc/contract/index.ts";
import { MAX_AGENT_ORIGINS, newAgentRow, viewSite, type AgentRow } from "../agentForm.ts";

/** The inject view a kind starts from. */
function injectOf(kind: AgentInjectKind): AgentInjectView {
  return {
    kind,
    headerName: kind === "header" ? "" : null,
    headerPrefix: kind === "header" ? "" : null,
    basicUser: kind === "basic" ? { kind: "literal", value: "" } : null,
    queryName: kind === "query" ? "" : null,
  };
}

function Site({
  index,
  value,
  onChange,
  onRemove,
}: {
  index: number;
  value: string;
  onChange: (value: string) => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();
  const { canonical, lookalike } = viewSite(value);
  const typed = value.trim() !== "";
  const hint =
    canonical !== null && canonical !== value.trim() ? t("agent.editor.canonical", { site: canonical }) : undefined;
  return (
    <div className="agent-editor__site">
      <TextField
        label={t("agent.editor.destination", { index })}
        type="url"
        autoComplete="off"
        placeholder={t("agent.editor.destinationPlaceholder")}
        value={value}
        hint={hint}
        error={typed && canonical === null ? t("agent.editor.badOrigin") : undefined}
        onChange={(event) => {
          onChange(event.target.value);
        }}
      />
      <IconButton
        icon="trash"
        label={t("agent.editor.removeDestination", { site: value.trim() === "" ? String(index) : value.trim() })}
        onClick={onRemove}
      />
      {lookalike && <Callout tone="warning">{t("agent.editor.lookalike")}</Callout>}
    </div>
  );
}

function Sites({ row, onChange }: { row: AgentRow; onChange: (row: AgentRow) => void }) {
  const { t } = useI18n();
  const patch = (origins: string[]) => {
    onChange({ ...row, origins });
  };
  return (
    <div className="agent-editor__sites">
      <h4 className="editor__group-heading">{t("agent.editor.destinations")}</h4>
      <p className="field__hint">{t("agent.editor.destinationsHint")}</p>
      {row.origins.map((site, at) => (
        <Site
          key={at}
          index={at + 1}
          value={site}
          onChange={(value) => {
            patch(row.origins.map((each, i) => (i === at ? value : each)));
          }}
          onRemove={() => {
            patch(row.origins.filter((_, i) => i !== at));
          }}
        />
      ))}
      <div>
        <Button
          disabled={row.origins.length >= MAX_AGENT_ORIGINS}
          onClick={() => {
            patch([...row.origins, ""]);
          }}
        >
          <Icon name="plus" />
          {t("agent.editor.addDestination")}
        </Button>
      </div>
    </div>
  );
}

/** The user name of a Basic binding: a fixed name, a text field of the item, or the value itself. */
function BasicUser({
  user,
  textFields,
  onChange,
}: {
  user: NonNullable<AgentInjectView["basicUser"]>;
  textFields: readonly Option[];
  onChange: (user: NonNullable<AgentInjectView["basicUser"]>) => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <Select
        label={t("agent.editor.basicUser")}
        value={user.kind}
        options={agentBasicUserKinds.map((kind) => ({ value: kind, label: t(`agent.editor.basicUserKind.${kind}`) }))}
        onChange={(event) => {
          const kind = event.target.value as AgentBasicUserKind;
          onChange({ kind, value: kind === "secretIsUser" ? null : (textFields[0]?.value ?? "") });
        }}
      />
      {user.kind === "literal" && (
        <TextField
          label={t("agent.editor.basicUserLiteral")}
          autoComplete="off"
          value={user.value ?? ""}
          onChange={(event) => {
            onChange({ kind: "literal", value: event.target.value });
          }}
        />
      )}
      {user.kind === "field" && (
        <Select
          label={t("agent.editor.basicUserField")}
          value={user.value ?? ""}
          options={textFields}
          onChange={(event) => {
            onChange({ kind: "field", value: event.target.value });
          }}
        />
      )}
    </>
  );
}

/** How the value is sent: the kind and what each kind needs. */
function How({
  row,
  textFields,
  onChange,
}: {
  row: AgentRow;
  textFields: readonly Option[];
  onChange: (row: AgentRow) => void;
}) {
  const { t } = useI18n();
  const { inject } = row;
  const patch = (change: Partial<AgentInjectView>) => {
    onChange({ ...row, inject: { ...inject, ...change } });
  };
  return (
    <>
      <Select
        label={t("agent.editor.how")}
        value={inject.kind}
        options={agentInjectKinds.map((kind) => ({ value: kind, label: t(`agent.editor.inject.${kind}`) }))}
        onChange={(event) => {
          onChange({ ...row, inject: injectOf(event.target.value as AgentInjectKind) });
        }}
      />
      {inject.kind === "header" && (
        <>
          <TextField
            label={t("agent.editor.headerName")}
            autoComplete="off"
            value={inject.headerName ?? ""}
            onChange={(event) => {
              patch({ headerName: event.target.value });
            }}
          />
          <TextField
            label={t("agent.editor.headerPrefix")}
            autoComplete="off"
            value={inject.headerPrefix ?? ""}
            onChange={(event) => {
              patch({ headerPrefix: event.target.value });
            }}
          />
        </>
      )}
      {inject.kind === "basic" && inject.basicUser !== null && (
        <BasicUser
          user={inject.basicUser}
          textFields={textFields}
          onChange={(basicUser) => {
            patch({ basicUser });
          }}
        />
      )}
      {inject.kind === "query" && (
        <>
          <TextField
            label={t("agent.editor.queryName")}
            autoComplete="off"
            value={inject.queryName ?? ""}
            onChange={(event) => {
              patch({ queryName: event.target.value });
            }}
          />
          <Callout tone="warning">{t("agent.editor.queryNote")}</Callout>
        </>
      )}
    </>
  );
}

export function AgentAccessEditor({
  label,
  row,
  textFields,
  onChange,
}: {
  /** The field's label, for the group name. */
  label: string;
  row: AgentRow | undefined;
  /** The item's text fields, for a Basic user name taken from a field. */
  textFields: readonly Option[];
  onChange: (row: AgentRow) => void;
}) {
  const { t } = useI18n();
  const on = row?.enabled === true;
  return (
    <fieldset className="agent-editor" aria-label={t("agent.editor.fieldLabel", { field: label })}>
      <legend className="editor__group-heading">{t("agent.editor.fieldLabel", { field: label })}</legend>
      <Switch
        label={t("agent.editor.switch")}
        hint={t("agent.editor.body")}
        checked={on}
        onChange={(event) => {
          onChange(event.target.checked ? { ...(row ?? newAgentRow()), enabled: true } : { ...(row ?? newAgentRow()), enabled: false });
        }}
      />
      {row !== undefined && on && (
        <>
          <Sites row={row} onChange={onChange} />
          <How row={row} textFields={textFields} onChange={onChange} />
          <TextField
            label={t("agent.editor.env")}
            hint={t("agent.editor.envHint")}
            autoComplete="off"
            value={row.env}
            onChange={(event) => {
              onChange({ ...row, env: event.target.value });
            }}
          />
        </>
      )}
    </fieldset>
  );
}
