//! The OPAQUE ciphersuite of the sync account login (CRYPTO_SPEC §9.1,
//! ADR-0003, ADR-0046), shared by the server and the HTTP client so that both
//! sides use one definition of every parameter.
//!
//! - ciphersuite: OPRF over `ristretto255`, `TripleDH` over `ristretto255` with
//!   SHA-512, key stretching `Identity` (the OPAQUE password is `OPW`, which
//!   is already Argon2id-stretched and carries the 128-bit Secret Key;
//!   ADR-0003);
//! - context [`LOGIN_CONTEXT`] on login and registration finish;
//! - identifiers: `client = account_id` (16 raw bytes), `server = canonical
//!   origin` of the server, so a login record cannot be replayed against
//!   another server;
//! - credential identifier: the account ID bytes.
//!
//! No cryptography is implemented here: this module only configures
//! `opaque-ke` and supplies its random number generator from the OS CSPRNG
//! through `getrandom` (ADR-0011).

use opaque_ke::ciphersuite::CipherSuite;
use opaque_ke::ksf::Identity;
use opaque_ke::rand::{CryptoRng, Error as RandError, RngCore};
use opaque_ke::{Identifiers, Ristretto255, TripleDh};

/// Re-export of the pinned `opaque-ke`, so client and server name one
/// version.
pub use opaque_ke as ke;

/// The ciphersuite of CRYPTO_SPEC §9.1.
pub struct Suite;

impl CipherSuite for Suite {
    type OprfCs = Ristretto255;
    type KeyExchange = TripleDh<Ristretto255, sha2_0_10::Sha512>;
    type Ksf = Identity;
}

/// The `context` of login and registration finish (CRYPTO_SPEC §9.1).
///
/// `ctx(label::OPAQUE_LOGIN)`, built from the registry of `mosaic-core`.
pub static LOGIN_CONTEXT: std::sync::LazyLock<Vec<u8>> =
    std::sync::LazyLock::new(|| mosaic_core::encoding::label::OPAQUE_LOGIN.ctx());

/// The identifiers of a login or registration: the client is the account ID
/// (16 raw bytes) and the server is its canonical origin
/// (`https://host[:port]`, lowercase, no trailing slash).
#[must_use]
pub const fn identifiers<'a>(account_id: &'a [u8], server_origin: &'a [u8]) -> Identifiers<'a> {
    Identifiers {
        client: Some(account_id),
        server: Some(server_origin),
    }
}

/// The OS CSPRNG as an `opaque-ke` random number generator.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsRng;

impl RngCore for OsRng {
    fn next_u32(&mut self) -> u32 {
        let mut b = [0u8; 4];
        self.fill_bytes(&mut b);
        u32::from_le_bytes(b)
    }

    fn next_u64(&mut self) -> u64 {
        let mut b = [0u8; 8];
        self.fill_bytes(&mut b);
        u64::from_le_bytes(b)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // `RngCore::fill_bytes` cannot fail. There is no fallback generator:
        // if the OS CSPRNG is unavailable the process stops rather than
        // continue with predictable bytes.
        if getrandom::fill(dest).is_err() {
            std::process::abort();
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), RandError> {
        getrandom::fill(dest).map_err(|_| RandError::from(core::num::NonZeroU32::MIN))
    }
}

impl CryptoRng for OsRng {}

#[cfg(test)]
mod tests {
    use opaque_ke::{
        ClientLogin, ClientLoginFinishParameters, ClientRegistration,
        ClientRegistrationFinishParameters, ServerLogin, ServerLoginParameters, ServerRegistration,
        ServerSetup,
    };

    use super::*;

    /// A full registration and login with the parameters of this module,
    /// and a login with another server identity fails.
    #[test]
    fn suite_round_trip_and_server_identity_binding() {
        let mut rng = OsRng;
        let setup = ServerSetup::<Suite>::new(&mut rng);
        let account = [7u8; 16];
        let origin = b"https://sync.example.com";
        let opw = [9u8; 32];

        let start = ClientRegistration::<Suite>::start(&mut rng, &opw).unwrap();
        let resp = ServerRegistration::<Suite>::start(&setup, start.message, &account).unwrap();
        let fin = start
            .state
            .finish(
                &mut rng,
                &opw,
                resp.message,
                ClientRegistrationFinishParameters::new(identifiers(&account, origin), None),
            )
            .unwrap();
        let record = ServerRegistration::<Suite>::finish(fin.message);

        let mut login = |server_origin: &[u8]| {
            let c = ClientLogin::<Suite>::start(&mut rng, &opw).unwrap();
            let s = ServerLogin::start(
                &mut rng,
                &setup,
                Some(record.clone()),
                c.message,
                &account,
                ServerLoginParameters {
                    context: Some(LOGIN_CONTEXT.as_slice()),
                    identifiers: identifiers(&account, server_origin),
                },
            )
            .unwrap();
            let done = c.state.finish(
                &mut rng,
                &opw,
                s.message,
                ClientLoginFinishParameters::new(
                    Some(LOGIN_CONTEXT.as_slice()),
                    identifiers(&account, origin),
                    None,
                ),
            );
            (done, s.state)
        };
        let (ok, state) = login(origin);
        let ok = ok.unwrap();
        let server_key = state
            .finish(
                ok.message,
                ServerLoginParameters {
                    context: Some(LOGIN_CONTEXT.as_slice()),
                    identifiers: identifiers(&account, origin),
                },
            )
            .unwrap();
        assert_eq!(ok.session_key, server_key.session_key);
        let (bad, _) = login(b"https://evil.example.com");
        assert!(bad.is_err());
    }

    /// The generator's three entry points draw fresh bytes from the OS.
    #[test]
    fn os_rng_fills_every_width_from_the_os() {
        let mut rng = OsRng;
        let (first64, second64) = (rng.next_u64(), rng.next_u64());
        assert_ne!(first64, second64);
        let (first32, second32) = (rng.next_u32(), rng.next_u32());
        assert_ne!(first32, second32);
        let mut first_fill = [0u8; 32];
        let mut second_fill = [0u8; 32];
        rng.try_fill_bytes(&mut first_fill).unwrap();
        rng.try_fill_bytes(&mut second_fill).unwrap();
        assert_ne!(first_fill, second_fill);
    }
}
