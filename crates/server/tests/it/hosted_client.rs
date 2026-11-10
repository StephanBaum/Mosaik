//! The hosted client functions of `mosaic-sync` (account info, second factor,
//! device list, re-key refusals and the retry after a lost answer) against a
//! real server on loopback. Each test is also a coverage target of the sync
//! crate, whose hosted code only runs here.

use mosaic_core::{Argon2Profile, InstallId, SecretKey};
use mosaic_server::Clock;
use mosaic_sync::TransportError;
use mosaic_sync::http::hosted::{self, RekeyOutcome, RekeyRefusal, RekeyStage};
use mosaic_sync::testutil::ChaosConfig;
use mosaic_sync::transport::Rejection;
use mosaic_totp::{Algorithm, Digits, Otp, OtpKind, OtpSecret, Period};
use mosaic_vault::session::{PreparedRekey, RekeyPlan, RekeyRequest, RotationKind};

use crate::hosted::{PASSWORD, World, pw};

const NEW_PASSWORD: &str = "a different horse battery staple";

fn code_now(w: &World, secret_b32: &str) -> String {
    let otp = Otp {
        secret: OtpSecret::from_base32(secret_b32).unwrap(),
        algorithm: Algorithm::Sha1,
        kind: OtpKind::Totp {
            digits: Digits::DEFAULT,
            period: Period::DEFAULT,
        },
    };
    otp.code_at(w.server.clock.now_ms() / 1000)
        .unwrap()
        .expose_secret()
        .to_owned()
}

fn login(w: &World, device: InstallId, name: &str) -> hosted::LoginGrant {
    w.server.advance_ms(60_000);
    hosted::login(&w.server.base, &w.email, w.account, &w.opw(0), device, name).unwrap()
}

/// The prepared re-key of device 0 over a new vault with one pushed state;
/// `wrong_head` names a head the server does not have.
fn prepared(w: &mut World, wrong_head: bool) -> PreparedRekey {
    let vault = w.new_vault(0, "Own");
    w.sync(0);
    let head = {
        let s = w.devs[0].s();
        s.sync_cursor(vault)
            .unwrap()
            .and_then(|c| <[u8; 8]>::try_from(c).ok())
            .map_or(0, u64::from_be_bytes)
    };
    let current = SecretKey::parse(w.secret_key.to_display().unwrap().expose_secret()).unwrap();
    let new_sk = SecretKey::generate().unwrap();
    let mut s = w.devs[0].s_mut();
    s.begin_rotation_job(RotationKind::Rekey, None).unwrap();
    s.prepare_rekey(RekeyRequest {
        current_password: &pw(PASSWORD),
        current_secret_key: &current,
        new_password: &pw(NEW_PASSWORD),
        new_secret_key: &new_sk,
        profile: Argon2Profile::Baseline,
        plan: RekeyPlan {
            heads: vec![(vault, if wrong_head { head + 5 } else { head })],
            ..RekeyPlan::default()
        },
    })
    .unwrap()
}

#[test]
fn account_info_epoch_totp_and_devices() {
    let w = World::new(301, ChaosConfig::default());
    let device = w.devs[0].device;
    let grant = login(&w, device, "device 0");
    assert!(format!("{grant:?}").contains("LoginGrant"));

    let info = hosted::account_info(&w.server.base, &grant.token).unwrap();
    assert!(!info.totp_enabled);
    assert_eq!(info.params.account_id, w.account);
    let (again, epoch) = hosted::account_epoch(&w.server.base, &grant.token).unwrap();
    assert_eq!(epoch, 1);
    assert_eq!(again.account_version, info.account_version);
    assert_eq!(again.wrapped_ak.as_bytes(), info.wrapped_ak.as_bytes());

    // Second factor: set up, a wrong code is refused, the right one confirms.
    let setup = hosted::totp_setup(&w.server.base, &grant.token).unwrap();
    assert_eq!(format!("{setup:?}"), "TotpSetup([REDACTED])");
    let secret = setup.secret.expose_secret().to_owned();
    assert!(matches!(
        hosted::totp_confirm(&w.server.base, &grant.token, "000000"),
        Err(TransportError::Rejected(Rejection::TwoFactorInvalid))
    ));
    let code = code_now(&w, &secret);
    hosted::totp_confirm(&w.server.base, &grant.token, &code).unwrap();
    assert!(
        hosted::account_info(&w.server.base, &grant.token)
            .unwrap()
            .totp_enabled
    );

    // A login now needs the code.
    let pending = login(&w, device, "device 0");
    assert_eq!(
        pending.status,
        mosaic_sync::wire::json::LoginStatus::TwoFactorRequired
    );
    w.server.advance_ms(30_000);
    let code = code_now(&w, &secret);
    hosted::login_totp(&w.server.base, &pending.token, &code).unwrap();

    // Devices: a second one signs in, is renamed and removed again.
    let other = InstallId::from_bytes([0x77; 16]).unwrap();
    let second = login(&w, other, "second");
    w.server.advance_ms(30_000);
    let code = code_now(&w, &secret);
    hosted::login_totp(&w.server.base, &second.token, &code).unwrap();
    hosted::device_rename(&w.server.base, &pending.token, other, "renamed").unwrap();
    hosted::device_remove(&w.server.base, &pending.token, other).unwrap();
    assert!(
        hosted::device_remove(&w.server.base, &pending.token, other).is_err(),
        "a removed device is gone"
    );

    // Turning it off needs a code; then the session ends.
    w.server.advance_ms(30_000);
    let code = code_now(&w, &secret);
    hosted::totp_disable(&w.server.base, &pending.token, &code).unwrap();
    hosted::logout(&w.server.base, &pending.token).unwrap();
}

#[test]
fn rekey_refused_when_a_head_moved_and_when_the_journal_write_fails() {
    let mut w = World::new(302, ChaosConfig::default());
    let device = w.devs[0].device;
    let prepared = prepared(&mut w, true);
    w.server.advance_ms(60_000);
    let outcome = hosted::rekey(
        &w.server.base,
        &w.email,
        &prepared,
        device,
        "device 0",
        None,
        &mut |_| true,
    )
    .unwrap();
    assert!(matches!(
        outcome,
        RekeyOutcome::Refused(RekeyRefusal::HeadMoved)
    ));

    // A journal write that fails stops the request before it is sent.
    w.server.advance_ms(60_000);
    let mut stages = Vec::new();
    let err = hosted::rekey(
        &w.server.base,
        &w.email,
        &prepared,
        device,
        "device 0",
        None,
        &mut |s| {
            stages.push(s);
            false
        },
    )
    .unwrap_err();
    assert!(matches!(err, TransportError::Io));
    assert_eq!(stages, vec![RekeyStage::Started]);
}

#[test]
fn rekey_retried_after_a_lost_answer_finds_it_applied() {
    let mut w = World::new(303, ChaosConfig::default());
    let device = w.devs[0].device;
    let prepared = prepared(&mut w, false);
    w.server.advance_ms(60_000);
    let first = hosted::rekey(
        &w.server.base,
        &w.email,
        &prepared,
        device,
        "device 0",
        None,
        &mut |_| true,
    )
    .unwrap();
    let RekeyOutcome::Done(done) = first else {
        panic!("the first re-key must apply");
    };
    assert_eq!(done.ak_epoch, 2);
    assert!(format!("{done:?}").contains("RekeyFinished"));

    // The answer was lost: the retry finds the old password refused, logs in
    // with the new one and sees the prepared wrapping at the target epoch.
    w.server.advance_ms(60_000);
    let mut stages = Vec::new();
    let retry = hosted::rekey(
        &w.server.base,
        &w.email,
        &prepared,
        device,
        "device 0",
        None,
        &mut |s| {
            stages.push(s);
            true
        },
    )
    .unwrap();
    let RekeyOutcome::Done(again) = retry else {
        panic!("the retry must settle as applied");
    };
    assert_eq!(again.ak_epoch, 2);
    assert_eq!(again.account_version, done.account_version);
    assert_eq!(stages, vec![RekeyStage::Accepted]);
}

#[test]
fn recovery_with_wrong_and_right_keys() {
    let w = World::new(304, ChaosConfig::default());
    let device = w.devs[0].device;
    let wrong = SecretKey::generate().unwrap();
    w.server.advance_ms(60_000);
    let err = hosted::recover_password(
        &w.server.base,
        &w.email,
        &pw(PASSWORD),
        &wrong,
        device,
        "device 0",
        None,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        TransportError::Rejected(Rejection::LoginFailed)
    ));
    w.server.advance_ms(60_000);
    let req = hosted::RecoverRequest {
        base: &w.server.base,
        address: &w.email,
        new_password: &pw(PASSWORD),
        new_secret_key: &wrong,
        device,
        device_name: "device 0",
        totp_code: None,
        member_vaults: &[],
    };
    assert_eq!(format!("{req:?}"), "RecoverRequest([REDACTED])");
    assert!(hosted::recover_rekey(&req).is_err());

    // The right keys log in and recover the account as it stands.
    w.server.advance_ms(60_000);
    let recovered = hosted::recover_password(
        &w.server.base,
        &w.email,
        &pw(PASSWORD),
        &w.secret_key,
        device,
        "device 0",
        None,
    )
    .unwrap();
    assert!(format!("{recovered:?}").contains("Recovered"));
    w.server.advance_ms(60_000);
    let req = hosted::RecoverRequest {
        new_secret_key: &w.secret_key,
        ..req
    };
    let rekeyed = hosted::recover_rekey(&req).unwrap();
    assert_eq!(rekeyed.account.ak_epoch, 1);
    assert!(format!("{rekeyed:?}").contains("RekeyRecovered"));
}
