//! Nextcloud's Login Flow v2: the phone opens `login_url` in the system
//! browser, the user approves, and polling the flow's endpoint hands back an
//! app password for this device. The app password can be revoked again
//! (`DELETE /ocs/v2.php/core/apppassword`).
//!
//! Everything the server names (the poll endpoint, the server in the result)
//! must be the origin the user confirmed, or the flow ends in
//! [`TransportError::Protocol`]: a malicious or compromised server cannot send
//! the app password to another host.

use reqwest::Method;
use reqwest::header::{AUTHORIZATION, HeaderName, HeaderValue, USER_AGENT};
use serde::Deserialize;
use zeroize::Zeroizing;

use super::MAX_SMALL_JSON;
use super::credential::ProviderCredential;
use super::http::{RemoteBody, RemoteHttp, RemoteReply, RemoteRequest, status_error};
use super::oauth::{form, path_segment};
use super::webdav::basic_auth;
use crate::folder::store::valid_component;
use crate::http::canonical_origin;
use crate::http::client::REQUEST_TIMEOUT;
use crate::transport::TransportError;

/// What polling needs. The token is a secret until the flow ends.
pub struct LoginPoll {
    endpoint: String,
    token: Zeroizing<String>,
    origin: String,
}

impl core::fmt::Debug for LoginPoll {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("LoginPoll([REDACTED])")
    }
}

/// The start of a login flow.
#[derive(Debug)]
pub struct LoginFlowStart {
    /// The page to open in the system browser.
    pub login_url: String,
    /// What to poll with.
    pub poll: LoginPoll,
}

#[derive(Deserialize)]
struct Begin {
    poll: BeginPoll,
    login: String,
}

#[derive(Deserialize)]
struct BeginPoll {
    token: String,
    endpoint: String,
}

#[derive(Deserialize)]
struct Done {
    server: String,
    #[serde(rename = "loginName")]
    login_name: String,
    #[serde(rename = "appPassword")]
    app_password: String,
}

/// Whether `url` is on exactly `origin` (a canonical origin).
fn on_origin(url: &str, origin: &str) -> bool {
    let Some((scheme, rest)) = url.split_once("://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let tail = rest.get(authority.len()..).unwrap_or_default();
    canonical_origin(&format!("{scheme}://{authority}")).as_deref() == Ok(origin)
        && !authority.contains('@')
        && !tail.contains('#')
}

fn expect_ok(reply: &RemoteReply) -> Result<(), TransportError> {
    match reply.status {
        200 => Ok(()),
        _ if reply.is_success() => Err(TransportError::Protocol),
        _ => Err(status_error(reply)),
    }
}

/// Starts a login flow on `origin` (the server the user confirmed).
///
/// # Errors
/// [`TransportError::InsecureUrl`] or `InvalidUrl` for the origin;
/// [`TransportError::Protocol`] for an answer that is not the flow's or that
/// names another origin; the usual transport errors.
pub fn login_flow_begin(
    http: &RemoteHttp,
    origin: &str,
    user_agent: &str,
) -> Result<LoginFlowStart, TransportError> {
    let origin = canonical_origin(origin)?;
    let agent = HeaderValue::from_str(user_agent).map_err(|_| TransportError::Protocol)?;
    let reply = http.send(RemoteRequest {
        method: Method::POST,
        url: &format!("{origin}/index.php/login/v2"),
        headers: &[(USER_AGENT, agent)],
        body: RemoteBody::None,
        timeout: REQUEST_TIMEOUT,
        cap: MAX_SMALL_JSON,
    })?;
    expect_ok(&reply)?;
    let body = Zeroizing::new(reply.body);
    let mut begin: Begin = serde_json::from_slice(&body).map_err(|_| TransportError::Protocol)?;
    let token = Zeroizing::new(std::mem::take(&mut begin.poll.token));
    if token.is_empty()
        || !on_origin(&begin.poll.endpoint, &origin)
        || !on_origin(&begin.login, &origin)
    {
        return Err(TransportError::Protocol);
    }
    Ok(LoginFlowStart {
        login_url: begin.login,
        poll: LoginPoll {
            endpoint: begin.poll.endpoint,
            token,
            origin,
        },
    })
}

/// Polls once. `None` while the user has not approved yet; after approval the
/// app password and the collection URL of the account
/// (`<origin>/remote.php/dav/files/<login>`).
///
/// # Errors
/// [`TransportError::Protocol`] for an answer that is not the flow's or whose
/// server is not the origin; the usual transport errors.
pub fn login_flow_poll(
    http: &RemoteHttp,
    poll: &LoginPoll,
) -> Result<Option<(ProviderCredential, String)>, TransportError> {
    let reply = http.send(RemoteRequest {
        method: Method::POST,
        url: &poll.endpoint,
        headers: &[],
        body: RemoteBody::Form(form(&[("token", poll.token.as_str())])),
        timeout: REQUEST_TIMEOUT,
        cap: MAX_SMALL_JSON,
    })?;
    if reply.status == 404 {
        return Ok(None);
    }
    expect_ok(&reply)?;
    let body = Zeroizing::new(reply.body);
    let mut done: Done = serde_json::from_slice(&body).map_err(|_| TransportError::Protocol)?;
    let app_password = Zeroizing::new(std::mem::take(&mut done.app_password));
    let server = done.server.trim_end_matches('/');
    if canonical_origin(server).as_deref() != Ok(poll.origin.as_str())
        || !valid_component(&done.login_name)
        || app_password.is_empty()
    {
        return Err(TransportError::Protocol);
    }
    let base = format!(
        "{}/remote.php/dav/files/{}",
        poll.origin,
        path_segment(&done.login_name)
    );
    Ok(Some((
        ProviderCredential::WebDav {
            login: done.login_name,
            app_password,
        },
        base,
    )))
}

/// Revokes the app password on the server (`DELETE
/// /ocs/v2.php/core/apppassword`, authenticated with the password itself).
///
/// # Errors
/// [`TransportError::Protocol`] for a credential that is not a `WebDAV` one;
/// [`TransportError::Unauthorized`] if the server no longer knows it; the
/// usual transport errors.
pub fn revoke_app_password(
    http: &RemoteHttp,
    origin: &str,
    credential: &ProviderCredential,
) -> Result<(), TransportError> {
    let origin = canonical_origin(origin)?;
    let ProviderCredential::WebDav {
        login,
        app_password,
    } = credential
    else {
        return Err(TransportError::Protocol);
    };
    let reply = http.send(RemoteRequest {
        method: Method::DELETE,
        url: &format!("{origin}/ocs/v2.php/core/apppassword"),
        headers: &[
            (AUTHORIZATION, basic_auth(login, app_password)?),
            (
                HeaderName::from_static("ocs-apirequest"),
                HeaderValue::from_static("true"),
            ),
        ],
        body: RemoteBody::None,
        timeout: REQUEST_TIMEOUT,
        cap: MAX_SMALL_JSON,
    })?;
    if reply.is_success() {
        Ok(())
    } else {
        Err(status_error(&reply))
    }
}
