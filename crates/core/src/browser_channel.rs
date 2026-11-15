//! The local app channels (CRYPTO_SPEC §15, ADR-0036, ADR-0160): a pre-shared
//! key `K_pair`, a mutual HMAC handshake over two fresh nonces, and
//! per-direction XChaCha20-Poly1305 sessions with strict sequence numbers.
//!
//! ```text
//! pairing_id = random(16), K_pair = random(32)        desktop, at approval
//! nonce_h = random(32) host, nonce_d = random(32) desktop     per connection
//! K_auth  = HKDF(K_pair, salt = pairing_id, ctx("browser/auth"), 32)
//! T       = ctx("browser/handshake") || 0x00 || pairing_id || nonce_h || nonce_d
//! proof_d = HMAC-SHA-256(K_auth, T || 0x02)           desktop proves first
//! proof_h = HMAC-SHA-256(K_auth, T || 0x01)
//! okm     = HKDF(K_pair, salt = pairing_id || nonce_h || nonce_d,
//!                ctx("browser/session"), 64)
//! K_h2d = okm[0..32], K_d2h = okm[32..64]
//! frame   = header || ct,  header = 0x01 (version) || 0x01 (XChaCha20-Poly1305) || seq:u64 (BE)
//! ct      = XChaCha20-Poly1305(K_dir, nonce = 0^16 || seq:u64 (BE), plaintext,
//!                              AAD = header || aad("browser/frame", dir:u8, pairing_id))
//! ```
//!
//! `dir` is `0x01` host to desktop and `0x02` desktop to host. Sequence
//! numbers start at 0 per direction; the receiver requires the next one in
//! order. There is no key agreement: the endpoint's OS access control and the
//! user's approval of the pairing authenticate the key (THREAT_MODEL R38,
//! R42, R45). Every failure of the handshake proofs or of a frame is the one
//! error [`Error::Decryption`].
//!
//! The nonce is derived from the sequence number, not drawn at random: a
//! `(key, seq)` pair is used once per direction, because the keys are fresh
//! per connection (they depend on both connection nonces) and the sequence
//! only moves forward (CRYPTO_SPEC §15).
//!
//! # Two channels, one construction
//!
//! The browser channel (`browser/*` labels, the formulas above) and the agent
//! channel (`agent/*` labels, ADR-0160) share every step with a different
//! label set, chosen with [`Channel`]. The labels enter `K_auth`, the
//! handshake transcript `T` (and so both proofs), `okm` and the frame AAD, so
//! a key, a proof or a frame of one channel never validates on the other even
//! when `K_pair`, the pairing ID and the nonces are equal. The channel is
//! fixed by the endpoint a connection arrived on, never by a message field.
//! `proof`, `verify_proof` and `session` are the browser channel; the `_on`
//! forms take the channel.

use core::fmt;

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::aead;
use crate::encoding::{Aad, CTX_PREFIX, Label, label};
use crate::error::{Error, Result};
use crate::kdf::{hkdf_ctx, hkdf_sha256};
use crate::rng;
use crate::secret::{SecretBytes, SecretVec};

/// Length of a pairing ID in bytes.
pub const PAIRING_ID_LEN: usize = 16;
/// Length of `K_pair` in bytes.
pub const PAIRING_KEY_LEN: usize = 32;
/// Length of a per-connection nonce in bytes.
pub const CHANNEL_NONCE_LEN: usize = 32;
/// Length of a handshake proof in bytes.
pub const PROOF_LEN: usize = 32;
/// Length of the frame header in bytes: version, algorithm and `seq:u64`.
pub const FRAME_HEADER_LEN: usize = 10;

/// Frame format version.
const FRAME_VERSION: u8 = 0x01;
/// Algorithm ID of XChaCha20-Poly1305 in a frame header.
const FRAME_ALG_XCHACHA20POLY1305: u8 = 0x01;

/// Which local channel a handshake or session belongs to (CRYPTO_SPEC §15,
/// ADR-0160). Each has its own label set; see the module docs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    /// The browser extension channel: `browser/auth|handshake|session|frame`.
    Browser,
    /// The agent channel: `agent/auth|handshake|session|frame`.
    Agent,
}

/// The four labels of one channel.
struct ChannelLabels {
    auth: Label,
    handshake: Label,
    session: Label,
    frame: Label,
}

impl Channel {
    const fn labels(self) -> ChannelLabels {
        match self {
            Self::Browser => ChannelLabels {
                auth: label::BROWSER_AUTH,
                handshake: label::BROWSER_HANDSHAKE,
                session: label::BROWSER_SESSION,
                frame: label::BROWSER_FRAME,
            },
            Self::Agent => ChannelLabels {
                auth: label::AGENT_AUTH,
                handshake: label::AGENT_HANDSHAKE,
                session: label::AGENT_SESSION,
                frame: label::AGENT_FRAME,
            },
        }
    }
}

/// Which end of the channel a party is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    /// The native messaging host (the connecting side).
    Host,
    /// The desktop app (the serving side).
    Desktop,
}

impl Role {
    /// The byte that ends the proof message and names the direction:
    /// `0x01` host, `0x02` desktop.
    const fn byte(self) -> u8 {
        match self {
            Self::Host => 0x01,
            Self::Desktop => 0x02,
        }
    }

    const fn peer(self) -> Self {
        match self {
            Self::Host => Self::Desktop,
            Self::Desktop => Self::Host,
        }
    }
}

/// A fresh `random(16)` pairing ID. Not secret.
///
/// # Errors
/// [`Error::Randomness`] if the OS CSPRNG fails.
pub fn new_pairing_id() -> Result<[u8; PAIRING_ID_LEN]> {
    rng::array()
}

/// A fresh `random(32)` per-connection nonce. Not secret.
///
/// # Errors
/// [`Error::Randomness`] if the OS CSPRNG fails.
pub fn new_channel_nonce() -> Result<[u8; CHANNEL_NONCE_LEN]> {
    rng::array()
}

/// A fresh `random(16)` token for the name of the Windows pipe. Not secret,
/// but unguessable names are what keeps other accounts from squatting the
/// pipe in advance (THREAT_MODEL R42). Exposed so that
/// `mosaic-native-messaging` needs no CSPRNG of its own (CRYPTO_SPEC §2).
///
/// # Errors
/// [`Error::Randomness`] if the OS CSPRNG fails.
pub fn new_endpoint_token() -> Result<[u8; 16]> {
    rng::array()
}

/// 30 fresh random bits for a pairing code (six Crockford base32 symbols;
/// the caller encodes them). Uniform: the top two bits of a random `u32`
/// are cleared, no modulo bias.
///
/// # Errors
/// [`Error::Randomness`] if the OS CSPRNG fails.
pub fn new_pairing_code_bits() -> Result<u32> {
    Ok(u32::from_be_bytes(rng::array()?) & 0x3fff_ffff)
}

/// `K_pair`: the 256-bit key the desktop draws when the user approves a
/// pairing. Zeroized on drop, not `Clone`, `Display` or `Serialize`.
pub struct PairingKey(SecretBytes<PAIRING_KEY_LEN>);

impl fmt::Debug for PairingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl ConstantTimeEq for PairingKey {
    fn ct_eq(&self, other: &Self) -> subtle::Choice {
        self.0.ct_eq(&other.0)
    }
}

impl PairingKey {
    /// A fresh `random(32)` key, generated in place.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the OS CSPRNG fails.
    pub fn generate() -> Result<Self> {
        Ok(Self(SecretBytes::random()?))
    }

    /// Moves `bytes` into a key and zeroizes the source array.
    #[must_use]
    pub fn from_bytes(bytes: &mut [u8; PAIRING_KEY_LEN]) -> Self {
        Self(SecretBytes::from_array(bytes))
    }

    /// Borrows the key bytes, to store or transfer them at pairing. Keep the
    /// borrow short and never log it.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8; PAIRING_KEY_LEN] {
        self.0.expose_secret()
    }

    /// `K_auth = HKDF(K_pair, salt = pairing_id, ctx("<channel>/auth"), 32)`.
    fn auth_key(
        &self,
        channel: Channel,
        pairing_id: &[u8; PAIRING_ID_LEN],
    ) -> Result<SecretBytes<32>> {
        hkdf_ctx(self.0.expose_secret(), pairing_id, channel.labels().auth)
    }

    /// The HMAC of `T || role` under `K_auth`.
    fn proof_mac(
        &self,
        channel: Channel,
        role: Role,
        pairing_id: &[u8; PAIRING_ID_LEN],
        nonce_h: &[u8; CHANNEL_NONCE_LEN],
        nonce_d: &[u8; CHANNEL_NONCE_LEN],
    ) -> Result<Hmac<Sha256>> {
        let k_auth = self.auth_key(channel, pairing_id)?;
        let transcript = Aad::with_header(&[], channel.labels().handshake)
            .fixed(pairing_id)
            .fixed(nonce_h)
            .fixed(nonce_d);
        let mut mac = <Hmac<Sha256> as KeyInit>::new_from_slice(k_auth.expose_secret())
            .map_err(|_| Error::SizeLimit)?;
        mac.update(transcript.as_bytes());
        mac.update(&[role.byte()]);
        Ok(mac)
    }

    /// The proof of `role` over this connection's transcript on the browser
    /// channel: `proof_h` for [`Role::Host`], `proof_d` for
    /// [`Role::Desktop`]. The desktop sends its proof first.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if a derivation fails (it cannot for these sizes).
    pub fn proof(
        &self,
        role: Role,
        pairing_id: &[u8; PAIRING_ID_LEN],
        nonce_h: &[u8; CHANNEL_NONCE_LEN],
        nonce_d: &[u8; CHANNEL_NONCE_LEN],
    ) -> Result<[u8; PROOF_LEN]> {
        self.proof_on(Channel::Browser, role, pairing_id, nonce_h, nonce_d)
    }

    /// [`PairingKey::proof`] on `channel`.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if a derivation fails (it cannot for these sizes).
    pub fn proof_on(
        &self,
        channel: Channel,
        role: Role,
        pairing_id: &[u8; PAIRING_ID_LEN],
        nonce_h: &[u8; CHANNEL_NONCE_LEN],
        nonce_d: &[u8; CHANNEL_NONCE_LEN],
    ) -> Result<[u8; PROOF_LEN]> {
        Ok(self
            .proof_mac(channel, role, pairing_id, nonce_h, nonce_d)?
            .finalize()
            .into_bytes()
            .into())
    }

    /// Checks `proof` as the browser channel proof of `role`, in constant
    /// time. A proof of the wrong length is not valid.
    ///
    /// # Errors
    /// [`Error::Decryption`] if the proof is not valid.
    pub fn verify_proof(
        &self,
        role: Role,
        pairing_id: &[u8; PAIRING_ID_LEN],
        nonce_h: &[u8; CHANNEL_NONCE_LEN],
        nonce_d: &[u8; CHANNEL_NONCE_LEN],
        proof: &[u8],
    ) -> Result<()> {
        self.verify_proof_on(Channel::Browser, role, pairing_id, nonce_h, nonce_d, proof)
    }

    /// [`PairingKey::verify_proof`] on `channel`.
    ///
    /// # Errors
    /// [`Error::Decryption`] if the proof is not valid.
    pub fn verify_proof_on(
        &self,
        channel: Channel,
        role: Role,
        pairing_id: &[u8; PAIRING_ID_LEN],
        nonce_h: &[u8; CHANNEL_NONCE_LEN],
        nonce_d: &[u8; CHANNEL_NONCE_LEN],
        proof: &[u8],
    ) -> Result<()> {
        let expected = self.proof_on(channel, role, pairing_id, nonce_h, nonce_d)?;
        if bool::from(expected.as_slice().ct_eq(proof)) {
            Ok(())
        } else {
            Err(Error::Decryption)
        }
    }

    /// The browser channel session keys of this connection, as seen by `role`
    /// (the caller's own role): it sends under its own direction key and
    /// opens under the peer's. Call it only after both proofs verified.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the derivation fails (it cannot for 64 bytes).
    pub fn session(
        &self,
        role: Role,
        pairing_id: &[u8; PAIRING_ID_LEN],
        nonce_h: &[u8; CHANNEL_NONCE_LEN],
        nonce_d: &[u8; CHANNEL_NONCE_LEN],
    ) -> Result<ChannelSession> {
        self.session_on(Channel::Browser, role, pairing_id, nonce_h, nonce_d)
    }

    /// [`PairingKey::session`] on `channel`. The session seals and opens
    /// frames under the channel's frame label.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] if the derivation fails (it cannot for 64 bytes).
    pub fn session_on(
        &self,
        channel: Channel,
        role: Role,
        pairing_id: &[u8; PAIRING_ID_LEN],
        nonce_h: &[u8; CHANNEL_NONCE_LEN],
        nonce_d: &[u8; CHANNEL_NONCE_LEN],
    ) -> Result<ChannelSession> {
        let (k_h2d, k_d2h) = self.session_keys(channel, pairing_id, nonce_h, nonce_d)?;
        let (send_key, recv_key) = match role {
            Role::Host => (k_h2d, k_d2h),
            Role::Desktop => (k_d2h, k_h2d),
        };
        Ok(ChannelSession {
            send_key,
            recv_key,
            send_dir: role.byte(),
            recv_dir: role.peer().byte(),
            pairing_id: *pairing_id,
            frame_label: channel.labels().frame,
            send_seq: 0,
            recv_seq: 0,
            poisoned: false,
        })
    }

    /// `okm = HKDF(K_pair, pairing_id || nonce_h || nonce_d,
    /// ctx("<channel>/session"), 64)` split into `(K_h2d, K_d2h)`.
    fn session_keys(
        &self,
        channel: Channel,
        pairing_id: &[u8; PAIRING_ID_LEN],
        nonce_h: &[u8; CHANNEL_NONCE_LEN],
        nonce_d: &[u8; CHANNEL_NONCE_LEN],
    ) -> Result<(SecretBytes<32>, SecretBytes<32>)> {
        let mut salt = [0u8; PAIRING_ID_LEN + 2 * CHANNEL_NONCE_LEN];
        let (id_part, nonces) = salt.split_at_mut(PAIRING_ID_LEN);
        let (h_part, d_part) = nonces.split_at_mut(CHANNEL_NONCE_LEN);
        id_part.copy_from_slice(pairing_id);
        h_part.copy_from_slice(nonce_h);
        d_part.copy_from_slice(nonce_d);
        let mut okm = SecretBytes::<64>::zero();
        hkdf_sha256(
            self.0.expose_secret(),
            &salt,
            &[CTX_PREFIX.as_bytes(), channel.labels().session.as_bytes()],
            okm.expose_secret_mut(),
        )?;
        let (left, right) = okm.expose_secret().split_at(32);
        Ok((
            SecretBytes::from_slice(left)?,
            SecretBytes::from_slice(right)?,
        ))
    }
}

/// The two directions of an authenticated connection: AEAD frames with
/// strict, gap-free sequence numbers. Not `Clone`; keys are zeroized on drop.
///
/// A frame that fails to open poisons the session: from then on `seal` and
/// `open` fail with [`Error::Decryption`]. The connection is dead and the
/// caller must close it (CRYPTO_SPEC §15).
pub struct ChannelSession {
    send_key: SecretBytes<32>,
    recv_key: SecretBytes<32>,
    send_dir: u8,
    recv_dir: u8,
    pairing_id: [u8; PAIRING_ID_LEN],
    /// The channel's frame label, in every frame's AAD.
    frame_label: Label,
    send_seq: u64,
    recv_seq: u64,
    /// Set by the first failed `open`; never cleared.
    poisoned: bool,
}

impl fmt::Debug for ChannelSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

/// `header = version || alg || seq:u64 (BE)`.
fn frame_header(seq: u64) -> [u8; FRAME_HEADER_LEN] {
    let [s0, s1, s2, s3, s4, s5, s6, s7] = seq.to_be_bytes();
    [
        FRAME_VERSION,
        FRAME_ALG_XCHACHA20POLY1305,
        s0,
        s1,
        s2,
        s3,
        s4,
        s5,
        s6,
        s7,
    ]
}

/// The 24-byte nonce `0^16 || seq:u64 (BE)`.
fn frame_nonce(seq: u64) -> [u8; aead::NONCE_LEN] {
    let mut nonce = [0u8; aead::NONCE_LEN];
    let (_, tail) = nonce.split_at_mut(aead::NONCE_LEN - 8);
    tail.copy_from_slice(&seq.to_be_bytes());
    nonce
}

/// `header || aad("<channel>/frame", dir, pairing_id)`.
fn frame_aad(header: &[u8], frame_label: Label, dir: u8, pairing_id: &[u8; PAIRING_ID_LEN]) -> Aad {
    Aad::with_header(header, frame_label)
        .u8(dir)
        .fixed(pairing_id)
}

impl ChannelSession {
    /// Seals `plaintext` as the next frame of the sending direction:
    /// `header || ct || tag`. The sequence number advances only on success.
    ///
    /// # Errors
    /// [`Error::Decryption`] if the session is poisoned by a failed `open`;
    /// [`Error::SizeLimit`] if the sequence number is exhausted (`u64::MAX`)
    /// or the plaintext is too large; [`Error::Allocation`] if the output
    /// buffer cannot be reserved.
    pub fn seal(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        if self.poisoned {
            return Err(Error::Decryption);
        }
        let seq = self.send_seq;
        if seq == u64::MAX {
            return Err(Error::SizeLimit);
        }
        let header = frame_header(seq);
        let aad = frame_aad(&header, self.frame_label, self.send_dir, &self.pairing_id);
        let frame = aead::seal_with_prefix(
            self.send_key.expose_secret(),
            &frame_nonce(seq),
            aad.as_bytes(),
            &[&header],
            plaintext,
        )?;
        self.send_seq = seq.checked_add(1).ok_or(Error::SizeLimit)?;
        Ok(frame)
    }

    /// Opens the next frame of the receiving direction. The sequence number
    /// advances only when the frame authenticates. Any failure poisons the
    /// session: every later `seal` and `open` fails, so a caller that
    /// ignores the error cannot go on using a connection that an attacker
    /// has tampered with.
    ///
    /// # Errors
    /// [`Error::Decryption`] for any failure: a poisoned session, a short
    /// frame, an unknown version or algorithm, a sequence number other than
    /// the expected one, a bad tag, the wrong key or direction.
    pub fn open(&mut self, frame: &[u8]) -> Result<SecretVec> {
        if self.poisoned {
            return Err(Error::Decryption);
        }
        let opened = self.open_next(frame);
        if opened.is_err() {
            self.poisoned = true;
        }
        opened
    }

    fn open_next(&mut self, frame: &[u8]) -> Result<SecretVec> {
        let seq = self.recv_seq;
        let (header, ciphertext) = frame
            .split_first_chunk::<FRAME_HEADER_LEN>()
            .ok_or(Error::Decryption)?;
        // A frame for `seq == u64::MAX` cannot authenticate: `seal` stops
        // before that number, so nobody holding the key ever sealed one.
        if *header != frame_header(seq) {
            return Err(Error::Decryption);
        }
        let aad = frame_aad(header, self.frame_label, self.recv_dir, &self.pairing_id);
        let plaintext = aead::open(
            self.recv_key.expose_secret(),
            &frame_nonce(seq),
            aad.as_bytes(),
            ciphertext,
        )?;
        self.recv_seq = seq.checked_add(1).ok_or(Error::Decryption)?;
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    fn bytes<const N: usize>(start: u8) -> [u8; N] {
        let mut out = [0u8; N];
        for (i, b) in out.iter_mut().enumerate() {
            *b = start.wrapping_add(u8::try_from(i).unwrap());
        }
        out
    }

    /// The fixed inputs of the golden vector (see the script below).
    fn fixed() -> (PairingKey, [u8; 16], [u8; 32], [u8; 32]) {
        (
            PairingKey::from_bytes(&mut bytes(0x00)),
            bytes(0x40),
            bytes(0x60),
            bytes(0x80),
        )
    }

    /// Both ends of one connection from the fixed inputs.
    fn pair() -> (ChannelSession, ChannelSession) {
        let (key, id, nh, nd) = fixed();
        (
            key.session(Role::Host, &id, &nh, &nd).unwrap(),
            key.session(Role::Desktop, &id, &nh, &nd).unwrap(),
        )
    }

    /// Golden vector for CRYPTO_SPEC §15 with `K_pair = 00..1f`,
    /// `pairing_id = 40..4f`, `nonce_h = 60..7f`, `nonce_d = 80..9f`. The
    /// HKDF and HMAC values come from this script, which uses only the
    /// Python standard library and writes HKDF out per RFC 5869:
    ///
    /// ```python
    /// import hashlib, hmac
    ///
    /// def hkdf(ikm, salt, info, n):
    ///     prk = hmac.new(salt if salt else b"\0" * 32, ikm, hashlib.sha256).digest()
    ///     okm, t, i = b"", b"", 1
    ///     while len(okm) < n:
    ///         t = hmac.new(prk, t + info + bytes([i]), hashlib.sha256).digest()
    ///         okm += t
    ///         i += 1
    ///     return okm[:n]
    ///
    /// k_pair = bytes(range(0x00, 0x20))
    /// pairing_id = bytes(range(0x40, 0x50))
    /// nonce_h = bytes(range(0x60, 0x80))
    /// nonce_d = bytes(range(0x80, 0xA0))
    ///
    /// k_auth = hkdf(k_pair, pairing_id, b"mosaic/v1/browser/auth", 32)
    /// t = b"mosaic/v1/browser/handshake" + b"\x00" + pairing_id + nonce_h + nonce_d
    /// proof_h = hmac.new(k_auth, t + b"\x01", hashlib.sha256).digest()
    /// proof_d = hmac.new(k_auth, t + b"\x02", hashlib.sha256).digest()
    /// okm = hkdf(k_pair, pairing_id + nonce_h + nonce_d, b"mosaic/v1/browser/session", 64)
    /// for name, v in [("k_auth", k_auth), ("proof_h", proof_h), ("proof_d", proof_d),
    ///                 ("k_h2d", okm[:32]), ("k_d2h", okm[32:])]:
    ///     print(name, v.hex())
    /// ```
    ///
    /// The two sealed frames are pinned from this implementation (there is
    /// no standard-library XChaCha20-Poly1305 to cross-check them); the
    /// cipher itself has its own known-answer test in `aead`.
    #[test]
    fn golden_vector() {
        let (key, id, nh, nd) = fixed();
        let k_auth = key.auth_key(Channel::Browser, &id).unwrap();
        assert_eq!(
            k_auth.expose_secret(),
            &hex!("481e75215802eaf13ef670c48cf5cb9a49731ce0f125ae9e2b3ed0581c8e9ffe")
        );
        assert_eq!(
            key.proof(Role::Host, &id, &nh, &nd).unwrap(),
            hex!("53a258195dba062ff5bcf445ad0da07b4a2d78522454fd75e7cdf0aacd491f26")
        );
        assert_eq!(
            key.proof(Role::Desktop, &id, &nh, &nd).unwrap(),
            hex!("5dd54627cfd11f284be710ea068c1d50c8cbfdd8d9d4a21c9d9f02f6feb132c6")
        );
        let (k_h2d, k_d2h) = key.session_keys(Channel::Browser, &id, &nh, &nd).unwrap();
        assert_eq!(
            k_h2d.expose_secret(),
            &hex!("6abb259740161f4a368d47d3c0d7af9b4696d46844a43b4b9b52e6d5c3545b03")
        );
        assert_eq!(
            k_d2h.expose_secret(),
            &hex!("5de6dc2606299306baeb1192c78818be134534b87abb3bb3b9043ed50e243930")
        );

        let (mut host, mut desktop) = pair();
        let h2d = host.seal(b"hello from the host").unwrap();
        let d2h = desktop.seal(b"hello from the desktop").unwrap();
        assert_eq!(h2d.as_slice(), GOLDEN_H2D);
        assert_eq!(d2h.as_slice(), GOLDEN_D2H);
        assert_eq!(
            desktop.open(&h2d).unwrap().expose_secret(),
            b"hello from the host"
        );
        assert_eq!(
            host.open(&d2h).unwrap().expose_secret(),
            b"hello from the desktop"
        );
    }

    const GOLDEN_H2D: &[u8] = &hex!(
        "01010000000000000000c9352957580de242848c152930548d9ffd00b781d1406e5decca24a4fbc43cb09c8ca1"
    );
    const GOLDEN_D2H: &[u8] = &hex!(
        "01010000000000000000786d9c1db5655f3d021408f5a90d749fdc4a616a68743070c6e0d1577e218109737bc3c73385"
    );

    /// Agent channel vector: the inputs of [`golden_vector`] with the
    /// `agent/*` labels. Same Python script with `browser` replaced by
    /// `agent` in the three labels `agent/auth`, `agent/handshake` and
    /// `agent/session`.
    #[test]
    fn agent_labels_give_a_different_vector() {
        let (key, id, nh, nd) = fixed();
        let on = Channel::Agent;
        assert_eq!(
            key.auth_key(on, &id).unwrap().expose_secret(),
            &hex!("25538b1b5cc96a7ff57ecacab49068ac1e47ecbe9ce257afe168561e939385d0")
        );
        assert_eq!(
            key.proof_on(on, Role::Host, &id, &nh, &nd).unwrap(),
            hex!("36933b26777d0f18c5c9ed37458ac54f3dea13d61726ea3a935b4152524c3ac0")
        );
        assert_eq!(
            key.proof_on(on, Role::Desktop, &id, &nh, &nd).unwrap(),
            hex!("684a097452becbe55428a5ef52d8149d66f29686c24f9cbc8e54cfd64f70eefe")
        );
        let (k_h2d, k_d2h) = key.session_keys(on, &id, &nh, &nd).unwrap();
        assert_eq!(
            k_h2d.expose_secret(),
            &hex!("49a45a500c5114ebc35ac1f6257a99e4ec97af9ef142b4f9e6ae895fc94bb82f")
        );
        assert_eq!(
            k_d2h.expose_secret(),
            &hex!("336e898e4f4d3ed4d1c1fbc23a1ef383e849b0cbf153508ae31d30034bdb66e8")
        );
        assert_ne!(
            key.proof_on(on, Role::Desktop, &id, &nh, &nd).unwrap(),
            key.proof_on(Channel::Browser, Role::Desktop, &id, &nh, &nd)
                .unwrap()
        );
    }

    #[test]
    fn browser_default_forms_are_the_browser_channel() {
        let (key, id, nh, nd) = fixed();
        assert_eq!(
            key.proof(Role::Host, &id, &nh, &nd).unwrap(),
            key.proof_on(Channel::Browser, Role::Host, &id, &nh, &nd)
                .unwrap()
        );
        let mut a = key.session(Role::Host, &id, &nh, &nd).unwrap();
        let mut b = key
            .session_on(Channel::Browser, Role::Desktop, &id, &nh, &nd)
            .unwrap();
        let frame = a.seal(b"x").unwrap();
        assert_eq!(b.open(&frame).unwrap().expose_secret(), b"x");
    }

    #[test]
    fn proofs_do_not_cross_channels_in_either_direction() {
        let (key, id, nh, nd) = fixed();
        for role in [Role::Host, Role::Desktop] {
            let browser = key.proof(role, &id, &nh, &nd).unwrap();
            let agent = key.proof_on(Channel::Agent, role, &id, &nh, &nd).unwrap();
            assert_eq!(
                key.verify_proof_on(Channel::Agent, role, &id, &nh, &nd, &browser),
                Err(Error::Decryption)
            );
            assert_eq!(
                key.verify_proof_on(Channel::Browser, role, &id, &nh, &nd, &agent),
                Err(Error::Decryption)
            );
            key.verify_proof_on(Channel::Agent, role, &id, &nh, &nd, &agent)
                .unwrap();
        }
    }

    #[test]
    fn frames_do_not_cross_channels_in_either_direction() {
        let (key, id, nh, nd) = fixed();
        let agent = |role| key.session_on(Channel::Agent, role, &id, &nh, &nd).unwrap();
        let browser = |role| {
            key.session_on(Channel::Browser, role, &id, &nh, &nd)
                .unwrap()
        };
        // Same K_pair, pairing ID and nonces: only the labels differ.
        let frame = agent(Role::Host).seal(b"agent frame").unwrap();
        assert_eq!(
            browser(Role::Desktop).open(&frame).err(),
            Some(Error::Decryption)
        );
        let frame = browser(Role::Host).seal(b"browser frame").unwrap();
        assert_eq!(
            agent(Role::Desktop).open(&frame).err(),
            Some(Error::Decryption)
        );
        let frame = agent(Role::Host).seal(b"agent frame").unwrap();
        assert_eq!(
            agent(Role::Desktop).open(&frame).unwrap().expose_secret(),
            b"agent frame"
        );
        assert_ne!(
            agent(Role::Host).seal(b"same").unwrap(),
            browser(Role::Host).seal(b"same").unwrap()
        );
    }

    #[test]
    fn proofs_verify_and_nothing_else_does() {
        let (key, id, nh, nd) = fixed();
        let ph = key.proof(Role::Host, &id, &nh, &nd).unwrap();
        let pd = key.proof(Role::Desktop, &id, &nh, &nd).unwrap();
        assert_ne!(ph, pd, "the roles prove different messages");
        key.verify_proof(Role::Host, &id, &nh, &nd, &ph).unwrap();
        key.verify_proof(Role::Desktop, &id, &nh, &nd, &pd).unwrap();
        let bad = |r: Result<()>| assert_eq!(r, Err(Error::Decryption));
        bad(key.verify_proof(Role::Host, &id, &nh, &nd, &pd));
        bad(key.verify_proof(Role::Desktop, &id, &nh, &nd, &ph));
        bad(key.verify_proof(Role::Host, &id, &nh, &nd, &ph[..31]));
        bad(key.verify_proof(Role::Host, &id, &nh, &nd, &[ph.as_slice(), &[0]].concat()));
        bad(key.verify_proof(Role::Host, &id, &nh, &nd, &[]));
        bad(key.verify_proof(Role::Host, &[0; 16], &nh, &nd, &ph));
        bad(key.verify_proof(Role::Host, &id, &[0; 32], &nd, &ph));
        bad(key.verify_proof(Role::Host, &id, &nh, &[0; 32], &ph));
        let other = PairingKey::from_bytes(&mut [9u8; 32]);
        bad(other.verify_proof(Role::Host, &id, &nh, &nd, &ph));
        // Swapping the nonces is a different transcript.
        bad(key.verify_proof(Role::Host, &id, &nd, &nh, &ph));
    }

    #[test]
    fn round_trip_in_order_over_many_frames() {
        let (mut host, mut desktop) = pair();
        for i in 0..20u8 {
            let msg = vec![i; usize::from(i) * 3];
            let frame = host.seal(&msg).unwrap();
            assert_eq!(frame.len(), FRAME_HEADER_LEN + msg.len() + 16);
            assert_eq!(desktop.open(&frame).unwrap().expose_secret(), &msg);
            let back = desktop.seal(&msg).unwrap();
            assert_eq!(host.open(&back).unwrap().expose_secret(), &msg);
        }
    }

    #[test]
    fn tampering_with_any_part_is_refused() {
        let (mut host, _) = pair();
        let frame = host.seal(b"payload").unwrap();
        for i in 0..frame.len() {
            let mut bad = frame.clone();
            bad[i] ^= 0x01;
            let (_, mut fresh) = pair();
            assert_eq!(fresh.open(&bad).unwrap_err(), Error::Decryption, "byte {i}");
        }
        // Every rejected frame poisons the session it was offered to, so
        // each rejection below runs on a fresh one.
        let rejects = |bad: &[u8]| {
            let (_, mut fresh) = pair();
            assert_eq!(fresh.open(bad).unwrap_err(), Error::Decryption);
        };
        rejects(&frame[..frame.len() - 1]);
        rejects(&[]);
        rejects(&frame[..9]);
        rejects(&frame[..FRAME_HEADER_LEN]);
        let (_, mut fresh) = pair();
        assert_eq!(fresh.open(&frame).unwrap().expose_secret(), b"payload");
    }

    #[test]
    fn a_failed_open_poisons_the_session() {
        let (mut host, mut desktop) = pair();
        let good = host.seal(b"first").unwrap();
        let mut bad = good.clone();
        bad[FRAME_HEADER_LEN] ^= 1;
        assert_eq!(desktop.open(&bad).unwrap_err(), Error::Decryption);
        // The genuine frame no longer opens and nothing can be sealed.
        assert_eq!(desktop.open(&good).unwrap_err(), Error::Decryption);
        assert_eq!(desktop.seal(b"reply").unwrap_err(), Error::Decryption);
        assert_eq!(desktop.open(&[]).unwrap_err(), Error::Decryption);
        // A session that never failed is unaffected, and so is the peer's
        // sending half.
        assert_eq!(host.seal(b"second").unwrap().len(), 10 + 6 + 16);
        let (_, mut other) = pair();
        assert_eq!(other.open(&good).unwrap().expose_secret(), b"first");
    }

    #[test]
    fn a_frame_does_not_open_in_the_wrong_direction() {
        let (mut host, _) = pair();
        let mut host_twin = pair().0;
        let frame = host.seal(b"x").unwrap();
        assert_eq!(host_twin.open(&frame).unwrap_err(), Error::Decryption);
        // Same key by construction, flipped direction byte: rebuild a session
        // that sends under K_h2d but claims the desktop's direction.
        let (key, id, nh, nd) = fixed();
        let mut forged = key.session(Role::Host, &id, &nh, &nd).unwrap();
        forged.send_dir = Role::Desktop.byte();
        let frame = forged.seal(b"x").unwrap();
        let (_, mut desktop) = pair();
        assert_eq!(desktop.open(&frame).unwrap_err(), Error::Decryption);
    }

    #[test]
    fn sequence_numbers_are_strict() {
        let (mut host, _) = pair();
        let (f0, f1, f2) = (
            host.seal(b"0").unwrap(),
            host.seal(b"1").unwrap(),
            host.seal(b"2").unwrap(),
        );
        let fresh = || pair().1;
        assert_eq!(fresh().open(&f1).unwrap_err(), Error::Decryption, "skip");
        assert_eq!(fresh().open(&f2).unwrap_err(), Error::Decryption, "gap");
        let mut desktop = fresh();
        desktop.open(&f0).unwrap();
        assert_eq!(desktop.open(&f0).unwrap_err(), Error::Decryption, "replay");
        let mut desktop = fresh();
        desktop.open(&f0).unwrap();
        desktop.open(&f1).unwrap();
        desktop.open(&f2).unwrap();
    }

    #[test]
    fn frames_do_not_cross_connections_or_pairings() {
        let (key, id, nh, nd) = fixed();
        let mut host = key.session(Role::Host, &id, &nh, &nd).unwrap();
        let frame = host.seal(b"x").unwrap();
        let mut other_conn = key.session(Role::Desktop, &id, &nh, &[7u8; 32]).unwrap();
        assert_eq!(other_conn.open(&frame).unwrap_err(), Error::Decryption);
        let mut other_id = key.session(Role::Desktop, &[1u8; 16], &nh, &nd).unwrap();
        assert_eq!(other_id.open(&frame).unwrap_err(), Error::Decryption);
        let wrong_key = PairingKey::from_bytes(&mut [5u8; 32]);
        let mut other_key = wrong_key.session(Role::Desktop, &id, &nh, &nd).unwrap();
        assert_eq!(other_key.open(&frame).unwrap_err(), Error::Decryption);
        let mut wrong_role = key.session(Role::Host, &id, &nh, &nd).unwrap();
        assert_eq!(wrong_role.open(&frame).unwrap_err(), Error::Decryption);
    }

    #[test]
    fn unknown_version_and_algorithm_are_refused() {
        let (mut host, _) = pair();
        let frame = host.seal(b"x").unwrap();
        for (i, v) in [(0usize, 2u8), (1, 2), (0, 0), (1, 0)] {
            let mut bad = frame.clone();
            bad[i] = v;
            let (_, mut desktop) = pair();
            assert_eq!(desktop.open(&bad).unwrap_err(), Error::Decryption);
        }
    }

    #[test]
    fn sequence_exhaustion_stops_the_sender() {
        let (mut host, mut desktop) = pair();
        host.send_seq = u64::MAX;
        assert_eq!(host.seal(b"x").unwrap_err(), Error::SizeLimit);
        // A receiver at the last number cannot be fed: no valid frame
        // exists for it.
        desktop.recv_seq = u64::MAX;
        assert_eq!(desktop.open(&[0u8; 40]).unwrap_err(), Error::Decryption);
        // The last usable number works and then the sender is spent.
        let (mut host, mut desktop) = pair();
        host.send_seq = u64::MAX - 1;
        desktop.recv_seq = u64::MAX - 1;
        let frame = host.seal(b"last").unwrap();
        assert_eq!(desktop.open(&frame).unwrap().expose_secret(), b"last");
        assert_eq!(host.seal(b"x").unwrap_err(), Error::SizeLimit);
    }

    #[test]
    fn pairing_keys_compare_in_constant_time() {
        let a = PairingKey::from_bytes(&mut [1u8; 32]);
        let same = PairingKey::from_bytes(&mut [1u8; 32]);
        let mut flipped = [1u8; 32];
        flipped[31] ^= 1;
        let other = PairingKey::from_bytes(&mut flipped);
        assert!(bool::from(a.ct_eq(&same)));
        assert!(!bool::from(a.ct_eq(&other)));
        assert!(!bool::from(other.ct_eq(&same)));
    }

    #[test]
    fn endpoint_tokens_and_code_bits_are_random_and_bounded() {
        assert_ne!(new_endpoint_token().unwrap(), new_endpoint_token().unwrap());
        let bits: Vec<u32> = (0..64).map(|_| new_pairing_code_bits().unwrap()).collect();
        assert!(bits.iter().all(|b| *b < 1 << 30));
        assert!(bits.iter().any(|b| *b != bits[0]));
    }

    #[test]
    fn debug_is_redacted_and_keys_are_not_the_pairing_key() {
        let (key, id, nh, nd) = fixed();
        assert_eq!(format!("{key:?}"), "[REDACTED]");
        let session = key.session(Role::Host, &id, &nh, &nd).unwrap();
        assert_eq!(format!("{session:?}"), "[REDACTED]");
        assert_ne!(session.send_key.expose_secret(), key.expose_secret());
        assert_ne!(session.recv_key.expose_secret(), key.expose_secret());
        assert_ne!(session.send_key, session.recv_key);
    }

    #[test]
    fn generation_is_random_and_from_bytes_wipes_the_source() {
        let (a, b) = (
            PairingKey::generate().unwrap(),
            PairingKey::generate().unwrap(),
        );
        assert_ne!(a.expose_secret(), b.expose_secret());
        assert_ne!(new_pairing_id().unwrap(), new_pairing_id().unwrap());
        assert_ne!(new_channel_nonce().unwrap(), new_channel_nonce().unwrap());
        let mut src = [3u8; 32];
        let key = PairingKey::from_bytes(&mut src);
        assert_eq!(src, [0u8; 32]);
        assert_eq!(key.expose_secret(), &[3u8; 32]);
    }

    #[test]
    fn header_and_nonce_layout() {
        assert_eq!(
            frame_header(0x0102_0304_0506_0708),
            [1, 1, 1, 2, 3, 4, 5, 6, 7, 8]
        );
        let nonce = frame_nonce(0x0102_0304_0506_0708);
        assert_eq!(&nonce[..16], &[0u8; 16]);
        assert_eq!(&nonce[16..], &[1, 2, 3, 4, 5, 6, 7, 8]);
        let aad = frame_aad(&[0xAA], label::BROWSER_FRAME, 2, &[7u8; 16]);
        let mut expected = vec![0xAA];
        expected.extend_from_slice(b"mosaic/v1/browser/frame\x00\x02");
        expected.extend_from_slice(&[7u8; 16]);
        assert_eq!(aad.as_bytes(), expected.as_slice());
    }
}
