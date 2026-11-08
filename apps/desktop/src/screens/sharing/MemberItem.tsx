/** One row of the members list: who, in what role, in what state, and what can be done. */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Chip } from "../../components/Chip.tsx";
import { Select } from "../../components/Select.tsx";
import { UserText } from "../../components/UserText.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { MemberRow, OutgoingInviteView, ShareRole } from "../../ipc/contract/index.ts";
import { isolate } from "../sync/hooks.ts";
import { useFormatWhen, useRoleLabel } from "./hooks.ts";

const ASSIGNABLE: readonly Exclude<ShareRole, "owner">[] = ["manage", "write", "read"];

export interface MemberHandlers {
  onRole: (role: Exclude<ShareRole, "owner">) => void;
  onRemove: () => void;
  onCancelInvite: (invite: OutgoingInviteView) => void;
  onVerify: (email: string) => void;
}

/** The roles, states and markers of a member. */
function MemberMarkers({ row, invite }: { row: MemberRow; invite: OutgoingInviteView | undefined }) {
  const { t } = useI18n();
  const roleLabel = useRoleLabel();
  const formatWhen = useFormatWhen();
  const flagged: Partial<Record<MemberRow["state"], string>> = {
    removing: t("sharing.members.removing"),
    unconfirmed: t("sharing.members.unconfirmed"),
    dropped: t("sharing.members.dropped"),
  };
  const warning = flagged[row.state];
  return (
    <span className="row__meta">
      {row.self && <Chip>{t("sharing.members.you")}</Chip>}
      <Chip>{roleLabel(row.role)}</Chip>
      {row.state === "invited" && <Chip>{t("sharing.members.invited")}</Chip>}
      {warning !== undefined && <Chip tone="warning">{warning}</Chip>}
      {!row.verified && row.state !== "unconfirmed" && (
        <Chip tone="warning">{t("sharing.members.unverified")}</Chip>
      )}
      {invite !== undefined && <span>{t("sharing.invite.expires", { when: formatWhen(invite.expiresAt) })}</span>}
    </span>
  );
}

/**
 * The role of a member: a choice and an explicit "Change role". Moving through the choices with
 * the arrow keys changes nothing, so a role never changes by accident.
 */
function RoleSelect({
  row,
  email,
  busy,
  onRole,
}: {
  row: MemberRow;
  email: string;
  busy: boolean;
  onRole: MemberHandlers["onRole"];
}) {
  const { t } = useI18n();
  const roleLabel = useRoleLabel();
  const [chosen, setChosen] = useState<Exclude<ShareRole, "owner">>(() =>
    row.role === "owner" ? "read" : row.role,
  );
  return (
    <div className="actions">
      <Select
        label={t("sharing.members.roleOf", { email })}
        value={chosen}
        disabled={busy}
        options={ASSIGNABLE.map((each) => ({ value: each, label: roleLabel(each) }))}
        onChange={(event) => {
          setChosen(ASSIGNABLE.find((each) => each === event.target.value) ?? chosen);
        }}
      />
      <Button
        size="sm"
        disabled={busy || chosen === row.role}
        aria-label={t("sharing.members.changeRoleOf", { email })}
        onClick={() => {
          onRole(chosen);
        }}
      >
        {t("sharing.members.changeRole")}
      </Button>
    </div>
  );
}

/** What this account may do with a member: change the role, verify, withdraw the invite, remove. */
function allowedActions(row: MemberRow, manager: boolean, invite: OutgoingInviteView | undefined) {
  const settled = row.state === "member" || row.state === "unconfirmed" || row.state === "dropped";
  // Nothing changes for the own row: no role change, and leaving is the vault's own button.
  const notOwner = manager && row.role !== "owner" && !row.self;
  return {
    role: notOwner && row.state === "member",
    verify: !row.verified && row.email !== null && row.state !== "removing" && !row.self,
    withdraw: manager && invite !== undefined,
    remove: notOwner && settled,
  };
}

function MemberActions({
  row,
  email,
  manager,
  invite,
  busy,
  handlers,
}: {
  row: MemberRow;
  email: string;
  manager: boolean;
  invite: OutgoingInviteView | undefined;
  busy: boolean;
  handlers: MemberHandlers;
}) {
  const { t } = useI18n();
  const allowed = allowedActions(row, manager, invite);
  return (
    <div className="actions">
      {allowed.role && <RoleSelect row={row} email={email} busy={busy} onRole={handlers.onRole} />}
      {allowed.verify && (
        <Button
          size="sm"
          aria-label={t("sharing.members.verifyOf", { email })}
          onClick={() => {
            handlers.onVerify(row.email ?? "");
          }}
        >
          {t("sharing.members.verify")}
        </Button>
      )}
      {allowed.withdraw && invite !== undefined && (
        <Button
          size="sm"
          variant="ghost"
          disabled={busy}
          aria-label={t("sharing.invite.cancelOf", { email })}
          onClick={() => {
            handlers.onCancelInvite(invite);
          }}
        >
          {t("sharing.invite.cancel")}
        </Button>
      )}
      {allowed.remove && (
        <Button
          size="sm"
          variant="danger"
          disabled={busy}
          aria-label={t("sharing.members.removeOf", { email })}
          onClick={handlers.onRemove}
        >
          {t("sharing.members.remove")}
        </Button>
      )}
    </div>
  );
}

export function MemberItem({
  row,
  own,
  invite,
  busy,
  handlers,
}: {
  row: MemberRow;
  own: ShareRole;
  invite: OutgoingInviteView | undefined;
  busy: boolean;
  handlers: MemberHandlers;
}) {
  const { t } = useI18n();
  const email = row.email ?? t("sharing.members.unknownEmail");
  const manager = own === "owner" || own === "manage";
  return (
    <li className="row">
      <div className="row__main">
        <span className="row__value">
          <UserText>{email}</UserText>
        </span>
        <MemberMarkers row={row} invite={invite} />
        {row.state === "unconfirmed" && <p className="field__hint">{t("sharing.members.unconfirmed.help")}</p>}
        {row.state === "dropped" && <p className="field__hint">{t("sharing.members.dropped.help")}</p>}
      </div>
      <MemberActions row={row} email={isolate(email)} manager={manager} invite={invite} busy={busy} handlers={handlers} />
    </li>
  );
}
