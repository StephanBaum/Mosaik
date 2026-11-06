//! The checkpoint (CRYPTO_SPEC §10): what a device has seen, MAC'd under
//! `K_cp`. The same bytes are the hosted upload body and the folder's
//! `head.mck`. Transports carry them as opaque bytes, filed under the device
//! ID of the path (`d/<device>/`) or of the hosted `deviceId`; a reader
//! decodes them together with that ID ([`Checkpoint::decode`]) and a body
//! whose device differs is refused with [`WireError::Path`].
//!
//! ```text
//! "MCKP" || 1 || device_id[16] || counter:u64 || issued_at_ms:u64 || n:u32
//! || n x ( vault_id[16] || generation:u32 || high_seq:u64 ) || mac[32]
//! seen_digest = SHA-256(issued_at_ms:u64 || n:u32 || entries)
//! mac = HMAC(K_cp, ctx("checkpoint") || device_id || counter:u64 || seen_digest)
//! ```
//!
//! Entries are sorted by `vault_id` without duplicates. `high_seq` is, for
//! the hosted backend, the device's cursor after the sync (the last
//! `server_seq` applied), and for the folder backend the highest segment
//! sequence the device wrote for the vault (0 if none).

use mosaic_core::{InstallId, KeyGeneration, VaultId};
use sha2::{Digest, Sha256};

use super::{Reader, SyncCrypto, WireError, split_mac};

const MAGIC: &[u8; 4] = b"MCKP";
const VERSION: u8 = 1;
/// `magic || version || device || counter || issued_at || n`.
const HEADER_LEN: usize = 4 + 1 + 16 + 8 + 8 + 4;
const ENTRY_LEN: usize = 16 + 4 + 8;
/// The most entries one checkpoint holds.
pub const MAX_ENTRIES: usize = 4096;
/// The longest checkpoint in bytes.
pub const MAX_LEN: usize = HEADER_LEN + MAX_ENTRIES * ENTRY_LEN + 32;

/// What a checkpoint says about one vault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointEntry {
    /// The vault.
    pub vault_id: VaultId,
    /// The device's current key generation of the vault.
    pub generation: KeyGeneration,
    /// The sequence number reached (see the module documentation).
    pub high_seq: u64,
}

/// A checkpoint of one device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    /// The install that published it.
    pub device_id: InstallId,
    /// Its monotonic counter.
    pub counter: u64,
    /// When it was issued (ms since the Unix epoch).
    pub issued_at_ms: u64,
    /// One entry per synced vault, strictly ascending by vault ID.
    pub entries: Vec<CheckpointEntry>,
}

impl Checkpoint {
    /// `SHA-256(issued_at_ms:u64 || n:u32 || entries)` for the entries as
    /// listed (CRYPTO_SPEC §10).
    ///
    /// # Errors
    /// [`WireError::Field`] for more than [`MAX_ENTRIES`] entries.
    pub fn seen_digest(&self) -> Result<[u8; 32], WireError> {
        let n = u32::try_from(self.entries.len()).map_err(|_| WireError::Field)?;
        let mut hash = Sha256::new();
        hash.update(self.issued_at_ms.to_be_bytes());
        hash.update(n.to_be_bytes());
        for e in &self.entries {
            hash.update(e.vault_id.as_bytes());
            hash.update(e.generation.get().to_be_bytes());
            hash.update(e.high_seq.to_be_bytes());
        }
        Ok(hash.finalize().into())
    }

    fn check_entries(&self) -> Result<(), WireError> {
        if self.entries.len() > MAX_ENTRIES {
            return Err(WireError::Field);
        }
        if self
            .entries
            .windows(2)
            .any(|w| w.first().map(|e| e.vault_id) >= w.get(1).map(|e| e.vault_id))
        {
            return Err(WireError::Order);
        }
        Ok(())
    }

    /// Encodes the checkpoint and MACs it with this install's key.
    /// `device_id` must be the install of `crypto`.
    ///
    /// # Errors
    /// [`WireError::Id`] if `device_id` is not the install of `crypto`;
    /// [`WireError::Field`] for too many entries; [`WireError::Order`] for
    /// entries that are not strictly ascending; [`WireError::Crypto`].
    pub fn encode(&self, crypto: &impl SyncCrypto) -> Result<Vec<u8>, WireError> {
        if self.device_id != crypto.install_id() {
            return Err(WireError::Id);
        }
        self.check_entries()?;
        let n = u32::try_from(self.entries.len()).map_err(|_| WireError::Field)?;
        let mut out = Vec::with_capacity(HEADER_LEN + self.entries.len() * ENTRY_LEN + 32);
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(self.device_id.as_bytes());
        out.extend_from_slice(&self.counter.to_be_bytes());
        out.extend_from_slice(&self.issued_at_ms.to_be_bytes());
        out.extend_from_slice(&n.to_be_bytes());
        for e in &self.entries {
            out.extend_from_slice(e.vault_id.as_bytes());
            out.extend_from_slice(&e.generation.get().to_be_bytes());
            out.extend_from_slice(&e.high_seq.to_be_bytes());
        }
        let mac = crypto.checkpoint_mac(self.counter, &self.seen_digest()?)?;
        out.extend_from_slice(&mac);
        Ok(out)
    }

    /// Decodes a checkpoint: length bounds, the MAC (the device and counter
    /// are read from their fixed offsets first, the digest is computed over
    /// the entries), then the content. `expected_device` is the device the
    /// transport filed the checkpoint under (the path's `d/<device>/` or the
    /// hosted `deviceId`); the body must name the same device.
    ///
    /// # Errors
    /// [`WireError::Length`], [`WireError::Mac`], [`WireError::Magic`],
    /// [`WireError::Version`], [`WireError::Id`], [`WireError::Path`],
    /// [`WireError::Field`], [`WireError::Order`], [`WireError::Crypto`].
    pub fn decode(
        bytes: &[u8],
        expected_device: InstallId,
        crypto: &impl SyncCrypto,
    ) -> Result<Self, WireError> {
        Self::decode_with(bytes, expected_device, |device, counter, digest, mac| {
            crypto.verify_checkpoint_mac(device, counter, digest, mac)
        })
    }

    /// Decodes the structure without checking the MAC, for a party without
    /// `AK` (the hosted server checks path, length and shape; the clients
    /// verify the MAC). The result must not be trusted until
    /// [`Checkpoint::verify`] accepts it.
    ///
    /// # Errors
    /// As [`Checkpoint::decode`] except [`WireError::Mac`].
    pub fn decode_unverified(bytes: &[u8], expected_device: InstallId) -> Result<Self, WireError> {
        Self::decode_with(bytes, expected_device, |_, _, _, _| Ok(true))
    }

    /// Whether `bytes` carry this checkpoint's MAC, in constant time.
    ///
    /// # Errors
    /// [`WireError::Length`], [`WireError::Crypto`].
    pub fn verify(&self, bytes: &[u8], crypto: &impl SyncCrypto) -> Result<bool, WireError> {
        let (_, mac) = split_mac(bytes)?;
        crypto.verify_checkpoint_mac(self.device_id, self.counter, &self.seen_digest()?, &mac)
    }

    fn decode_with(
        bytes: &[u8],
        expected_device: InstallId,
        verify: impl FnOnce(InstallId, u64, &[u8; 32], &[u8; 32]) -> Result<bool, WireError>,
    ) -> Result<Self, WireError> {
        if bytes.len() < HEADER_LEN + 32 || bytes.len() > MAX_LEN {
            return Err(WireError::Length);
        }
        let (body, mac) = split_mac(bytes)?;
        // The MAC comes first: only the device and the counter at their fixed
        // offsets and the digest over the rest are needed for it.
        let mut peek = Reader::new(body.get(5..).ok_or(WireError::Length)?);
        let mac_device = InstallId::from_bytes(peek.array()?).map_err(|_| WireError::Id)?;
        let counter = peek.u64()?;
        let signed = body.get(4 + 1 + 16 + 8..).ok_or(WireError::Length)?;
        let seen_digest: [u8; 32] = Sha256::digest(signed).into();
        if !verify(mac_device, counter, &seen_digest, &mac)? {
            return Err(WireError::Mac);
        }
        let mut r = Reader::new(body);
        if &r.array::<4>()? != MAGIC {
            return Err(WireError::Magic);
        }
        if r.u8()? != VERSION {
            return Err(WireError::Version);
        }
        let device_id = InstallId::from_bytes(r.array()?).map_err(|_| WireError::Id)?;
        if device_id != expected_device {
            return Err(WireError::Path);
        }
        let counter = r.u64()?;
        let issued_at_ms = r.u64()?;
        let n = usize::try_from(r.u32()?).map_err(|_| WireError::Field)?;
        if n > MAX_ENTRIES || n.checked_mul(ENTRY_LEN) != Some(r.remaining()) {
            return Err(WireError::Length);
        }
        let mut entries = Vec::with_capacity(n);
        for _ in 0..n {
            entries.push(CheckpointEntry {
                vault_id: VaultId::from_bytes(r.array()?).map_err(|_| WireError::Id)?,
                generation: KeyGeneration::new(r.u32()?).map_err(|_| WireError::Field)?,
                high_seq: r.u64()?,
            });
        }
        r.finish()?;
        let checkpoint = Self {
            device_id,
            counter,
            issued_at_ms,
            entries,
        };
        checkpoint.check_entries()?;
        Ok(checkpoint)
    }
}

#[cfg(test)]
mod tests {
    use mosaic_core::AccountKey;

    use super::*;
    use crate::wire::KeyedCrypto;

    fn crypto() -> (KeyedCrypto, InstallId) {
        let ak = AccountKey::generate().unwrap();
        let install = InstallId::new_v7().unwrap();
        (
            KeyedCrypto::new(&ak, mosaic_core::AccountId::new_v7().unwrap(), install).unwrap(),
            install,
        )
    }

    fn checkpoint(device_id: InstallId, n: u8) -> Checkpoint {
        Checkpoint {
            device_id,
            counter: 9,
            issued_at_ms: 1_700_000_000_000,
            entries: (1..=n)
                .map(|i| CheckpointEntry {
                    vault_id: VaultId::from_bytes([i; 16]).unwrap(),
                    generation: KeyGeneration::new(u32::from(i)).unwrap(),
                    high_seq: u64::from(i) * 100,
                })
                .collect(),
        }
    }

    #[test]
    fn round_trip_and_layout() {
        let (c, id) = crypto();
        for n in [0u8, 1, 3] {
            let cp = checkpoint(id, n);
            let bytes = cp.encode(&c).unwrap();
            assert_eq!(bytes.len(), HEADER_LEN + usize::from(n) * ENTRY_LEN + 32);
            assert_eq!(&bytes[..5], b"MCKP\x01");
            assert_eq!(Checkpoint::decode(&bytes, id, &c).unwrap(), cp);
            assert_eq!(Checkpoint::decode_unverified(&bytes, id).unwrap(), cp);
            assert!(cp.verify(&bytes, &c).unwrap());
        }
    }

    #[test]
    fn the_digest_is_the_spec_formula() {
        let (_, id) = crypto();
        let cp = checkpoint(id, 2);
        let mut flat = Vec::new();
        flat.extend_from_slice(&cp.issued_at_ms.to_be_bytes());
        flat.extend_from_slice(&2u32.to_be_bytes());
        for e in &cp.entries {
            flat.extend_from_slice(e.vault_id.as_bytes());
            flat.extend_from_slice(&e.generation.get().to_be_bytes());
            flat.extend_from_slice(&e.high_seq.to_be_bytes());
        }
        assert_eq!(
            cp.seen_digest().unwrap(),
            <[u8; 32]>::from(Sha256::digest(&flat))
        );
    }

    #[test]
    fn rejections() {
        let (c, id) = crypto();
        let cp = checkpoint(id, 3);
        let bytes = cp.encode(&c).unwrap();
        // A flipped bit anywhere fails the MAC (or the magic/version checks
        // that come first for the checks that need no key).
        // The magic and the version are constants outside the MAC; they are
        // checked, after it, on their own.
        for i in [5usize, 21, 29, 33, 40, bytes.len() - 1] {
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert_eq!(
                Checkpoint::decode(&bad, id, &c),
                Err(WireError::Mac),
                "byte {i}"
            );
        }
        assert_eq!(Checkpoint::decode(&[], id, &c), Err(WireError::Length));
        assert_eq!(
            Checkpoint::decode(&bytes[..HEADER_LEN + 31], id, &c),
            Err(WireError::Length)
        );
        // Wrong device or key.
        let (other, _) = crypto();
        assert_eq!(Checkpoint::decode(&bytes, id, &other), Err(WireError::Mac));
        assert!(!cp.verify(&bytes, &other).unwrap());
        // The unverified decoder checks the shape only.
        let mut bad_magic = bytes.clone();
        bad_magic[0] = b'X';
        assert_eq!(
            Checkpoint::decode_unverified(&bad_magic, id),
            Err(WireError::Magic)
        );
        let mut bad_version = bytes.clone();
        bad_version[4] = 2;
        assert_eq!(
            Checkpoint::decode_unverified(&bad_version, id),
            Err(WireError::Version)
        );
        assert_eq!(
            Checkpoint::decode(&bad_version, id, &c),
            Err(WireError::Version)
        );
        assert_eq!(
            Checkpoint::decode(&bad_magic, id, &c),
            Err(WireError::Magic)
        );
        let mut nil = bytes.clone();
        nil[5..21].fill(0);
        assert_eq!(Checkpoint::decode_unverified(&nil, id), Err(WireError::Id));
        let mut trailing = bytes.clone();
        trailing.insert(bytes.len() - 32, 0);
        assert_eq!(
            Checkpoint::decode_unverified(&trailing, id),
            Err(WireError::Length)
        );
        let mut count = bytes.clone();
        count[HEADER_LEN - 1] = 2;
        assert_eq!(
            Checkpoint::decode_unverified(&count, id),
            Err(WireError::Length)
        );
        let mut huge = bytes.clone();
        huge[HEADER_LEN - 4..HEADER_LEN].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            Checkpoint::decode_unverified(&huge, id),
            Err(WireError::Length)
        );
    }

    #[test]
    fn entry_rejections() {
        let (c, id) = crypto();
        let bytes = checkpoint(id, 3).encode(&c).unwrap();
        // Entries: nil vault, generation 0, unsorted, duplicate.
        let entry = HEADER_LEN;
        let mut nil_vault = bytes.clone();
        nil_vault[entry..entry + 16].fill(0);
        assert_eq!(
            Checkpoint::decode_unverified(&nil_vault, id),
            Err(WireError::Id)
        );
        let mut gen0 = bytes.clone();
        gen0[entry + 16..entry + 20].fill(0);
        assert_eq!(
            Checkpoint::decode_unverified(&gen0, id),
            Err(WireError::Field)
        );
        let mut unsorted = bytes.clone();
        let (a, b) = (entry, entry + ENTRY_LEN);
        for k in 0..16 {
            unsorted.swap(a + k, b + k);
        }
        assert_eq!(
            Checkpoint::decode_unverified(&unsorted, id),
            Err(WireError::Order)
        );
        let mut dup = bytes.clone();
        for k in 0..16 {
            dup[b + k] = dup[a + k];
        }
        assert_eq!(
            Checkpoint::decode_unverified(&dup, id),
            Err(WireError::Order)
        );
    }

    /// A checkpoint of device A filed under device B is refused, with a
    /// valid MAC (H1: the filing is not authenticated, the body is).
    #[test]
    fn a_checkpoint_filed_under_another_device_is_refused() {
        let (c, a) = crypto();
        let b = InstallId::new_v7().unwrap();
        let bytes = checkpoint(a, 2).encode(&c).unwrap();
        assert!(Checkpoint::decode(&bytes, a, &c).is_ok());
        assert_eq!(Checkpoint::decode(&bytes, b, &c), Err(WireError::Path));
        assert_eq!(
            Checkpoint::decode_unverified(&bytes, b),
            Err(WireError::Path)
        );
        // With a wrong MAC the MAC error comes first.
        let mut bad = bytes;
        let last = bad.len() - 1;
        bad[last] ^= 1;
        assert_eq!(Checkpoint::decode(&bad, b, &c), Err(WireError::Mac));
    }

    #[test]
    fn encode_checks_its_input() {
        let (c, id) = crypto();
        let mut cp = checkpoint(id, 2);
        cp.entries.swap(0, 1);
        assert_eq!(cp.encode(&c), Err(WireError::Order));
        let mut cp = checkpoint(id, 2);
        cp.entries[1].vault_id = cp.entries[0].vault_id;
        assert_eq!(cp.encode(&c), Err(WireError::Order));
        let cp = checkpoint(InstallId::new_v7().unwrap(), 1);
        assert_eq!(cp.encode(&c), Err(WireError::Id), "not this install");
        let mut cp = checkpoint(id, 1);
        cp.entries = (0..=u32::try_from(MAX_ENTRIES).unwrap())
            .map(|i| CheckpointEntry {
                vault_id: VaultId::from_bytes({
                    let mut b = [0u8; 16];
                    b[12..].copy_from_slice(&(i + 1).to_be_bytes());
                    b
                })
                .unwrap(),
                generation: KeyGeneration::FIRST,
                high_seq: 0,
            })
            .collect();
        assert_eq!(cp.encode(&c), Err(WireError::Field));
        cp.entries.truncate(MAX_ENTRIES);
        let bytes = cp.encode(&c).unwrap();
        assert_eq!(bytes.len(), MAX_LEN);
        assert_eq!(
            Checkpoint::decode(&bytes, id, &c).unwrap().entries.len(),
            MAX_ENTRIES
        );
    }
}
