//! Tests of the sharing commands (M6b T5) against the real server on
//! `127.0.0.1` with two accounts: look up and verify a contact, invite, the
//! checks before joining, roles, removal with its native question, the
//! refusal to leave the server while a vault is shared, and emergency access
//! with its fresh login, wait and refusal. The folder and locked cases need no
//! server.
#![expect(clippy::too_many_lines, reason = "the scenarios are long")]

use crate::contract::errors::{ErrorCode, IpcResult};
use crate::contract::secret::SecretIn;
use crate::contract::sharing::{
    ContactCodeRequest, ContactStatus, EmergencyDecision, EmergencyGrantRequest,
    EmergencyRequestRequest, EmergencyRespondRequest, EmergencyScopeView, EmergencyState,
    InviteAcceptRequest, InviteDeclineRequest, InviteRequest, LookupRequest, MemberRequest,
    MemberRowState, SetRoleRequest, ShareRole, ShareVaultRequest,
};
use crate::contract::vaults::VaultRequest;
use crate::sync_handler_tests::loopback::LoopbackServer;
use crate::sync_handler_tests::{Env, PASSWORD, wait_for};
use crate::test_support::kdf_permit;

const ALEX: &str = "alex@example.com";
const SAM: &str = "sam@example.com";
const WRONG_CODE: &str = "AAAAA AAAAA AAAAA AAAAA AAAAAA";

fn code<T>(result: IpcResult<T>) -> ErrorCode {
    result
        .err()
        .map_or_else(|| panic!("expected an error"), |error| error.code)
}

/// Alex and Sam, both with a sync account on `server` and a finished first run
/// (which publishes each one's identity key).
fn pair(server: &LoopbackServer) -> (Env, Env) {
    let alex = Env::new();
    alex.create_as(ALEX);
    alex.connect_hosted(server);
    alex.wait_synced();
    let sam = Env::new();
    sam.create_as(SAM);
    sam.connect_hosted(server);
    sam.wait_synced();
    (alex, sam)
}

#[test]
#[expect(
    clippy::cognitive_complexity,
    reason = "one scenario from the first look-up to the removal"
)]
fn a_vault_is_shared_joined_changed_and_a_member_removed() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let (alex, sam) = pair(&server);

    let own = alex.b().sharing_identity().unwrap();
    assert!(own.published && own.safety_code.is_some());

    let vault = alex.b().list_vaults().unwrap()[0].vault_id;
    let found = alex
        .b()
        .sharing_lookup(&LookupRequest { email: SAM.into() })
        .unwrap();
    assert_eq!(found.status, ContactStatus::Pinned);
    let share = |role| ShareVaultRequest {
        vault_id: vault,
        account_id: found.account_id,
        role,
    };

    // Verification before sharing is required: nothing is sealed before the
    // user compared the code, and a code that is not the pinned key's is refused.
    assert_eq!(
        code(alex.b().vault_share(&share(ShareRole::Read))),
        ErrorCode::SharingContactUnverified
    );
    assert_eq!(
        code(alex.b().sharing_verify_contact(&ContactCodeRequest {
            account_id: found.account_id,
            safety_code: WRONG_CODE.into(),
        })),
        ErrorCode::SharingKeyChanged
    );
    alex.b()
        .sharing_verify_contact(&ContactCodeRequest {
            account_id: found.account_id,
            safety_code: found.safety_code.clone(),
        })
        .unwrap();
    let contacts = alex.b().sharing_contacts().unwrap().contacts;
    assert!(
        contacts
            .iter()
            .any(|c| c.email == SAM && c.verified && !c.changed)
    );

    assert_eq!(
        code(alex.b().vault_share(&share(ShareRole::Owner))),
        ErrorCode::InvalidInput
    );
    let sent = alex.b().vault_share(&share(ShareRole::Write)).unwrap();
    assert!(sent.expires_at > 0);
    assert_eq!(
        code(alex.b().vault_share(&share(ShareRole::Write))),
        ErrorCode::SharingAlreadyMember
    );
    let outgoing = alex.b().invites_list().unwrap().outgoing;
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].recipient_email.as_deref(), Some(SAM));

    // A vault that is shared cannot leave the server (CRYPTO_SPEC §10.2); nothing
    // is signed out first.
    assert_eq!(
        code(alex.b().sync_disconnect()),
        ErrorCode::SyncHasSharedVaults
    );
    assert!(alex.status().config.is_some());

    // Sam: the invite is opened and checked, then joined after the code.
    let incoming = sam.b().invites_list().unwrap().incoming;
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].sender_email, ALEX);
    assert_eq!(incoming[0].role, ShareRole::Write);
    let invite = incoming[0].invite_id;
    let preview = sam
        .b()
        .invite_preview(&InviteRequest { invite_id: invite })
        .unwrap();
    assert_eq!(preview.vault_name.as_deref(), Some("Personal"));
    assert_eq!(preview.sender_status, ContactStatus::Pinned);
    assert_eq!(preview.owner_account_id, preview.sender_account_id);
    let accept = |safety_code: Option<&str>| InviteAcceptRequest {
        invite_id: invite,
        safety_code: safety_code.map(str::to_owned),
    };
    assert_eq!(
        code(sam.b().invite_accept(&accept(None))),
        ErrorCode::SharingContactUnverified
    );
    assert_eq!(
        code(sam.b().invite_accept(&accept(Some(WRONG_CODE)))),
        ErrorCode::SharingKeyChanged
    );
    sam.b()
        .invite_accept(&accept(Some(&preview.safety_code)))
        .unwrap();
    sam.b().sync_now().unwrap();
    wait_for("the shared vault on Sam's device", || {
        sam.b().list_vaults().is_ok_and(|v| v.len() == 2)
    });
    assert_eq!(sam.b().invites_list().unwrap().incoming.len(), 0);

    // The owner sees Sam as a verified member and changes the role.
    let view = alex
        .b()
        .vault_members(&VaultRequest { vault_id: vault })
        .unwrap();
    assert_eq!(view.own_role, ShareRole::Owner);
    let member = view
        .members
        .iter()
        .find(|m| m.email.as_deref() == Some(SAM))
        .unwrap();
    assert_eq!(member.state, MemberRowState::Member);
    assert!(member.verified);
    alex.b()
        .vault_member_set_role(&SetRoleRequest {
            vault_id: vault,
            account_id: found.account_id,
            role: ShareRole::Read,
        })
        .unwrap();
    assert_eq!(
        code(alex.b().vault_member_set_role(&SetRoleRequest {
            vault_id: vault,
            account_id: found.account_id,
            role: ShareRole::Owner,
        })),
        ErrorCode::InvalidInput
    );

    // Removing asks in a native dialog (the sentence of CRYPTO_SPEC §8.3); "no" changes nothing.
    let remove = MemberRequest {
        vault_id: vault,
        account_id: found.account_id,
    };
    alex.answer(false);
    assert_eq!(
        code(alex.b().vault_member_remove(&remove)),
        ErrorCode::Cancelled
    );
    assert!(
        alex.b()
            .vault_members(&VaultRequest { vault_id: vault })
            .unwrap()
            .members
            .iter()
            .any(|m| m.email.as_deref() == Some(SAM))
    );
    alex.answer(true);
    let removed = alex.b().vault_member_remove(&remove).unwrap();
    assert_eq!(removed.unreachable_attachments, 0);
    let question = alex.asked().pop().unwrap();
    // The language is the setting; the system language decides here.
    assert!(
        question.contains("keeps whatever they already downloaded")
            || question.contains("behält, was schon heruntergeladen wurde"),
        "{question}"
    );
    assert!(question.contains(SAM), "{question}");
    let after = alex
        .b()
        .vault_members(&VaultRequest { vault_id: vault })
        .unwrap();
    assert!(
        after
            .members
            .iter()
            .all(|m| m.email.as_deref() != Some(SAM))
    );
    drop(server);
}

#[test]
fn an_invite_can_be_declined_and_withdrawn() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let (alex, sam) = pair(&server);
    let vault = alex.b().list_vaults().unwrap()[0].vault_id;
    let found = alex
        .b()
        .sharing_lookup(&LookupRequest { email: SAM.into() })
        .unwrap();
    alex.b()
        .sharing_verify_contact(&ContactCodeRequest {
            account_id: found.account_id,
            safety_code: found.safety_code.clone(),
        })
        .unwrap();
    let share = ShareVaultRequest {
        vault_id: vault,
        account_id: found.account_id,
        role: ShareRole::Read,
    };
    alex.b().vault_share(&share).unwrap();
    let invite = sam.b().invites_list().unwrap().incoming[0].invite_id;
    sam.b()
        .invite_decline(&InviteDeclineRequest {
            invite_id: invite,
            block: false,
        })
        .unwrap();
    assert_eq!(sam.b().invites_list().unwrap().incoming.len(), 0);
    assert_eq!(
        code(sam.b().invite_preview(&InviteRequest { invite_id: invite })),
        ErrorCode::SharingInviteExpired
    );
    drop(server);
}

#[test]
fn emergency_access_is_granted_with_the_password_requested_and_refused() {
    let _kdf = kdf_permit();
    let server = LoopbackServer::start();
    let (alex, sam) = pair(&server);
    let found = alex
        .b()
        .sharing_lookup(&LookupRequest { email: SAM.into() })
        .unwrap();
    let grant = |wait_days: u8, password: &str| EmergencyGrantRequest {
        account_id: found.account_id,
        scope: EmergencyScopeView::All,
        wait_days,
        password: SecretIn::test(password),
        code: None,
    };
    // The contact has to be verified first; the wait has to be one of the choices.
    assert_eq!(
        code(alex.b().emergency_grant(&grant(1, PASSWORD))),
        ErrorCode::SharingContactUnverified
    );
    alex.b()
        .sharing_verify_contact(&ContactCodeRequest {
            account_id: found.account_id,
            safety_code: found.safety_code.clone(),
        })
        .unwrap();
    assert_eq!(
        code(alex.b().emergency_grant(&grant(3, PASSWORD))),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(alex.b().emergency_grant(&grant(1, "not the password"))),
        ErrorCode::WrongCredentials
    );
    alex.b().emergency_grant(&grant(1, PASSWORD)).unwrap();
    alex.wait_synced();

    let overview = alex.b().emergency_overview().unwrap();
    assert_eq!(overview.granted.len(), 1);
    assert_eq!(overview.granted[0].state, EmergencyState::Idle);
    assert_eq!(overview.granted[0].wait_days, 1);
    assert_eq!(overview.granted[0].scope, EmergencyScopeView::All);

    // Sam asks; the wait runs on the server.
    let received = sam.b().emergency_overview().unwrap().received;
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].email, ALEX);
    let grantor = received[0].grantor_account_id;
    let requested = sam
        .b()
        .emergency_request(&EmergencyRequestRequest {
            grantor_account_id: grantor,
        })
        .unwrap();
    assert!(requested.release_at > 0);
    assert_eq!(
        code(
            sam.b()
                .emergency_claim(&crate::contract::sharing::EmergencyClaimRequest {
                    grantor_account_id: grantor,
                })
        ),
        ErrorCode::EmergencyWaiting
    );
    wait_for("the request on Alex's overview", || {
        alex.b()
            .emergency_overview()
            .is_ok_and(|o| o.granted[0].state == EmergencyState::Waiting)
    });

    // Approving needs the password; refusing does not.
    let respond = |decision, password: Option<&str>| EmergencyRespondRequest {
        grantee_account_id: found.account_id,
        decision,
        password: password.map(SecretIn::test),
        code: None,
    };
    assert_eq!(
        code(
            alex.b()
                .emergency_respond(&respond(EmergencyDecision::Approve, None))
        ),
        ErrorCode::EmptyPassword
    );
    alex.b()
        .emergency_respond(&respond(EmergencyDecision::Decline, None))
        .unwrap();
    assert_eq!(
        alex.b().emergency_overview().unwrap().granted[0].state,
        EmergencyState::Idle
    );
    // A refusal starts the cooldown.
    assert_eq!(
        code(sam.b().emergency_request(&EmergencyRequestRequest {
            grantor_account_id: grantor,
        })),
        ErrorCode::EmergencyCooldown
    );
    // Revoking removes the grant.
    alex.b()
        .emergency_revoke(&crate::contract::sharing::EmergencyRevokeRequest {
            account_id: found.account_id,
        })
        .unwrap();
    assert_eq!(alex.b().emergency_overview().unwrap().granted.len(), 0);
    drop(server);
}

#[test]
fn sharing_needs_the_server_and_the_unlocked_app() {
    let _kdf = kdf_permit();
    let locked = Env::new();
    assert_eq!(code(locked.b().sharing_identity()), ErrorCode::Locked);
    assert_eq!(code(locked.b().invites_list()), ErrorCode::Locked);
    assert_eq!(code(locked.b().sharing_contacts()), ErrorCode::Locked);

    // No sync, and a sync folder: nothing to share with.
    let env = Env::new();
    env.create();
    let none = |env: &Env| {
        assert_eq!(
            code(env.b().sharing_identity()),
            ErrorCode::SharingNotHosted
        );
        assert_eq!(code(env.b().invites_list()), ErrorCode::SharingNotHosted);
        assert_eq!(
            code(env.b().emergency_overview()),
            ErrorCode::SharingNotHosted
        );
        assert_eq!(
            code(env.b().sharing_lookup(&LookupRequest { email: SAM.into() })),
            ErrorCode::SharingNotHosted
        );
        let vault = env.b().list_vaults().unwrap()[0].vault_id;
        assert_eq!(
            code(env.b().vault_members(&VaultRequest { vault_id: vault })),
            ErrorCode::SharingNotHosted
        );
        assert_eq!(
            code(env.b().vault_leave(&VaultRequest { vault_id: vault })),
            ErrorCode::SharingNotHosted
        );
    };
    none(&env);
    let dir = tempfile::tempdir().unwrap();
    env.connect_folder(dir.path());
    none(&env);
    // Nothing the commands refuse reaches the sharing findings of the overview.
    assert_eq!(env.status().sharing_findings.len(), 0);
}
