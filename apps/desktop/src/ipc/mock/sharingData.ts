/**
 * The mock's sharing state and the `?sharing=<variant>` seeds (see the header of `mock.ts`).
 * The mock "server" knows three people (sam, kim, lee at example.com); any other address gets
 * a made-up key, like the real directory. Safety codes are fixed samples. The master password
 * of the grant and approve calls is the sample account's.
 */
import type {
  ContactView,
  GrantedView,
  IncomingInviteView,
  InvitePreviewView,
  MemberRow,
  OutgoingInviteView,
  ReceivedView,
  ShareRole,
  SharingFindingView,
} from "../contract/index.ts";
import { sampleAccount } from "./data.ts";
import type { MockState } from "./state.ts";

export const sharingVariants = [
  "basic",
  "empty",
  "invites",
  "emergency",
  "request",
  "findings",
  "keyChanged",
  "manager",
  "viewer",
  "rotation",
] as const;
export type SharingVariant = (typeof sharingVariants)[number];

export const MOCK_OWN_CODE = "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V";

const DAY = 86_400_000;
const WORK = "019a2b3c-4d5e-7010-8000-000000000002";
const PERSONAL = "019a2b3c-4d5e-7010-8000-000000000001";

export interface DirectoryEntry {
  accountId: string;
  /** The code the server serves for the address now. */
  code: string;
}

export const SAM = {
  accountId: "019a2b3c-4d5e-7030-8000-000000000002",
  email: "sam@example.com",
  code: "2B9MQ 7XK4D RT6W0 ZNCF8 V3HJ4P",
};
export const KIM = {
  accountId: "019a2b3c-4d5e-7030-8000-000000000003",
  email: "kim@example.com",
  code: "5HR3V 8WQ2N 6TXC1 D4MZ9 K7BJ0G",
};
export const LEE = {
  accountId: "019a2b3c-4d5e-7030-8000-000000000004",
  email: "lee@example.com",
  code: "9DQ6K 3MV7X 1WHT4 B8ZN2 C5RF0J",
};
const NEW_SAM_CODE = "8XW4N 1KT9B 6QMD3 V7CZ2 H5RJ0F";

/** A vault this account received through an invite or an emergency release. */
export interface MockSharing {
  ownCode: string | null;
  contacts: ContactView[];
  directory: Map<string, DirectoryEntry>;
  /** Members by vault; a vault without an entry is not shared. */
  members: Map<string, MemberRow[]>;
  /** This account's role by shared vault. */
  roles: Map<string, ShareRole>;
  pendingRotation: Set<string>;
  unprotected: number;
  /** What a removal could not protect (`vault_member_remove`). */
  removalUnreachable: number;
  incoming: IncomingInviteView[];
  previews: Map<string, InvitePreviewView & { vaultId: string }>;
  unreadable: Set<string>;
  outgoing: OutgoingInviteView[];
  granted: GrantedView[];
  received: ReceivedView[];
  findings: SharingFindingView[];
  /** Vaults this account left: the server refuses them now. */
  left: Set<string>;
  inviteCounter: number;
}

function row(
  account: { accountId: string; email: string },
  role: ShareRole,
  fields: Partial<MemberRow> = {},
): MemberRow {
  return {
    accountId: account.accountId,
    email: account.email,
    role,
    verified: true,
    self: account.accountId === sampleAccount.accountId,
    state: "member",
    origin: "invite",
    ...fields,
  };
}

const me = { accountId: sampleAccount.accountId, email: sampleAccount.email };

function seededMembers(variant: SharingVariant): Map<string, MemberRow[]> {
  if (variant === "empty") {
    return new Map();
  }
  const ownRole = variant === "manager" ? "manage" : variant === "viewer" ? "read" : "owner";
  const owner = ownRole === "owner" ? me : SAM;
  const rows: MemberRow[] = [
    row(owner, "owner", { origin: "owner" }),
    ...(ownRole === "owner" ? [] : [row(me, ownRole, { origin: "invite" })]),
    row(ownRole === "owner" ? SAM : KIM, "write"),
    row(ownRole === "owner" ? KIM : LEE, "read", ownRole === "owner" ? { verified: false, state: "invited", origin: null } : {}),
  ];
  if (variant === "rotation") {
    rows.push(row(LEE, "read", { state: "removing" }));
  }
  if (variant === "findings") {
    rows.push(
      row({ accountId: "019a2b3c-4d5e-7030-8000-000000000005", email: "" }, "read", {
        email: null,
        verified: false,
        state: "unconfirmed",
        origin: "emergency",
      }),
      row(LEE, "read", { state: "dropped", origin: null }),
    );
  }
  return new Map([[WORK, rows]]);
}

function seededRoles(variant: SharingVariant): Map<string, ShareRole> {
  if (variant === "empty") {
    return new Map();
  }
  return new Map([[WORK, variant === "manager" ? "manage" : variant === "viewer" ? "read" : "owner"]]);
}

interface PreviewSpec {
  vaultId: string;
  name: string | null;
  sender: typeof SAM;
  owner: typeof SAM;
  role: ShareRole;
  senderStatus: InvitePreviewView["senderStatus"];
  ownerStatus: InvitePreviewView["ownerStatus"];
}

function previewOf(spec: PreviewSpec): InvitePreviewView & { vaultId: string } {
  const { sender, owner } = spec;
  return {
    vaultId: spec.vaultId,
    vaultName: spec.name,
    role: spec.role,
    senderAccountId: sender.accountId,
    senderEmail: sender.email,
    safetyCode: sender.code,
    senderStatus: spec.senderStatus,
    ownerAccountId: owner.accountId,
    ownerEmail: owner.email,
    ownerCode: owner.code,
    ownerStatus: spec.ownerStatus,
  };
}

function seededInvites(variant: SharingVariant, now: number): Pick<MockSharing, "incoming" | "previews" | "unreadable"> {
  const incoming: IncomingInviteView[] = [];
  const previews = new Map<string, InvitePreviewView & { vaultId: string }>();
  const unreadable = new Set<string>();
  if (variant === "invites" || variant === "findings") {
    incoming.push(
      { inviteId: "019a2b3c-4d5e-7060-8000-000000000011", senderEmail: SAM.email, role: "write", expiresAt: now + 5 * DAY },
      { inviteId: "019a2b3c-4d5e-7060-8000-000000000012", senderEmail: KIM.email, role: "read", expiresAt: now + DAY },
      { inviteId: "019a2b3c-4d5e-7060-8000-000000000013", senderEmail: LEE.email, role: "manage", expiresAt: now + 2 * DAY },
    );
    // Sam owns the vault and has not been verified; Kim is a manager inviting to Sam's vault
    // (the owner has to be verified, too).
    previews.set(
      "019a2b3c-4d5e-7060-8000-000000000011",
      previewOf({
        vaultId: "019a2b3c-4d5e-7010-8000-000000000021",
        name: "Family",
        sender: SAM,
        owner: SAM,
        role: "write",
        senderStatus: "pinned",
        ownerStatus: "pinned",
      }),
    );
    previews.set(
      "019a2b3c-4d5e-7060-8000-000000000012",
      previewOf({
        vaultId: "019a2b3c-4d5e-7010-8000-000000000022",
        name: "Holiday plans",
        sender: KIM,
        owner: SAM,
        role: "read",
        senderStatus: "verified",
        ownerStatus: "pinned",
      }),
    );
    unreadable.add("019a2b3c-4d5e-7060-8000-000000000013");
  }
  return { incoming, previews, unreadable };
}

function seededGrants(variant: SharingVariant, now: number): Pick<MockSharing, "granted" | "received"> {
  if (variant === "request") {
    return {
      granted: [
        {
          granteeAccountId: SAM.accountId,
          email: SAM.email,
          waitDays: 7,
          scope: { kind: "all" },
          state: "waiting",
          requestedAt: now - 2 * DAY,
          releaseAt: now + 5 * DAY,
        },
      ],
      received: [],
    };
  }
  if (variant !== "emergency" && variant !== "findings") {
    return { granted: [], received: [] };
  }
  return {
    granted: [
      {
        granteeAccountId: SAM.accountId,
        email: SAM.email,
        waitDays: 7,
        scope: { kind: "all" },
        state: "idle",
        requestedAt: null,
        releaseAt: null,
      },
      {
        granteeAccountId: KIM.accountId,
        email: KIM.email,
        waitDays: 30,
        scope: { kind: "vaults", vaultIds: [PERSONAL] },
        state: "released",
        requestedAt: now - 40 * DAY,
        releaseAt: now - 10 * DAY,
      },
    ],
    received: [
      {
        grantorAccountId: LEE.accountId,
        email: LEE.email,
        waitDays: 2,
        state: "idle",
        requestedAt: null,
        releaseAt: null,
        cooldownUntil: null,
      },
      {
        grantorAccountId: SAM.accountId,
        email: SAM.email,
        waitDays: 14,
        state: "waiting",
        requestedAt: now - DAY,
        releaseAt: now + 13 * DAY,
        cooldownUntil: null,
      },
      {
        grantorAccountId: KIM.accountId,
        email: KIM.email,
        waitDays: 1,
        state: "released",
        requestedAt: now - 3 * DAY,
        releaseAt: now - 2 * DAY,
        cooldownUntil: null,
      },
    ],
  };
}

function seededFindings(variant: SharingVariant): SharingFindingView[] {
  if (variant !== "findings") {
    return [];
  }
  return [
    { kind: "identityMismatch" },
    { kind: "rosterInvalid", vaultName: "Work" },
    { kind: "shareUntrusted", vaultName: "Work", senderEmail: KIM.email },
    { kind: "memberUnverified", vaultId: WORK, vaultName: "Work", accountId: KIM.accountId, email: KIM.email },
    { kind: "memberKeyMismatch", vaultId: WORK, vaultName: "Work", accountId: SAM.accountId, email: SAM.email },
    { kind: "vaultHeld", vaultId: PERSONAL, vaultName: "Personal" },
    { kind: "memberDropped", vaultId: WORK, vaultName: "Work", accountId: LEE.accountId, email: LEE.email },
    { kind: "accessLost", vaultId: PERSONAL, vaultName: "Personal" },
  ];
}

export function seedSharing(variant: SharingVariant | undefined): MockSharing {
  const v: SharingVariant = variant ?? "basic";
  const now = Date.now();
  const directory = new Map<string, DirectoryEntry>(
    [SAM, KIM, LEE].map((p) => [p.email, { accountId: p.accountId, code: p.code }]),
  );
  if (v === "keyChanged") {
    directory.set(SAM.email, { accountId: SAM.accountId, code: NEW_SAM_CODE });
  }
  const pinned: ContactView[] =
    v === "empty"
      ? []
      : [
          { accountId: SAM.accountId, email: SAM.email, safetyCode: SAM.code, verified: v !== "invites", changed: false },
          { accountId: KIM.accountId, email: KIM.email, safetyCode: KIM.code, verified: v === "invites", changed: false },
        ];
  return {
    ownCode: MOCK_OWN_CODE,
    contacts: pinned,
    directory,
    members: seededMembers(v),
    roles: seededRoles(v),
    pendingRotation: new Set<string>(v === "rotation" ? [WORK] : []),
    unprotected: v === "rotation" ? 2 : 0,
    removalUnreachable: v === "rotation" ? 2 : 0,
    ...seededInvites(v, now),
    outgoing:
      v === "basic" || v === "rotation"
        ? [
            {
              inviteId: "019a2b3c-4d5e-7060-8000-000000000001",
              vaultId: WORK,
              vaultName: "Work",
              recipientEmail: KIM.email,
              role: "read",
              expiresAt: now + 5 * DAY,
            },
          ]
        : [],
    ...seededGrants(v, now),
    findings: seededFindings(v),
    left: new Set(),
    inviteCounter: 100,
  };
}

/** Whether the mock's backend is the Mosaic server (the only one that shares). */
export function hostedBackend(state: MockState): boolean {
  return state.sync.config?.kind === "hosted";
}

/** What the last event told the app (invites, requests that wait for this account). */
export function sharingCounts(state: MockState): { invites: number; emergencyRequests: number } {
  return {
    invites: state.sharing.incoming.length,
    emergencyRequests: state.sharing.granted.filter((g) => g.state === "waiting").length,
  };
}

/** Tells the app the sharing counts (after an unlock or a subscription). */
export function announceSharing(state: MockState): void {
  if (!state.unlocked || !hostedBackend(state)) {
    return;
  }
  const counts = sharingCounts(state);
  if (counts.invites === 0 && counts.emergencyRequests === 0) {
    return;
  }
  setTimeout(() => {
    state.emit({ type: "sharingChanged", ...counts });
  }, 0);
}

export function publishSharing(state: MockState): void {
  state.emit({ type: "sharingChanged", ...sharingCounts(state) });
}
