//! What a platform autofill request is for (ADR-0092): a web origin or an
//! Android app.

use url::{Host, Url};

use crate::android_browsers::{is_browser_package, is_trusted_browser};
use crate::app::is_valid_package_name;
use crate::{PageOrigin, idn_unicode};

/// Why a request names no usable target.
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TargetError {
    /// The caller's package name is not valid.
    #[error("bad package name")]
    BadPackage,
    /// The domain is not a domain name (an IP address, a URL, an empty string).
    #[error("bad domain")]
    BadDomain,
    /// The scheme is not `http` or `https`.
    #[error("unsupported scheme")]
    Scheme,
    /// The result is not a canonical origin.
    #[error("not an origin")]
    NotAnOrigin,
    /// An allowlisted browser sent no `webDomain`: the page is unknown, and
    /// the browser app itself is never a fill target (M10-G2 H1).
    #[error("browser without a web domain")]
    BrowserWithoutDomain,
    /// The package is an allowlisted browser's, but its signer is not: no
    /// fill, neither as a browser nor as an app (M10-G3-5).
    #[error("browser with an unknown signer")]
    UntrustedBrowser,
}

/// What a fill is for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FillTarget {
    /// A web page.
    Web {
        /// The page origin.
        origin: PageOrigin,
        /// The Unicode form of the host when it has punycode labels (the UI
        /// shows it as a warning).
        idn_unicode: Option<String>,
    },
    /// An Android app that is not a trusted browser.
    AndroidApp {
        /// The package name.
        package: String,
        /// SHA-256 of each current signing certificate.
        certs: Vec<[u8; 32]>,
    },
}

/// An Android autofill caller.
#[derive(Clone, Copy, Debug)]
pub struct AndroidCaller<'a> {
    /// The package of the client activity.
    pub package: &'a str,
    /// SHA-256 of each current signing certificate of that package.
    pub certs: &'a [[u8; 32]],
    /// The `webDomain` of the root view node, if any.
    pub web_domain: Option<&'a str>,
    /// The `webScheme` of the root view node, if any (`https` when absent).
    pub web_scheme: Option<&'a str>,
}

fn web(origin: PageOrigin) -> FillTarget {
    let idn_unicode = idn_unicode(&origin.host_str());
    FillTarget::Web {
        origin,
        idn_unicode,
    }
}

/// The target of an Android request. A `web_domain` counts **only** when the
/// caller is an allowlisted browser (package and certificate); from any other
/// app it is ignored and the target is the app.
///
/// # Errors
/// [`TargetError`].
pub fn android_fill_target(caller: &AndroidCaller<'_>) -> Result<FillTarget, TargetError> {
    if !is_valid_package_name(caller.package) {
        return Err(TargetError::BadPackage);
    }
    if let Some(domain) = caller.web_domain
        && is_trusted_browser(caller.package, caller.certs)
    {
        let scheme = caller.web_scheme.unwrap_or("https");
        if scheme != "http" && scheme != "https" {
            return Err(TargetError::Scheme);
        }
        let origin = PageOrigin::parse(&format!("{scheme}://{}", domain.to_ascii_lowercase()))
            .map_err(|_| TargetError::NotAnOrigin)?;
        return Ok(web(origin));
    }
    if is_trusted_browser(caller.package, caller.certs) {
        return Err(TargetError::BrowserWithoutDomain);
    }
    // A browser package with a signer the list does not know (a sideloaded
    // fake, a rotated key) is no app either (M10-G3-5).
    if is_browser_package(caller.package) {
        return Err(TargetError::UntrustedBrowser);
    }
    Ok(FillTarget::AndroidApp {
        package: caller.package.to_owned(),
        certs: caller.certs.to_vec(),
    })
}

/// An iOS `ASCredentialServiceIdentifier`.
#[derive(Clone, Copy, Debug)]
pub enum IosIdentifier<'a> {
    /// `.domain`: a host name, with no port.
    Domain(&'a str),
    /// `.URL`: a URL; its scheme, host and port form the origin.
    Url(&'a str),
}

/// The target of an iOS request. A `Domain` gives `https://<host>` (the host
/// converted to ASCII); a `Url` keeps its scheme (`http` or `https`), host and
/// port and drops userinfo, path, query and fragment.
///
/// # Errors
/// [`TargetError`].
pub fn ios_fill_target(identifier: IosIdentifier<'_>) -> Result<FillTarget, TargetError> {
    match identifier {
        IosIdentifier::Domain(d) => {
            let Ok(Host::Domain(host)) = Host::parse(d.trim()) else {
                return Err(TargetError::BadDomain);
            };
            let origin = PageOrigin::parse(&format!("https://{host}"))
                .map_err(|_| TargetError::NotAnOrigin)?;
            Ok(web(origin))
        }
        IosIdentifier::Url(u) => {
            let parsed = Url::parse(u.trim()).map_err(|_| TargetError::BadDomain)?;
            if parsed.scheme() != "http" && parsed.scheme() != "https" {
                return Err(TargetError::Scheme);
            }
            let origin = PageOrigin::from_url(&parsed).map_err(|_| TargetError::NotAnOrigin)?;
            Ok(web(origin))
        }
    }
}
