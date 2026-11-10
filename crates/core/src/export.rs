//! The encrypted export container (CRYPTO_SPEC §11.2, ADR-0073).
//!
//! A passphrase-protected stream: Argon2id (§5.3, at least the standard
//! profile), HKDF-SHA-256 and the STREAM-BE32-XChaCha20-Poly1305 chunks of
//! §7.3. This module knows only the container; the JSON inside is built and
//! read by `mosaic-vault` and `mosaic-import`.
//!
//! ```text
//! header = "MOSX" || version:u8 (1) || alg:u8 (2) || chunk_log2:u8 (16) || profile:u8
//!          || salt[16] || export_id[16] || nonce_prefix[19]                     (59 bytes)
//! P_x    = Argon2id(NFC(passphrase), salt, profile)
//! K_x    = HKDF(P_x, salt = export_id, info = ctx("export-key"), 32)
//! chunks : §7.3 layout and nonces under K_x, aad = header || aad("export", export_id)
//! ```
//!
//! STREAM authenticates each chunk when it is read. A reader therefore
//! yields chunks that were verified one by one, but a consumer must not act
//! on any of them until [`ExportReader::next_chunk`] has returned `Ok(None)`
//! (the verified last chunk): a stream cut at a chunk boundary fails only at
//! its final chunk, and an error is returned again on every later call.

use core::fmt;
use std::io::{self, Read, Write};

use aead_stream::{NewStream, StreamBE32, StreamPrimitive};
use chacha20poly1305::XChaCha20Poly1305;
use chacha20poly1305::aead::KeyInit;
use unicode_normalization::UnicodeNormalization;

use crate::aead::TAG_LEN;
use crate::attachment::{
    ALG_STREAM_BE32_XCHACHA20_POLY1305, CHUNK_LEN, CHUNK_LOG2, NONCE_PREFIX_LEN,
};
use crate::encoding::{Aad, label, take};
use crate::envelope::FORMAT_VERSION;
use crate::error::{Error, Result};
use crate::kdf::{Argon2Cost, Argon2Profile, argon2id, hkdf_ctx};
use crate::rng;
use crate::secret::{SecretBytes, SecretString, SecretVec};

/// The plaintext limit of one export: 2 GiB (§11.2).
pub const MAX_PLAINTEXT_LEN: u64 = 2 * 1024 * 1024 * 1024;
/// Length of the cleartext header.
pub const HEADER_LEN: usize = 4 + 1 + 1 + 1 + 1 + 16 + 16 + NONCE_PREFIX_LEN;
/// The weakest Argon2id profile an export may use (§11.2).
pub const MIN_PROFILE: Argon2Profile = Argon2Profile::Standard;

const MAGIC: [u8; 4] = *b"MOSX";
const CHUNK_CT_LEN: usize = CHUNK_LEN + TAG_LEN;

type Stream = StreamBE32<XChaCha20Poly1305>;

fn io_err(e: &io::Error) -> Error {
    Error::Io(e.kind())
}

/// The export passphrase: `NFC(passphrase)` as UTF-8 in zeroizing memory
/// (§5.1, §11.2). It is a secret of its own, never the master password.
/// Not `Clone`; prints as `[REDACTED]`.
pub struct ExportPassphrase(SecretString);

impl fmt::Debug for ExportPassphrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl ExportPassphrase {
    /// Prepares a passphrase. The caller keeps (and zeroizes) its own copy;
    /// the strength rule of §5.1 is the client's, the crypto layer rejects
    /// only an empty passphrase.
    ///
    /// # Errors
    /// [`Error::EmptyPassword`]; [`Error::Allocation`] / [`Error::SizeLimit`].
    pub fn new(passphrase: &SecretString) -> Result<Self> {
        let p = passphrase.expose_secret();
        if p.is_empty() {
            return Err(Error::EmptyPassword);
        }
        // NFC expands UTF-8 by at most 3x; reserving it up front means no
        // reallocation leaves a stray copy.
        let cap = p.len().checked_mul(3).ok_or(Error::SizeLimit)?;
        let mut out = SecretString::with_capacity(cap)?;
        for c in p.nfc() {
            out.push(c);
        }
        Ok(Self(out))
    }
}

/// The parsed, validated cleartext header.
struct Header {
    raw: [u8; HEADER_LEN],
    profile: Argon2Profile,
    salt: [u8; 16],
    export_id: [u8; 16],
    nonce_prefix: [u8; NONCE_PREFIX_LEN],
}

impl Header {
    fn encode(
        profile_id: u8,
        salt: &[u8; 16],
        export_id: &[u8; 16],
        nonce_prefix: &[u8; NONCE_PREFIX_LEN],
    ) -> Result<[u8; HEADER_LEN]> {
        let mut out = Vec::with_capacity(HEADER_LEN);
        out.extend_from_slice(&MAGIC);
        out.push(FORMAT_VERSION);
        out.push(ALG_STREAM_BE32_XCHACHA20_POLY1305);
        out.push(CHUNK_LOG2);
        out.push(profile_id);
        out.extend_from_slice(salt);
        out.extend_from_slice(export_id);
        out.extend_from_slice(nonce_prefix);
        out.try_into().map_err(|_| Error::Malformed)
    }

    /// Reads and validates the header, before any key stretching: unknown
    /// versions, algorithms and profiles (and the baseline profile) never
    /// cost the reader an Argon2id run.
    fn read<R: Read>(r: &mut R) -> Result<Self> {
        let mut raw = [0u8; HEADER_LEN];
        r.read_exact(&mut raw).map_err(|e| {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                Error::Malformed
            } else {
                io_err(&e)
            }
        })?;
        let (magic, rest) = take::<4>(&raw)?;
        let ([version, alg, chunk_log2, profile], rest) = take::<4>(rest)?;
        if *magic != MAGIC {
            return Err(Error::Malformed);
        }
        if *version != FORMAT_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        if *alg != ALG_STREAM_BE32_XCHACHA20_POLY1305 || *chunk_log2 != CHUNK_LOG2 {
            return Err(Error::UnsupportedAlgorithm);
        }
        let profile = Argon2Profile::from_id(*profile)?;
        check_profile(profile)?;
        let (salt, rest) = take::<16>(rest)?;
        let (export_id, rest) = take::<16>(rest)?;
        let (nonce_prefix, _) = take::<NONCE_PREFIX_LEN>(rest)?;
        Ok(Self {
            raw,
            profile,
            salt: *salt,
            export_id: *export_id,
            nonce_prefix: *nonce_prefix,
        })
    }
}

/// An export may not use a profile below [`MIN_PROFILE`].
fn check_profile(profile: Argon2Profile) -> Result<()> {
    if profile < MIN_PROFILE {
        Err(Error::UnknownProfile)
    } else {
        Ok(())
    }
}

/// `K_x = HKDF(Argon2id(NFC(passphrase), salt), export_id, ctx("export-key"), 32)`.
fn derive_key(
    pass: &ExportPassphrase,
    salt: &[u8; 16],
    export_id: &[u8; 16],
    cost: Argon2Cost,
) -> Result<SecretBytes<32>> {
    let mut px = SecretBytes::<32>::zero();
    argon2id(
        pass.0.expose_secret().as_bytes(),
        salt,
        &[],
        &[],
        cost,
        px.expose_secret_mut(),
    )?;
    hkdf_ctx(px.expose_secret(), export_id, label::EXPORT_KEY)
}

fn stream(kx: &SecretBytes<32>, prefix: &[u8; NONCE_PREFIX_LEN]) -> Stream {
    let aead = XChaCha20Poly1305::new(kx.expose_secret().into());
    Stream::from_aead(aead, &(*prefix).into())
}

fn chunk_aad(header: &[u8; HEADER_LEN], export_id: &[u8; 16]) -> Aad {
    Aad::with_header(header, label::EXPORT).fixed(export_id)
}

/// Writes an encrypted export, chunk by chunk, holding at most one chunk of
/// plaintext. Drop it without [`ExportWriter::finish`] and the output is a
/// truncated stream that no reader accepts.
pub struct ExportWriter<W: Write> {
    w: W,
    stream: Stream,
    aad: Aad,
    buf: SecretVec,
    index: u32,
    total: u64,
    /// Set by the first error; every later call returns it, so a writer
    /// that failed once never produces a file that looks finished.
    failed: Option<Error>,
}

impl<W: Write> ExportWriter<W> {
    /// Derives `K_x` (Argon2id at `profile`, a fresh salt) and writes the
    /// header to `w`.
    ///
    /// # Errors
    /// [`Error::UnknownProfile`] for a profile below standard (§11.2);
    /// [`Error::Allocation`] if the profile's memory cannot be reserved;
    /// [`Error::Randomness`]; [`Error::Io`] if the header cannot be written.
    pub fn new(w: W, pass: &ExportPassphrase, profile: Argon2Profile) -> Result<Self> {
        check_profile(profile)?;
        let salt: [u8; 16] = rng::array()?;
        let export_id: [u8; 16] = rng::array()?;
        let nonce_prefix: [u8; NONCE_PREFIX_LEN] = rng::array()?;
        let kx = derive_key(pass, &salt, &export_id, profile.cost())?;
        Self::with_parts(w, &kx, profile.id(), &salt, &export_id, &nonce_prefix)
    }

    pub(crate) fn with_parts(
        mut w: W,
        kx: &SecretBytes<32>,
        profile_id: u8,
        salt: &[u8; 16],
        export_id: &[u8; 16],
        nonce_prefix: &[u8; NONCE_PREFIX_LEN],
    ) -> Result<Self> {
        let header = Header::encode(profile_id, salt, export_id, nonce_prefix)?;
        w.write_all(&header).map_err(|e| io_err(&e))?;
        Ok(Self {
            w,
            stream: stream(kx, nonce_prefix),
            aad: chunk_aad(&header, export_id),
            buf: SecretVec::with_capacity(CHUNK_CT_LEN)?,
            index: 0,
            total: 0,
            failed: None,
        })
    }

    /// A writer under a given `K_x`, for the fuzz targets only (Argon2id
    /// would cost them 256 MiB per run).
    ///
    /// # Errors
    /// As [`ExportWriter::new`], without the key stretching.
    #[cfg(fuzzing)]
    #[doc(hidden)]
    pub fn new_with_key_for_fuzzing(w: W, kx: SecretBytes<32>) -> Result<Self> {
        // Fixed salt, ID and nonce prefix: a crash reproduces from its input.
        Self::with_parts(
            w,
            &kx,
            MIN_PROFILE.id(),
            &[0x5A; 16],
            &[0xE1; 16],
            &[0x77; NONCE_PREFIX_LEN],
        )
    }

    /// Appends plaintext. The bytes are split into 64 KiB chunks however
    /// they arrive (a call need not align to a chunk); a full chunk is
    /// written only once more data follows, because the last chunk carries
    /// the STREAM end flag.
    ///
    /// # Errors
    /// [`Error::SizeLimit`] above [`MAX_PLAINTEXT_LEN`]; [`Error::Io`]. After
    /// any error the writer stays failed and returns that error again from
    /// [`ExportWriter::write_chunk`] and [`ExportWriter::finish`].
    pub fn write_chunk(&mut self, data: &[u8]) -> Result<()> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        self.write_inner(data)
            .inspect_err(|e| self.failed = Some(*e))
    }

    fn write_inner(&mut self, mut data: &[u8]) -> Result<()> {
        self.total = u64::try_from(data.len())
            .ok()
            .and_then(|n| self.total.checked_add(n))
            .filter(|t| *t <= MAX_PLAINTEXT_LEN)
            .ok_or(Error::SizeLimit)?;
        while !data.is_empty() {
            if self.buf.len() == CHUNK_LEN {
                self.flush(false)?;
            }
            let room = CHUNK_LEN.saturating_sub(self.buf.len());
            let (head, tail) = data.split_at(room.min(data.len()));
            self.buf.as_mut_vec().extend_from_slice(head);
            data = tail;
        }
        Ok(())
    }

    fn flush(&mut self, last: bool) -> Result<()> {
        let v = self.buf.as_mut_vec();
        self.stream
            .encrypt_in_place(self.index, last, self.aad.as_bytes(), v)
            .map_err(|_| Error::SizeLimit)?;
        self.w.write_all(v).map_err(|e| io_err(&e))?;
        v.clear();
        self.index = self.index.checked_add(1).ok_or(Error::SizeLimit)?;
        Ok(())
    }

    /// Writes the last chunk and returns the underlying writer. An empty
    /// export is one empty last chunk.
    ///
    /// # Errors
    /// [`Error::Io`]; [`Error::SizeLimit`].
    pub fn finish(mut self) -> Result<W> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        self.flush(true)?;
        self.w.flush().map_err(|e| io_err(&e))?;
        Ok(self.w)
    }
}

/// Reads an encrypted export: one verified chunk at a time, with one chunk
/// of look-ahead (to know which chunk is the last).
pub struct ExportReader<R: Read> {
    r: R,
    stream: Stream,
    aad: Aad,
    /// Ciphertext of the next chunk to open.
    cur: Vec<u8>,
    /// Look-ahead: the ciphertext after `cur`; empty at the end of the stream.
    nxt: Vec<u8>,
    /// The first chunk, opened by [`ExportReader::open`] so that a wrong
    /// passphrase fails there.
    first: Option<SecretVec>,
    /// Read position inside the chunk handed out by the `Read` adapter.
    pending: Option<(SecretVec, usize)>,
    index: u32,
    total: u64,
    done: bool,
    failed: Option<Error>,
}

impl<R: Read> ExportReader<R> {
    /// Reads the header, derives `K_x` and verifies the first chunk, so a
    /// wrong passphrase and a damaged file fail here.
    ///
    /// # Errors
    /// [`Error::Malformed`] (not an export, truncated header),
    /// [`Error::UnsupportedVersion`], [`Error::UnsupportedAlgorithm`],
    /// [`Error::UnknownProfile`] (unknown profile or below standard; no
    /// Argon2id run happens), [`Error::Allocation`],
    /// [`Error::Decryption`] (wrong passphrase or damaged), [`Error::Io`].
    pub fn open(mut r: R, pass: &ExportPassphrase) -> Result<Self> {
        let h = Header::read(&mut r)?;
        let kx = derive_key(pass, &h.salt, &h.export_id, h.profile.cost())?;
        Self::with_key(r, &h, &kx)
    }

    /// A reader under a given `K_x`, for the fuzz targets only.
    ///
    /// # Errors
    /// As [`ExportReader::open`], without the key stretching.
    #[cfg(fuzzing)]
    #[doc(hidden)]
    pub fn open_with_key_for_fuzzing(mut r: R, kx: SecretBytes<32>) -> Result<Self> {
        let h = Header::read(&mut r)?;
        Self::with_key(r, &h, &kx)
    }

    fn with_key(r: R, h: &Header, kx: &SecretBytes<32>) -> Result<Self> {
        let mut cur = Vec::new();
        let mut nxt = Vec::new();
        cur.try_reserve_exact(CHUNK_CT_LEN)
            .map_err(|_| Error::Allocation)?;
        nxt.try_reserve_exact(CHUNK_CT_LEN)
            .map_err(|_| Error::Allocation)?;
        let mut this = Self {
            r,
            stream: stream(kx, &h.nonce_prefix),
            aad: chunk_aad(&h.raw, &h.export_id),
            cur,
            nxt,
            first: None,
            pending: None,
            index: 0,
            total: 0,
            done: false,
            failed: None,
        };
        fill(&mut this.r, &mut this.cur)?;
        if this.cur.is_empty() {
            // No chunk at all: a writer always emits at least one.
            return Err(Error::Malformed);
        }
        fill(&mut this.r, &mut this.nxt)?;
        this.first = this.step()?;
        Ok(this)
    }

    /// The next verified chunk of plaintext (up to 64 KiB), or `None` after
    /// the verified last chunk. After any error the reader stays failed and
    /// returns that error again: a stream that failed is never complete.
    ///
    /// # Errors
    /// [`Error::Decryption`] (a chunk failed: tampering, reordering,
    /// truncation, extension or a wrong key), [`Error::Malformed`] (a
    /// non-canonical layout), [`Error::SizeLimit`] (above 2 GiB),
    /// [`Error::Io`].
    pub fn next_chunk(&mut self) -> Result<Option<SecretVec>> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        if let Some(first) = self.first.take() {
            return Ok(Some(first));
        }
        self.step()
    }

    fn step(&mut self) -> Result<Option<SecretVec>> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        if self.done {
            return Ok(None);
        }
        match self.open_current() {
            Ok(chunk) => Ok(chunk),
            Err(e) => {
                self.failed = Some(e);
                Err(e)
            }
        }
    }

    fn open_current(&mut self) -> Result<Option<SecretVec>> {
        let last = self.nxt.is_empty();
        let ct_len = self.cur.len();
        if ct_len < TAG_LEN {
            return Err(Error::Decryption);
        }
        // Canonical layout (§7.3): an empty chunk can only be the only one.
        if last && ct_len == TAG_LEN && self.index > 0 {
            return Err(Error::Malformed);
        }
        let mut pt = SecretVec::from_slice(&self.cur)?;
        self.stream
            .decrypt_in_place(self.index, last, self.aad.as_bytes(), pt.as_mut_vec())
            .map_err(|_| Error::Decryption)?;
        self.total = u64::try_from(pt.len())
            .ok()
            .and_then(|n| self.total.checked_add(n))
            .filter(|t| *t <= MAX_PLAINTEXT_LEN)
            .ok_or(Error::SizeLimit)?;
        self.index = self.index.checked_add(1).ok_or(Error::SizeLimit)?;
        if last {
            self.done = true;
        } else {
            core::mem::swap(&mut self.cur, &mut self.nxt);
            fill(&mut self.r, &mut self.nxt)?;
        }
        Ok(if pt.is_empty() { None } else { Some(pt) })
    }
}

/// Reads up to one ciphertext chunk into `buf` (fewer bytes only at the end
/// of the stream).
fn fill<R: Read>(r: &mut R, buf: &mut Vec<u8>) -> Result<()> {
    buf.clear();
    buf.resize(CHUNK_CT_LEN, 0);
    let mut n = 0usize;
    while n < CHUNK_CT_LEN {
        let Some(dst) = buf.get_mut(n..) else { break };
        match r.read(dst) {
            Ok(0) => break,
            Ok(k) => n = n.saturating_add(k).min(CHUNK_CT_LEN),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(io_err(&e)),
        }
    }
    buf.truncate(n);
    Ok(())
}

/// The plaintext as a byte stream, for parsers that read from a [`Read`].
/// An error (including a stream that fails at its last chunk) is returned
/// as [`io::ErrorKind::InvalidData`] and again on every later call, never as
/// a clean end of file.
impl<R: Read> Read for ExportReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        loop {
            let n = self.copy_pending(out);
            if n > 0 {
                return Ok(n);
            }
            match self.next_chunk() {
                Ok(Some(chunk)) => self.pending = Some((chunk, 0)),
                Ok(None) => return Ok(0),
                Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
            }
        }
    }
}

impl<R: Read> ExportReader<R> {
    /// Copies what is left of the chunk the `Read` adapter holds into `out`;
    /// 0 when nothing is left (the chunk is then dropped).
    fn copy_pending(&mut self, out: &mut [u8]) -> usize {
        let Some((chunk, offset)) = &mut self.pending else {
            return 0;
        };
        let rest = chunk.expose_secret().get(*offset..).unwrap_or_default();
        let n = rest.len().min(out.len());
        if let (Some(dst), Some(src)) = (out.get_mut(..n), rest.get(..n)) {
            dst.copy_from_slice(src);
        }
        *offset = offset.saturating_add(n);
        if n == 0 {
            self.pending = None;
        }
        n
    }
}

#[cfg(test)]
pub(crate) mod kat {
    /// One value of `crates/core/tests/vectors/export_kat.txt`: hex-decoded,
    /// or the raw text for the `*_text` lines.
    pub(crate) fn vector(name: &str) -> Vec<u8> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("vectors")
            .join("export_kat.txt");
        let text = std::fs::read_to_string(path).unwrap();
        let value = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .find_map(|l| l.strip_prefix(name).and_then(|r| r.strip_prefix('=')))
            .unwrap();
        if name.ends_with("_text") {
            return value.as_bytes().to_vec();
        }
        (0..value.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aead::{self, NONCE_LEN};
    use crate::kdf::TEST_COST;
    use sha2::{Digest, Sha256};

    const SALT: [u8; 16] = [0x5A; 16];
    const EXPORT_ID: [u8; 16] = [0xE1; 16];
    const PREFIX: [u8; NONCE_PREFIX_LEN] = [0x77; NONCE_PREFIX_LEN];

    fn pass(s: &str) -> ExportPassphrase {
        ExportPassphrase::new(&SecretString::from_str_copy(s).unwrap()).unwrap()
    }

    /// `K_x` with the cheap test cost: the same HKDF step as production, a
    /// 64 KiB Argon2id so the tests stay fast and small.
    fn kx(p: &str) -> SecretBytes<32> {
        derive_key(&pass(p), &SALT, &EXPORT_ID, TEST_COST).unwrap()
    }

    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| u8::try_from(i % 251).unwrap()).collect()
    }

    fn write(kx: &SecretBytes<32>, parts: &[&[u8]]) -> Vec<u8> {
        let mut w = ExportWriter::with_parts(
            Vec::new(),
            kx,
            Argon2Profile::Standard.id(),
            &SALT,
            &EXPORT_ID,
            &PREFIX,
        )
        .unwrap();
        for p in parts {
            w.write_chunk(p).unwrap();
        }
        w.finish().unwrap()
    }

    fn open<'a>(kx: &SecretBytes<32>, bytes: &'a [u8]) -> Result<ExportReader<&'a [u8]>> {
        let mut r = bytes;
        let h = Header::read(&mut r)?;
        ExportReader::with_key(r, &h, kx)
    }

    fn read_all(kx: &SecretBytes<32>, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut reader = open(kx, bytes)?;
        let mut out = Vec::new();
        while let Some(c) = reader.next_chunk()? {
            assert!(!c.is_empty() && c.len() <= CHUNK_LEN);
            out.extend_from_slice(c.expose_secret());
        }
        Ok(out)
    }

    #[test]
    fn round_trip_small_and_multi_chunk() {
        let k = kx("correct horse battery staple");
        let sizes = [
            0,
            1,
            CHUNK_LEN - 1,
            CHUNK_LEN,
            CHUNK_LEN + 1,
            2 * CHUNK_LEN,
            2 * CHUNK_LEN + 17,
        ];
        for len in sizes {
            let pt = pattern(len);
            let file = write(&k, &[&pt]);
            let chunks = len.div_ceil(CHUNK_LEN).max(1);
            assert_eq!(file.len(), HEADER_LEN + len + chunks * TAG_LEN, "len {len}");
            assert_eq!(&file[..4], b"MOSX");
            assert_eq!(read_all(&k, &file).unwrap(), pt, "len {len}");
            // The same plaintext split into odd pieces gives the same layout.
            let pieces: Vec<&[u8]> = pt.chunks(7919).collect();
            let split = write(&k, &pieces);
            assert_eq!(read_all(&k, &split).unwrap(), pt, "split len {len}");
            assert_eq!(split.len(), file.len());
        }
    }

    #[test]
    fn matches_the_reference_construction() {
        // Chunk bytes equal single-shot XChaCha20-Poly1305 (pinned by the
        // draft KAT in `aead`) with nonce `prefix || BE32(i) || last`.
        let k = kx("reference");
        let pt = pattern(CHUNK_LEN + 5);
        let file = write(&k, &[&pt]);
        let header = &file[..HEADER_LEN];
        let mut aad = header.to_vec();
        aad.extend_from_slice(b"mosaic/v1/export\x00");
        aad.extend_from_slice(&EXPORT_ID);
        let nonce = |i: u32, last: bool| {
            let mut n = [0u8; NONCE_LEN];
            n[..NONCE_PREFIX_LEN].copy_from_slice(&PREFIX);
            n[NONCE_PREFIX_LEN..NONCE_LEN - 1].copy_from_slice(&i.to_be_bytes());
            n[NONCE_LEN - 1] = u8::from(last);
            n
        };
        let c0 = aead::seal_with_prefix(
            k.expose_secret(),
            &nonce(0, false),
            &aad,
            &[],
            &pt[..CHUNK_LEN],
        )
        .unwrap();
        let c1 = aead::seal_with_prefix(
            k.expose_secret(),
            &nonce(1, true),
            &aad,
            &[],
            &pt[CHUNK_LEN..],
        )
        .unwrap();
        assert_eq!(&file[HEADER_LEN..HEADER_LEN + CHUNK_CT_LEN], c0.as_slice());
        assert_eq!(&file[HEADER_LEN + CHUNK_CT_LEN..], c1.as_slice());
    }

    #[test]
    fn truncated_refused() {
        let k = kx("truncate");
        let file = write(&k, &[&pattern(3 * CHUNK_LEN + 100)]);
        // Cut at every chunk boundary and inside chunks and the header.
        let cuts = [
            HEADER_LEN - 1,
            HEADER_LEN,
            HEADER_LEN + 1,
            HEADER_LEN + CHUNK_CT_LEN,
            HEADER_LEN + 2 * CHUNK_CT_LEN,
            HEADER_LEN + 3 * CHUNK_CT_LEN,
            file.len() - 1,
            file.len() - TAG_LEN,
        ];
        for cut in cuts {
            assert!(
                read_all(&k, &file[..cut]).is_err(),
                "cut at {cut} must not read to the end"
            );
        }
        // Appended bytes (extension) are refused as well.
        let mut longer = file.clone();
        longer.push(0);
        assert!(read_all(&k, &longer).is_err());
        let mut more = file.clone();
        more.extend_from_slice(&file[HEADER_LEN..HEADER_LEN + CHUNK_CT_LEN]);
        assert!(read_all(&k, &more).is_err());
    }

    #[test]
    fn reordered_chunk_refused() {
        let k = kx("reorder");
        let file = write(&k, &[&pattern(3 * CHUNK_LEN + 100)]);
        let body = &file[HEADER_LEN..];
        let chunks: Vec<&[u8]> = body.chunks(CHUNK_CT_LEN).collect();
        assert_eq!(chunks.len(), 4);
        for (i, j) in [(0, 1), (1, 2), (0, 2)] {
            let mut order = chunks.clone();
            order.swap(i, j);
            let mut edited = file[..HEADER_LEN].to_vec();
            for c in &order {
                edited.extend_from_slice(c);
            }
            assert_eq!(read_all(&k, &edited).unwrap_err(), Error::Decryption);
        }
        // Dropping or duplicating a chunk is refused too.
        let mut dropped = file[..HEADER_LEN].to_vec();
        for c in [chunks[0], chunks[2], chunks[3]] {
            dropped.extend_from_slice(c);
        }
        assert!(read_all(&k, &dropped).is_err());
        let mut dup = file[..HEADER_LEN].to_vec();
        for c in [chunks[0], chunks[0], chunks[1], chunks[2], chunks[3]] {
            dup.extend_from_slice(c);
        }
        assert!(read_all(&k, &dup).is_err());
    }

    #[test]
    fn header_tamper_fails() {
        let k = kx("header");
        let file = write(&k, &[&pattern(100)]);
        for i in 0..HEADER_LEN {
            let mut bad = file.clone();
            bad[i] ^= 0x01;
            // Magic, version, algorithm, chunk size and profile are refused
            // by the header check; salt and ID changes alter the key and the
            // AAD; the nonce prefix changes every chunk nonce. None reads.
            assert!(read_all(&k, &bad).is_err(), "header byte {i}");
        }
        // The header fields that the key does not cover are bound by the AAD
        // alone: with the key fixed, a changed profile byte or prefix fails.
        let mut bad = file.clone();
        bad[7] = Argon2Profile::Strong.id();
        assert_eq!(read_all(&k, &bad).unwrap_err(), Error::Decryption);
        let mut bad = file.clone();
        bad[HEADER_LEN - 1] ^= 1;
        assert_eq!(read_all(&k, &bad).unwrap_err(), Error::Decryption);
    }

    #[test]
    fn unsupported_header_values_have_their_own_errors() {
        let k = kx("hdr");
        let file = write(&k, &[b"x"]);
        let case = |i: usize, v: u8| {
            let mut bad = file.clone();
            bad[i] = v;
            read_all(&k, &bad).unwrap_err()
        };
        assert_eq!(case(0, b'Y'), Error::Malformed);
        assert_eq!(case(4, 2), Error::UnsupportedVersion);
        assert_eq!(case(5, 1), Error::UnsupportedAlgorithm);
        assert_eq!(case(6, 15), Error::UnsupportedAlgorithm);
        assert_eq!(case(7, 0x00), Error::UnknownProfile);
        assert_eq!(case(7, 0x05), Error::UnknownProfile);
        assert_eq!(case(7, 0x01), Error::UnknownProfile);
        assert_eq!(
            read_all(&k, &file[..HEADER_LEN - 1]).unwrap_err(),
            Error::Malformed
        );
        assert_eq!(read_all(&k, &[]).unwrap_err(), Error::Malformed);
        // A header and no chunk at all.
        assert_eq!(
            read_all(&k, &file[..HEADER_LEN]).unwrap_err(),
            Error::Malformed
        );
    }

    /// The profile byte selects the section 5.3 parameters: pin every row, so
    /// a change to a profile (which would change every export's key) fails
    /// here, and the header profile is what the reader stretches with.
    #[test]
    fn header_profile_is_pinned_to_the_53_cost_row() {
        let rows = [
            (0x02u8, Argon2Profile::Standard, 256 * 1024, 3, 4),
            (0x03, Argon2Profile::Strong, 512 * 1024, 3, 4),
            (0x04, Argon2Profile::Max, 1024 * 1024, 4, 4),
        ];
        for (id, profile, m, t, p) in rows {
            let header = Header::encode(id, &SALT, &EXPORT_ID, &PREFIX).unwrap();
            let parsed = Header::read(&mut header.as_slice()).unwrap();
            assert_eq!(parsed.profile, profile);
            assert_eq!(parsed.profile.id(), id);
            let cost = parsed.profile.cost();
            assert_eq!((cost.m_kib, cost.t, cost.p), (m, t, p), "profile {id:#x}");
        }
    }

    #[test]
    fn profile_below_standard_refused() {
        let p = pass("any passphrase will do");
        // Refused before any key stretching, so this is instant.
        let err = ExportWriter::new(Vec::new(), &p, Argon2Profile::Baseline)
            .err()
            .unwrap();
        assert_eq!(err, Error::UnknownProfile);
        // A file that claims the baseline profile is refused by the reader
        // without an Argon2id run.
        let k = kx("x");
        let mut file = write(&k, &[b"x"]);
        file[7] = Argon2Profile::Baseline.id();
        assert_eq!(
            ExportReader::open(file.as_slice(), &p).err().unwrap(),
            Error::UnknownProfile
        );
        assert_eq!(MIN_PROFILE, Argon2Profile::Standard);
    }

    #[test]
    fn wrong_passphrase_unreadable() {
        let k = kx("right passphrase");
        let file = write(&k, &[&pattern(CHUNK_LEN + 3)]);
        for wrong in ["wrong passphrase", "right passphrasE", "right passphrase "] {
            assert_eq!(
                open(&kx(wrong), &file).err().unwrap(),
                Error::Decryption,
                "{wrong:?}"
            );
        }
        // And through the real derivation (the standard profile, 256 MiB):
        // the right passphrase reads, a wrong one is unreadable at open.
        let right = pass("right passphrase");
        let mut w = ExportWriter::new(Vec::new(), &right, Argon2Profile::Standard).unwrap();
        w.write_chunk(b"{\"format\":\"mosaic-export\"}").unwrap();
        let file = w.finish().unwrap();
        assert_eq!(file[7], Argon2Profile::Standard.id());
        let mut reader = ExportReader::open(file.as_slice(), &right).unwrap();
        assert_eq!(
            reader.next_chunk().unwrap().unwrap().expose_secret(),
            b"{\"format\":\"mosaic-export\"}"
        );
        assert!(reader.next_chunk().unwrap().is_none());
        assert_eq!(
            ExportReader::open(file.as_slice(), &pass("wrong passphrase"))
                .err()
                .unwrap(),
            Error::Decryption
        );
    }

    #[test]
    fn failed_reader_stays_failed() {
        let k = kx("poison");
        let mut file = write(&k, &[&pattern(2 * CHUNK_LEN + 1)]);
        let n = file.len();
        file[n - 1] ^= 1;
        let mut r = open(&k, &file).unwrap();
        assert!(r.next_chunk().unwrap().is_some());
        assert!(r.next_chunk().unwrap().is_some());
        assert_eq!(r.next_chunk().unwrap_err(), Error::Decryption);
        // Never a clean end after a failure.
        assert_eq!(r.next_chunk().unwrap_err(), Error::Decryption);
        let mut buf = [0u8; 8];
        assert_eq!(
            r.read(&mut buf).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn non_canonical_empty_last_chunk_refused() {
        // An authentic stream whose last chunk is empty although it is not
        // the only one: STREAM accepts it, the canonical rule (§7.3) must not.
        let k = kx("canon");
        let aad = chunk_aad(
            &Header::encode(0x02, &SALT, &EXPORT_ID, &PREFIX).unwrap(),
            &EXPORT_ID,
        );
        let s = stream(&k, &PREFIX);
        let mut file = Header::encode(0x02, &SALT, &EXPORT_ID, &PREFIX)
            .unwrap()
            .to_vec();
        let mut c0 = pattern(CHUNK_LEN);
        s.encrypt_in_place(0, false, aad.as_bytes(), &mut c0)
            .unwrap();
        let mut c1 = Vec::new();
        s.encrypt_in_place(1, true, aad.as_bytes(), &mut c1)
            .unwrap();
        file.extend_from_slice(&c0);
        file.extend_from_slice(&c1);
        assert_eq!(read_all(&k, &file).unwrap_err(), Error::Malformed);
    }

    #[test]
    fn read_adapter_streams_the_plaintext() {
        let k = kx("read");
        let pt = pattern(2 * CHUNK_LEN + 1234);
        let file = write(&k, &[&pt]);
        let mut reader = open(&k, &file).unwrap();
        let mut out = Vec::new();
        let mut small = [0u8; 1000];
        loop {
            let n = reader.read(&mut small).unwrap();
            if n == 0 {
                break;
            }
            out.extend_from_slice(&small[..n]);
        }
        assert_eq!(out, pt);
        assert_eq!(reader.read(&mut small).unwrap(), 0);
        // An empty export reads as an empty stream.
        let empty = write(&k, &[]);
        let mut reader = open(&k, &empty).unwrap();
        assert_eq!(reader.read(&mut small).unwrap(), 0);
        assert!(reader.next_chunk().unwrap().is_none());
    }

    #[test]
    fn size_limit_is_checked_before_allocation_of_more() {
        let k = kx("limit");
        let mut w =
            ExportWriter::with_parts(Vec::new(), &k, 0x02, &SALT, &EXPORT_ID, &PREFIX).unwrap();
        w.total = MAX_PLAINTEXT_LEN;
        assert_eq!(w.write_chunk(b"x").unwrap_err(), Error::SizeLimit);
        assert_eq!(w.write_chunk(b"").unwrap_err(), Error::SizeLimit, "sticky");
    }

    /// A writer that refuses exactly one write, then works again.
    struct FailsOnce {
        calls: u32,
        fail_at: u32,
        out: Vec<u8>,
    }
    impl Write for FailsOnce {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.calls += 1;
            if self.calls == self.fail_at {
                return Err(io::Error::from(io::ErrorKind::StorageFull));
            }
            self.out.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn failed_writer_stays_failed() {
        let k = kx("sticky");
        // Write 1 is the header, write 2 the first full chunk: it fails.
        let sink = FailsOnce {
            calls: 0,
            fail_at: 2,
            out: Vec::new(),
        };
        let mut w = ExportWriter::with_parts(sink, &k, 0x02, &SALT, &EXPORT_ID, &PREFIX).unwrap();
        let err = Error::Io(io::ErrorKind::StorageFull);
        w.write_chunk(&pattern(CHUNK_LEN)).unwrap();
        assert_eq!(w.write_chunk(b"x").unwrap_err(), err);
        // The sink works again, but the writer does not resume.
        assert_eq!(w.write_chunk(b"more").unwrap_err(), err);
        assert_eq!(w.write_chunk(b"").unwrap_err(), err);
        assert_eq!(w.finish().err().unwrap(), err);
        // A failure at finish is reported by finish itself.
        let sink = FailsOnce {
            calls: 0,
            fail_at: 2,
            out: Vec::new(),
        };
        let mut w = ExportWriter::with_parts(sink, &k, 0x02, &SALT, &EXPORT_ID, &PREFIX).unwrap();
        w.write_chunk(b"abc").unwrap();
        assert_eq!(w.finish().err().unwrap(), err);
    }

    #[test]
    fn writer_error_is_reported_not_swallowed() {
        struct Failing;
        impl Write for Failing {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let err = ExportWriter::with_parts(Failing, &kx("io"), 0x02, &SALT, &EXPORT_ID, &PREFIX)
            .err()
            .unwrap();
        assert_eq!(err, Error::Io(io::ErrorKind::PermissionDenied));
    }

    #[test]
    fn passphrase_is_nfc_and_never_empty_and_redacted() {
        let composed = pass("caf\u{e9} passphrase");
        let decomposed = pass("cafe\u{301} passphrase");
        assert_eq!(composed.0.expose_secret(), decomposed.0.expose_secret());
        assert_eq!(
            ExportPassphrase::new(&SecretString::from_str_copy("").unwrap())
                .err()
                .unwrap(),
            Error::EmptyPassword
        );
        assert_eq!(format!("{composed:?}"), "[REDACTED]");
        // Whitespace is kept as typed (no trimming).
        assert_ne!(
            pass("a b").0.expose_secret(),
            pass("a b ").0.expose_secret()
        );
    }

    /// `crates/core/tests/vectors/export_kat.txt`, written by the
    /// independent script `export_kat.py`: header, `K_x` (from a fixed `P_x`
    /// in place of Argon2id, which the M1 vectors cover) and two whole
    /// files.
    mod kat {
        use super::*;
        use crate::export::kat::vector;

        fn kat_key() -> SecretBytes<32> {
            hkdf_ctx(&[0x33; 32], &EXPORT_ID, label::EXPORT_KEY).unwrap()
        }

        #[test]
        fn matches_python() {
            let k = kat_key();
            assert_eq!(k.expose_secret().as_slice(), vector("k_x"));
            let header = Header::encode(0x02, &SALT, &EXPORT_ID, &PREFIX).unwrap();
            assert_eq!(header.as_slice(), vector("header"));

            let small_pt = vector("small_plaintext");
            let small = write(&k, &[&small_pt]);
            assert_eq!(small, vector("small_file"));
            assert_eq!(read_all(&k, &small).unwrap(), small_pt);

            // Two chunks: 64 KiB + 100 bytes of the pattern i % 251.
            let big_pt = pattern(CHUNK_LEN + 100);
            let big = write(&k, &[&big_pt]);
            let len = String::from_utf8(vector("big_file_len_text")).unwrap();
            assert_eq!(big.len(), len.parse::<usize>().unwrap());
            assert_eq!(Sha256::digest(&big).as_slice(), vector("big_file_sha256"));
            assert_eq!(
                Sha256::digest(&big[HEADER_LEN..HEADER_LEN + CHUNK_CT_LEN]).as_slice(),
                vector("big_chunk0_sha256")
            );
            assert_eq!(&big[big.len() - TAG_LEN..], vector("big_last_tag"));
            assert_eq!(read_all(&k, &big).unwrap(), big_pt);

            // The empty export: one empty last chunk (header + tag).
            let empty = write(&k, &[]);
            assert_eq!(empty, vector("empty_file"));
            assert_eq!(empty.len(), HEADER_LEN + TAG_LEN);
            assert_eq!(read_all(&k, &empty).unwrap(), Vec::<u8>::new());

            // A last chunk of exactly 64 KiB: one full chunk, flagged last.
            let exact_pt = pattern(CHUNK_LEN);
            let exact = write(&k, &[&exact_pt]);
            assert_eq!(
                Sha256::digest(&exact).as_slice(),
                vector("exact_file_sha256")
            );
            assert_eq!(exact.len(), HEADER_LEN + CHUNK_CT_LEN);
            assert_eq!(read_all(&k, &exact).unwrap(), exact_pt);
        }
    }
}
