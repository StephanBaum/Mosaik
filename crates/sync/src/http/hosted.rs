//! The client side of the hosted account protocol (CRYPTO_SPEC §9, M6 spec
//! §6.2, §6.3): registration, login (with the second factor), the join of a
//! new device and the master password change, all over OPAQUE.
//!
//! The server never sees the master password, the Secret Key or a key: the
//! client sends OPAQUE messages made from `OPW` (CRYPTO_SPEC §5.4), which
//! already is Argon2id-stretched and carries the Secret Key. A session token
//! is derived from the OPAQUE session key on both sides
//! (`sync_auth::session_token`); the server never sends it.
//!
//! Every function checks `GET /v1/info` first: the server must name the
//! origin the caller asked for, because the OPAQUE login is bound to it.

use mosaic_core::sync_auth::session_token;
use mosaic_core::{
    AccountId, AccountSalt, InstallId, MasterPassword, OpaquePassword, PasswordParams, SecretKey,
    SessionToken, WrappedKey,
};
use mosaic_vault::session::{AccountBootstrap, JoinKeys, LockedSession, PreparedPasswordChange};
use reqwest::Method;
use zeroize::Zeroize;

use super::client::{Bearer, Http, Token, canonical_origin};
use crate::transport::{Rejection, TransportError};
use crate::wire::email;
use crate::wire::json::{
    B64, B64Fixed, CodeRequest, Empty, LoginFinishRequest, LoginResponse, LoginStartRequest,
    LoginStartResponse, LoginStatus, PasswordFinishRequest, PasswordFinishResponse,
    PasswordStartRequest, PreloginRequest, PreloginResponse, RegisterFinishRequest,
    RegisterStartRequest, RegistrationStartResponse,
};
use crate::wire::opaque::ke::{ClientLoginFinishResult, ClientRegistrationFinishResult};
use crate::wire::opaque::{
    LOGIN_CONTEXT, OsRng, Suite, identifiers,
    ke::{
        ClientLogin, ClientLoginFinishParameters, ClientRegistration,
        ClientRegistrationFinishParameters, CredentialResponse, RegistrationResponse,
    },
};

/// A session the server granted.
pub struct LoginGrant {
    /// The bearer token for [`super::HttpTransport::connect`]. Zeroizing,
    /// redacted in `Debug`.
    pub token: SessionToken,
    /// `TwoFactorRequired`: the session is pending until [`login_totp`].
    pub status: LoginStatus,
    /// When the session expires if idle (ms since the Unix epoch, server
    /// time).
    pub expires_at_ms: u64,
}

impl core::fmt::Debug for LoginGrant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoginGrant")
            .field("status", &self.status)
            .field("expires_at_ms", &self.expires_at_ms)
            .finish_non_exhaustive()
    }
}

/// A join login: the derived keys and the session.
pub struct JoinLogin {
    /// The keys of the join, to hand to
    /// [`LockedSession::join_account_with_keys`].
    pub keys: JoinKeys,
    /// The session (pending while `status` asks for the second factor).
    pub grant: LoginGrant,
}

impl core::fmt::Debug for JoinLogin {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("JoinLogin")
            .field("grant", &self.grant)
            .finish_non_exhaustive()
    }
}

/// The account values a joining device fetches.
#[derive(Debug)]
pub struct JoinAccount {
    /// `AK` wrapped under the password `KEK`.
    pub wrapped_ak: WrappedKey,
    /// The account version.
    pub account_version: u64,
    /// The account has a second factor.
    pub totp_enabled: bool,
}

/// The result of a password change.
pub struct PasswordChanged {
    /// The account version after the change.
    pub account_version: u64,
    /// The fresh session of the change; it stays valid (the device does not
    /// have to log in again).
    pub grant: LoginGrant,
}

impl core::fmt::Debug for PasswordChanged {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PasswordChanged")
            .field("account_version", &self.account_version)
            .finish_non_exhaustive()
    }
}

const OK_STATUS: [u16; 1] = [200];

/// Wipes what a finished OPAQUE registration holds that this client never
/// uses: opaque-ke keeps the export key in a plain array.
fn wipe_registration(finished: &mut ClientRegistrationFinishResult<Suite>) {
    finished.export_key.as_mut_slice().zeroize();
}

/// Wipes the session key and the export key of a finished OPAQUE login (the
/// session token was derived from the first; the second is not used).
fn wipe_login(finished: &mut ClientLoginFinishResult<Suite>) {
    finished.session_key.as_mut_slice().zeroize();
    finished.export_key.as_mut_slice().zeroize();
}

fn connect(base: &str) -> Result<Http, TransportError> {
    let http = Http::new(&canonical_origin(base)?)?;
    http.info()?;
    Ok(http)
}

fn origin_of(base: &str) -> Result<String, TransportError> {
    canonical_origin(base)
}

fn normalized(address: &str) -> Result<String, TransportError> {
    email::normalize(address).map_err(|_| TransportError::Rejected(Rejection::InvalidEmail))
}

/// `POST /v1/prelogin`: the account ID, salt and Argon2id profile of an
/// email (real, or the server's deterministic fake for an unknown one).
///
/// # Errors
/// [`TransportError`].
pub fn prelogin(base: &str, address: &str) -> Result<PasswordParams, TransportError> {
    let http = connect(base)?;
    prelogin_with(&http, address)
}

fn prelogin_with(http: &Http, address: &str) -> Result<PasswordParams, TransportError> {
    let (_, r): (u16, PreloginResponse) = http.call(
        Method::POST,
        "/v1/prelogin",
        None,
        Some(&PreloginRequest {
            email: normalized(address)?,
        }),
        &OK_STATUS,
    )?;
    Ok(PasswordParams {
        account_id: r.account_id,
        salt: AccountSalt::from_bytes(r.salt.0),
        profile: r.profile,
    })
}

/// Registers an account: OPAQUE registration of `OPW`, then the public
/// parameters and the wrapped account key from `bootstrap` (the local
/// account, [`mosaic_vault::session::UnlockedSession::account_bootstrap`]).
/// `device` is registered with the account.
///
/// # Errors
/// [`Rejection::AccountExists`], [`Rejection::RegistrationClosed`],
/// [`Rejection::InvalidInvite`], [`Rejection::InvalidEmail`] and the other
/// [`TransportError`]s.
pub fn register(
    base: &str,
    address: &str,
    invite_code: Option<&str>,
    bootstrap: &AccountBootstrap,
    opaque_password: &OpaquePassword,
    device: InstallId,
    device_name: &str,
) -> Result<(), TransportError> {
    let http = connect(base)?;
    let origin = origin_of(base)?;
    let address = normalized(address)?;
    let account_id = bootstrap.params.account_id;
    let mut rng = OsRng;
    let start = ClientRegistration::<Suite>::start(&mut rng, opaque_password.expose_secret())
        .map_err(|_| TransportError::Io)?;
    let (_, started): (u16, RegistrationStartResponse) = http.call(
        Method::POST,
        "/v1/register/start",
        None,
        Some(&RegisterStartRequest {
            email: address.clone(),
            account_id,
            registration_request: B64(start.message.serialize().to_vec()),
        }),
        &OK_STATUS,
    )?;
    let response = RegistrationResponse::deserialize(&started.registration_response.0)
        .map_err(|_| TransportError::Protocol)?;
    let mut finished = start
        .state
        .finish(
            &mut rng,
            opaque_password.expose_secret(),
            response,
            ClientRegistrationFinishParameters::new(
                identifiers(account_id.as_bytes(), origin.as_bytes()),
                None,
            ),
        )
        .map_err(|_| TransportError::Protocol)?;
    wipe_registration(&mut finished);
    http.call::<_, Empty>(
        Method::POST,
        "/v1/register/finish",
        None,
        Some(&RegisterFinishRequest {
            email: address,
            account_id,
            salt: B64Fixed(*bootstrap.params.salt.as_bytes()),
            profile: bootstrap.params.profile,
            registration_upload: B64(finished.message.serialize().to_vec()),
            wrapped_ak: bootstrap.wrapped_ak.clone(),
            invite_code: invite_code.map(str::to_owned),
            device_id: device,
            device_name: device_name.to_owned(),
        }),
        &[201],
    )?;
    Ok(())
}

/// OPAQUE login. The caller learns the account ID from [`prelogin`] (or has
/// it stored). A wrong password, an unknown account and a server that names
/// another identity all end as [`Rejection::LoginFailed`] (the first two are
/// the same answer by design, CRYPTO_SPEC §9.2).
///
/// With `TwoFactorRequired` the session is pending: finish with
/// [`login_totp`].
///
/// # Errors
/// [`Rejection::LoginFailed`], [`Rejection::DeviceRevoked`] and the other
/// [`TransportError`]s.
pub fn login(
    base: &str,
    address: &str,
    account_id: AccountId,
    opaque_password: &OpaquePassword,
    device: InstallId,
    device_name: &str,
) -> Result<LoginGrant, TransportError> {
    let http = connect(base)?;
    login_with(
        &http,
        &origin_of(base)?,
        address,
        account_id,
        opaque_password,
        device,
        device_name,
    )
}

fn login_with(
    http: &Http,
    origin: &str,
    address: &str,
    account_id: AccountId,
    opaque_password: &OpaquePassword,
    device: InstallId,
    device_name: &str,
) -> Result<LoginGrant, TransportError> {
    let mut rng = OsRng;
    let start = ClientLogin::<Suite>::start(&mut rng, opaque_password.expose_secret())
        .map_err(|_| TransportError::Io)?;
    let (_, started): (u16, LoginStartResponse) = http.call(
        Method::POST,
        "/v1/login/start",
        None,
        Some(&LoginStartRequest {
            email: normalized(address)?,
            credential_request: B64(start.message.serialize().to_vec()),
        }),
        &OK_STATUS,
    )?;
    let response = CredentialResponse::deserialize(&started.credential_response.0)
        .map_err(|_| TransportError::Protocol)?;
    let mut finished = start
        .state
        .finish(
            &mut rng,
            opaque_password.expose_secret(),
            response,
            ClientLoginFinishParameters::new(
                Some(LOGIN_CONTEXT),
                identifiers(account_id.as_bytes(), origin.as_bytes()),
                None,
            ),
        )
        // The password is wrong, or the server's response is not for this
        // account and origin: the same to the caller.
        .map_err(|_| TransportError::Rejected(Rejection::LoginFailed))?;
    let token = session_token(&finished.session_key);
    wipe_login(&mut finished);
    let token = token.map_err(|_| TransportError::Io)?;
    let (_, done): (u16, LoginResponse) = http.call(
        Method::POST,
        "/v1/login/finish",
        None,
        Some(&LoginFinishRequest {
            login_id: started.login_id,
            credential_finalization: B64(finished.message.serialize().to_vec()),
            device_id: device,
            device_name: device_name.to_owned(),
        }),
        &OK_STATUS,
    )?;
    Ok(LoginGrant {
        token,
        status: done.status,
        expires_at_ms: done.expires_at,
    })
}

/// Completes a pending session with a TOTP code (`POST /v1/login/totp`).
/// Returns when the session expires if idle.
///
/// # Errors
/// [`Rejection::TwoFactorInvalid`] (five wrong codes delete the pending
/// session: log in again) and the other [`TransportError`]s.
pub fn login_totp(base: &str, token: &impl Token, code: &str) -> Result<u64, TransportError> {
    let http = connect(base)?;
    login_totp_with(&http, token, code)
}

fn login_totp_with(http: &Http, token: &impl Token, code: &str) -> Result<u64, TransportError> {
    let bearer = Bearer::new(token)?;
    let (_, r): (u16, LoginResponse) = http.call(
        Method::POST,
        "/v1/login/totp",
        Some(&bearer),
        Some(&CodeRequest { code: code.into() }),
        &OK_STATUS,
    )?;
    if r.status != LoginStatus::Ok {
        return Err(TransportError::Protocol);
    }
    Ok(r.expires_at)
}

/// Ends the session on the server (`POST /v1/logout`).
///
/// # Errors
/// [`TransportError`].
pub fn logout(base: &str, token: &impl Token) -> Result<(), TransportError> {
    let http = connect(base)?;
    let bearer = Bearer::new(token)?;
    http.call::<Empty, Empty>(Method::POST, "/v1/logout", Some(&bearer), None, &OK_STATUS)
        .map(|_| ())
}

/// What `GET /v1/account` says about the signed-in account, for a client
/// that already holds its keys.
#[derive(Clone, Debug)]
pub struct AccountInfo {
    /// The account version.
    pub account_version: u64,
    /// The account has a second factor.
    pub totp_enabled: bool,
    /// The public parameters the server holds (account ID, salt, profile).
    pub params: PasswordParams,
    /// `AK` wrapped under the server's current password `KEK`: a device that
    /// compares it with its own wrapping finds a master password change it
    /// missed.
    pub wrapped_ak: WrappedKey,
}

/// `GET /v1/account` with a completed session.
///
/// # Errors
/// [`TransportError`].
pub fn account_info(base: &str, token: &impl Token) -> Result<AccountInfo, TransportError> {
    let http = connect(base)?;
    let bearer = Bearer::new(token)?;
    let (_, account): (u16, crate::wire::json::AccountResponse) = http.call(
        Method::GET,
        "/v1/account",
        Some(&bearer),
        None::<&Empty>,
        &OK_STATUS,
    )?;
    Ok(AccountInfo {
        account_version: account.account_version,
        totp_enabled: account.totp_enabled,
        params: PasswordParams {
            account_id: account.account_id,
            salt: AccountSalt::from_bytes(account.salt.0),
            profile: account.profile,
        },
        wrapped_ak: account.wrapped_ak,
    })
}

/// The secret of a second factor being set up: the base32 secret and the
/// `otpauth://` link that holds it. Zeroizing, redacted in `Debug`.
pub struct TotpSetup {
    /// The base32 secret.
    pub secret: crate::wire::json::SecretText,
    /// The `otpauth://` link.
    pub otpauth_uri: crate::wire::json::SecretText,
}

impl core::fmt::Debug for TotpSetup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("TotpSetup([REDACTED])")
    }
}

/// `POST /v1/account/totp/setup`: the server creates a pending second-factor
/// secret. Needs a fresh login (the session of [`login`] within five
/// minutes).
///
/// # Errors
/// [`TransportError::FreshLoginRequired`] and the other [`TransportError`]s.
pub fn totp_setup(base: &str, token: &impl Token) -> Result<TotpSetup, TransportError> {
    let http = connect(base)?;
    let bearer = Bearer::new(token)?;
    let (_, r): (u16, crate::wire::json::TotpSetupResponse) = http.call(
        Method::POST,
        "/v1/account/totp/setup",
        Some(&bearer),
        None::<&Empty>,
        &OK_STATUS,
    )?;
    Ok(TotpSetup {
        secret: r.secret,
        otpauth_uri: r.otpauth_uri,
    })
}

/// `POST /v1/account/totp/confirm`: turns the pending second factor on once
/// the code from the authenticator app is right. Needs a fresh login.
///
/// # Errors
/// [`Rejection::TwoFactorInvalid`], [`TransportError::FreshLoginRequired`]
/// and the other [`TransportError`]s.
pub fn totp_confirm(base: &str, token: &impl Token, code: &str) -> Result<(), TransportError> {
    totp_code_call(base, token, "/v1/account/totp/confirm", code)
}

/// `POST /v1/account/totp/disable`: turns the second factor off. Needs a fresh
/// login and a current code.
///
/// # Errors
/// As [`totp_confirm`].
pub fn totp_disable(base: &str, token: &impl Token, code: &str) -> Result<(), TransportError> {
    totp_code_call(base, token, "/v1/account/totp/disable", code)
}

fn totp_code_call(
    base: &str,
    token: &impl Token,
    path: &str,
    code: &str,
) -> Result<(), TransportError> {
    let http = connect(base)?;
    let bearer = Bearer::new(token)?;
    http.call::<_, Empty>(
        Method::POST,
        path,
        Some(&bearer),
        Some(&CodeRequest { code: code.into() }),
        &OK_STATUS,
    )
    .map(|_| ())
}

/// `PATCH /v1/devices/{device}`: renames a device of the account.
///
/// # Errors
/// [`TransportError::NotFound`] and the other [`TransportError`]s.
pub fn device_rename(
    base: &str,
    token: &impl Token,
    device: InstallId,
    name: &str,
) -> Result<(), TransportError> {
    let http = connect(base)?;
    let bearer = Bearer::new(token)?;
    http.call::<_, Empty>(
        Method::PATCH,
        &format!("/v1/devices/{device}"),
        Some(&bearer),
        Some(&crate::wire::json::RenameDeviceRequest {
            name: name.to_owned(),
        }),
        &OK_STATUS,
    )
    .map(|_| ())
}

/// `DELETE /v1/devices/{device}`: ends the device's sessions and refuses its
/// later logins. Needs a fresh login; a device cannot remove itself.
///
/// # Errors
/// [`Rejection::DeviceIsCurrent`], [`TransportError::FreshLoginRequired`]
/// and the other [`TransportError`]s.
pub fn device_remove(
    base: &str,
    token: &impl Token,
    device: InstallId,
) -> Result<(), TransportError> {
    let http = connect(base)?;
    let bearer = Bearer::new(token)?;
    http.call::<Empty, Empty>(
        Method::DELETE,
        &format!("/v1/devices/{device}"),
        Some(&bearer),
        None,
        &OK_STATUS,
    )
    .map(|_| ())
}

/// The first half of joining an account from a new device: pre-login for the
/// account's parameters, the Argon2id derivation of the keys from the
/// password and the Secret Key, and an OPAQUE login with `OPW`.
///
/// With `TwoFactorRequired` in the grant, finish the session with
/// [`login_totp`] before [`join_fetch_account`].
///
/// # Errors
/// [`Rejection::LoginFailed`] for a wrong password, Secret Key or email;
/// [`Rejection::BadRequest`] for an empty password; the other
/// [`TransportError`]s.
pub fn join_login(
    base: &str,
    address: &str,
    password: &MasterPassword,
    secret_key: &SecretKey,
    device: InstallId,
    device_name: &str,
) -> Result<JoinLogin, TransportError> {
    let http = connect(base)?;
    let origin = origin_of(base)?;
    let params = prelogin_with(&http, address)?;
    let keys =
        LockedSession::derive_join_keys(&params, password, secret_key).map_err(|e| match e {
            mosaic_vault::session::Error::Core(mosaic_core::Error::EmptyPassword) => {
                TransportError::Rejected(Rejection::BadRequest)
            }
            _ => TransportError::Io,
        })?;
    let grant = login_with(
        &http,
        &origin,
        address,
        params.account_id,
        keys.opaque_password(),
        device,
        device_name,
    )?;
    Ok(JoinLogin { keys, grant })
}

/// The second half of the server side of a join: `GET /v1/account` with the
/// completed session. The account ID, salt and profile it names must be the
/// ones the keys were derived under.
///
/// # Errors
/// [`TransportError::Protocol`] if the server's parameters differ from the
/// pre-login's; the other [`TransportError`]s.
pub fn join_fetch_account(
    base: &str,
    token: &impl Token,
    keys: &JoinKeys,
) -> Result<JoinAccount, TransportError> {
    let http = connect(base)?;
    let bearer = Bearer::new(token)?;
    let (_, account): (u16, crate::wire::json::AccountResponse) = http.call(
        Method::GET,
        "/v1/account",
        Some(&bearer),
        None::<&Empty>,
        &OK_STATUS,
    )?;
    let params = keys.params();
    if account.account_id != params.account_id
        || account.salt.0 != *params.salt.as_bytes()
        || account.profile != params.profile
    {
        return Err(TransportError::Protocol);
    }
    Ok(JoinAccount {
        wrapped_ak: account.wrapped_ak,
        account_version: account.account_version,
        totp_enabled: account.totp_enabled,
    })
}

/// What [`recover_password`] returns: the keys of the new password, the
/// server's wrapped `AK` and the session of the login.
pub struct Recovered {
    /// The keys derived from the typed password under the server's
    /// parameters ([`JoinKeys::params`] are those parameters), to hand to
    /// [`mosaic_vault::session::UnlockedSession::adopt_remote_password`].
    pub keys: JoinKeys,
    /// The account values the server holds now.
    pub account: JoinAccount,
    /// The session (completed: the second factor, if any, was passed).
    pub grant: LoginGrant,
}

impl core::fmt::Debug for Recovered {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Recovered")
            .field("grant", &self.grant)
            .finish_non_exhaustive()
    }
}

/// The server side of recovering a master password change that another run
/// of this device applied to the server only (CRYPTO_SPEC §9.4, "Change
/// applied elsewhere"): pre-login for the server's current parameters, the
/// Argon2id derivation from the typed (new) password, an OPAQUE login, the
/// second factor if the account has one, and `GET /v1/account`. The caller
/// adopts the result with
/// [`mosaic_vault::session::UnlockedSession::adopt_remote_password`], which
/// refuses it unless it opens to the device's own account key.
///
/// # Errors
/// As [`join_login`] and [`join_fetch_account`], plus
/// [`TransportError::TwoFactorRequired`] if the account needs `totp_code`
/// and none was given and [`Rejection::TwoFactorInvalid`].
pub fn recover_password(
    base: &str,
    address: &str,
    new_password: &MasterPassword,
    secret_key: &SecretKey,
    device: InstallId,
    device_name: &str,
    totp_code: Option<&str>,
) -> Result<Recovered, TransportError> {
    let mut login = join_login(base, address, new_password, secret_key, device, device_name)?;
    if login.grant.status == LoginStatus::TwoFactorRequired {
        let code = totp_code.ok_or(TransportError::TwoFactorRequired)?;
        login.grant.expires_at_ms = login_totp(base, &login.grant.token, code)?;
        login.grant.status = LoginStatus::Ok;
    }
    let account = join_fetch_account(base, &login.grant.token, &login.keys)?;
    Ok(Recovered {
        keys: login.keys,
        account,
        grant: login.grant,
    })
}

/// Changes the account's master password on the server (CRYPTO_SPEC §9.4):
/// a fresh login with the old `OPW`, the second factor if the account has
/// one, OPAQUE re-registration with the new `OPW`, and one upload of the new
/// registration, salt, profile and wrapped account key. The server accepts
/// the upload only if the account is still at the version this session
/// logged in at, and ends every other session of the account with it.
/// Commit the change locally only after this returned
/// ([`mosaic_vault::session::UnlockedSession::commit_master_password_change`]).
///
/// A lost answer to the upload is recovered on the retry: if the old `OPW`
/// no longer logs in but the new one does, and the account's wrapped `AK` on
/// the server is byte-equal to the prepared one, the change went through and
/// this returns as if it had just succeeded. Anything else is the original
/// error.
///
/// # Errors
/// [`Rejection::LoginFailed`] for a wrong current password;
/// [`TransportError::TwoFactorRequired`] if the account needs `totp_code`
/// and none was given; [`Rejection::TwoFactorInvalid`];
/// [`Rejection::AccountVersionMismatch`] if another change got in first; the
/// other [`TransportError`]s.
pub fn password_change(
    base: &str,
    address: &str,
    prepared: &PreparedPasswordChange,
    device: InstallId,
    device_name: &str,
    totp_code: Option<&str>,
) -> Result<PasswordChanged, TransportError> {
    let http = connect(base)?;
    let origin = origin_of(base)?;
    let account_id = prepared.new_params().account_id;
    let grant = match login_with(
        &http,
        &origin,
        address,
        account_id,
        prepared.old_opaque_password(),
        device,
        device_name,
    ) {
        Ok(grant) => grant,
        Err(TransportError::Rejected(Rejection::LoginFailed)) => {
            // The old password may no longer be the account's: the upload of
            // an earlier attempt could have gone through with its answer
            // lost.
            return recover_applied_change(
                &http,
                &origin,
                address,
                prepared,
                device,
                device_name,
                totp_code,
            )
            .map_err(|_| TransportError::Rejected(Rejection::LoginFailed));
        }
        Err(e) => return Err(e),
    };
    let mut grant = grant;
    if grant.status == LoginStatus::TwoFactorRequired {
        let code = totp_code.ok_or(TransportError::TwoFactorRequired)?;
        grant.expires_at_ms = login_totp_with(&http, &grant.token, code)?;
        grant.status = LoginStatus::Ok;
    }
    let bearer = Bearer::new(&grant.token)?;
    let mut rng = OsRng;
    let new_opw = prepared.new_opaque_password().expose_secret();
    let start =
        ClientRegistration::<Suite>::start(&mut rng, new_opw).map_err(|_| TransportError::Io)?;
    let (_, started): (u16, RegistrationStartResponse) = http.call(
        Method::POST,
        "/v1/account/password/start",
        Some(&bearer),
        Some(&PasswordStartRequest {
            registration_request: B64(start.message.serialize().to_vec()),
        }),
        &OK_STATUS,
    )?;
    let response = RegistrationResponse::deserialize(&started.registration_response.0)
        .map_err(|_| TransportError::Protocol)?;
    let mut finished = start
        .state
        .finish(
            &mut rng,
            new_opw,
            response,
            ClientRegistrationFinishParameters::new(
                identifiers(account_id.as_bytes(), origin.as_bytes()),
                None,
            ),
        )
        .map_err(|_| TransportError::Protocol)?;
    wipe_registration(&mut finished);
    let new_params = prepared.new_params();
    let (_, done): (u16, PasswordFinishResponse) = http.call(
        Method::POST,
        "/v1/account/password/finish",
        Some(&bearer),
        Some(&PasswordFinishRequest {
            registration_upload: B64(finished.message.serialize().to_vec()),
            wrapped_ak: prepared.new_wrapped_ak().clone(),
            salt: B64Fixed(*new_params.salt.as_bytes()),
            profile: new_params.profile,
        }),
        &OK_STATUS,
    )?;
    Ok(PasswordChanged {
        account_version: done.account_version,
        grant,
    })
}

/// The retry after a lost answer to `password/finish`: the new `OPW` logs in
/// and the account's wrapped `AK` is the prepared one.
fn recover_applied_change(
    http: &Http,
    origin: &str,
    address: &str,
    prepared: &PreparedPasswordChange,
    device: InstallId,
    device_name: &str,
    totp_code: Option<&str>,
) -> Result<PasswordChanged, TransportError> {
    let account_id = prepared.new_params().account_id;
    let mut grant = login_with(
        http,
        origin,
        address,
        account_id,
        prepared.new_opaque_password(),
        device,
        device_name,
    )?;
    if grant.status == LoginStatus::TwoFactorRequired {
        let code = totp_code.ok_or(TransportError::TwoFactorRequired)?;
        grant.expires_at_ms = login_totp_with(http, &grant.token, code)?;
        grant.status = LoginStatus::Ok;
    }
    let bearer = Bearer::new(&grant.token)?;
    let (_, account): (u16, crate::wire::json::AccountResponse) = http.call(
        Method::GET,
        "/v1/account",
        Some(&bearer),
        None::<&Empty>,
        &OK_STATUS,
    )?;
    if account.account_id != account_id
        || account.wrapped_ak.as_bytes() != prepared.new_wrapped_ak().as_bytes()
    {
        return Err(TransportError::Protocol);
    }
    Ok(PasswordChanged {
        account_version: account.account_version,
        grant,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::opaque::ke::{
        ServerLogin, ServerLoginParameters, ServerRegistration, ServerSetup,
    };

    /// C-L1: the session key and the export key of a finished login, and the
    /// export key of a finished registration, are zero after the wipe (and
    /// were not before it).
    #[test]
    fn opaque_keys_are_wiped_after_use() {
        let mut rng = OsRng;
        let setup = ServerSetup::<Suite>::new(&mut rng);
        let account = [7u8; 16];
        let origin = b"https://sync.example.com";
        let opw = [9u8; 32];
        let ids = || identifiers(&account, origin);

        let start = ClientRegistration::<Suite>::start(&mut rng, &opw).unwrap();
        let resp = ServerRegistration::<Suite>::start(&setup, start.message, &account).unwrap();
        let mut fin = start
            .state
            .finish(
                &mut rng,
                &opw,
                resp.message,
                ClientRegistrationFinishParameters::new(ids(), None),
            )
            .unwrap();
        assert!(fin.export_key.iter().any(|b| *b != 0));
        let record = ServerRegistration::<Suite>::finish(fin.message.clone());
        wipe_registration(&mut fin);
        assert!(fin.export_key.iter().all(|b| *b == 0));

        let c = ClientLogin::<Suite>::start(&mut rng, &opw).unwrap();
        let s = ServerLogin::start(
            &mut rng,
            &setup,
            Some(record),
            c.message,
            &account,
            ServerLoginParameters {
                context: Some(LOGIN_CONTEXT),
                identifiers: ids(),
            },
        )
        .unwrap();
        let mut done = c
            .state
            .finish(
                &mut rng,
                &opw,
                s.message,
                ClientLoginFinishParameters::new(Some(LOGIN_CONTEXT), ids(), None),
            )
            .unwrap();
        assert!(done.session_key.iter().any(|b| *b != 0));
        assert!(done.export_key.iter().any(|b| *b != 0));
        wipe_login(&mut done);
        assert!(done.session_key.iter().all(|b| *b == 0));
        assert!(done.export_key.iter().all(|b| *b == 0));
    }
}
