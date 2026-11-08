//! The folder side of an account re-key (M7, CRYPTO_SPEC §6.6, §6.7, §10.1,
//! ADR-0058): what a device sees of a re-key in the account directory, the
//! account files it may adopt, and what a device that re-keyed or adopted
//! publishes.
//!
//! A hard stop exists only for a re-key marker (`.mrek`) whose MAC verifies
//! under this device's keys. An account file that does not verify and names
//! this device's epoch (after a re-key or an adoption) or a higher one is
//! reported, never a stop: anyone with write access to the folder can plant
//! one. Objects of another epoch never verify, so a device ignores them.

use std::collections::BTreeSet;

use mosaic_core::InstallId;
use sha2::{Digest, Sha256};

use super::fsutil;
use super::{FolderCrypto, FolderTransport, decoded};
use crate::transport::{
    MaintainContext, RekeyCandidate, RekeyMarkerSeen, RekeyPublish, RekeySignals, TransportError,
    UnverifiedAccountFile,
};
use crate::wire::account_file::{self, AccountFile, AccountFilePath};
use crate::wire::folder_names::{self, FolderName};
use crate::wire::rekey_marker::{self, RekeyMarker, RekeyMarkerPath};

/// The most account files and markers one scan reads (newest first): a flood
/// of planted names costs this much and no more.
const MAX_SCANNED: usize = 64;

/// What the account directory lists, by kind.
struct AccountDir {
    /// `(version, device)`, highest version first, then lowest device.
    files: Vec<(u64, [u8; 16])>,
    /// `(from_epoch, device)`, highest epoch first.
    markers: Vec<(u32, [u8; 16])>,
}

impl<C: FolderCrypto> FolderTransport<C> {
    fn list_account_dir(&self) -> Result<AccountDir, TransportError> {
        let mut files = Vec::new();
        let mut markers = Vec::new();
        for e in fsutil::list(&self.layout.account_files())?
            .iter()
            .filter(|e| !e.is_dir)
        {
            match folder_names::parse(&e.name) {
                Some(FolderName::Account { version, device }) => files.push((version, device)),
                Some(FolderName::RekeyMarker { from_epoch, device }) => {
                    markers.push((from_epoch, device));
                }
                _ => {}
            }
        }
        files.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        markers.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        Ok(AccountDir { files, markers })
    }

    /// The bytes of an account file, `None` if it is missing, not a regular
    /// file or longer than any account file.
    fn read_account_file(
        &self,
        version: u64,
        device: &[u8; 16],
    ) -> Result<Option<Vec<u8>>, TransportError> {
        let path = self
            .layout
            .account_files()
            .join(folder_names::account_name(version, device));
        match fsutil::read_capped(&path, account_file::LEN as u64) {
            Ok(bytes) => Ok(bytes),
            Err(TransportError::Protocol) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// The first marker for `epoch` that verifies under this device's keys.
    fn verified_marker(
        &self,
        dir: &AccountDir,
        epoch: u32,
    ) -> Result<Option<RekeyMarker>, TransportError> {
        for (from_epoch, device) in dir
            .markers
            .iter()
            .filter(|(e, _)| *e == epoch)
            .take(MAX_SCANNED)
        {
            let Ok(device_id) = InstallId::from_bytes(*device) else {
                continue;
            };
            let path = self
                .layout
                .account_files()
                .join(folder_names::rekey_marker_name(*from_epoch, device));
            let bytes = match fsutil::read_capped(&path, rekey_marker::LEN as u64) {
                Ok(Some(b)) => b,
                Ok(None) | Err(TransportError::Protocol) => continue,
                Err(e) => return Err(e),
            };
            let id = RekeyMarkerPath {
                account_id: self.account,
                from_epoch: *from_epoch,
                device_id,
            };
            if let Some(marker) = decoded(RekeyMarker::decode(&bytes, &id, &self.crypto))? {
                return Ok(Some(marker));
            }
        }
        Ok(None)
    }

    pub(super) fn folder_rekey_signals(
        &mut self,
        epoch: u32,
    ) -> Result<RekeySignals, TransportError> {
        self.refresh_epoch()?;
        let dir = self.list_account_dir()?;
        if let Some(m) = self.verified_marker(&dir, epoch)? {
            return Ok(RekeySignals {
                marker: Some(RekeyMarkerSeen {
                    to_epoch: m.to_epoch,
                    to_account_version: m.to_account_version,
                    device: m.device_id,
                }),
                unverified: Vec::new(),
            });
        }
        let mut unverified = Vec::new();
        for (version, device) in dir.files.iter().take(MAX_SCANNED) {
            let Ok(device_id) = InstallId::from_bytes(*device) else {
                continue;
            };
            let Some(bytes) = self.read_account_file(*version, device)? else {
                continue;
            };
            let id = AccountFilePath {
                account_id: self.account,
                account_version: *version,
                device_id,
            };
            // Only a file that parses at all is worth a finding: shapeless
            // junk was ignored before and still is.
            let Ok(header) = AccountFile::decode_for_join(&bytes, &id) else {
                continue;
            };
            if decoded(AccountFile::verify_mac(&bytes, &self.crypto))? != Some(false) {
                continue;
            }
            // A higher epoch is a re-key this device cannot verify; the same
            // epoch is a concurrent re-key, which only exists once this
            // device is past epoch 1.
            if header.ak_epoch > epoch || (header.ak_epoch == epoch && epoch > 1) {
                unverified.push(UnverifiedAccountFile {
                    name: folder_names::account_name(*version, device),
                    sha256: Sha256::digest(&bytes).into(),
                    epoch: header.ak_epoch,
                });
            }
        }
        Ok(RekeySignals {
            marker: None,
            unverified,
        })
    }

    pub(super) fn folder_rekey_candidates(
        &mut self,
        epoch: u32,
        limit: usize,
    ) -> Result<Vec<RekeyCandidate>, TransportError> {
        let dir = self.list_account_dir()?;
        let named = self
            .verified_marker(&dir, epoch)?
            .map(|m| (m.to_account_version, *m.device_id.as_bytes()));
        let mut order: Vec<(u64, [u8; 16])> = Vec::new();
        order.extend(named);
        let mut seen: BTreeSet<(u64, [u8; 16])> = order.iter().copied().collect();
        for f in dir.files.iter().take(MAX_SCANNED) {
            if seen.insert(*f) {
                order.push(*f);
            }
        }
        let mut out = Vec::new();
        for (version, device) in order {
            if out.len() >= limit {
                break;
            }
            let Ok(device_id) = InstallId::from_bytes(device) else {
                continue;
            };
            let Some(bytes) = self.read_account_file(version, &device)? else {
                continue;
            };
            let id = AccountFilePath {
                account_id: self.account,
                account_version: version,
                device_id,
            };
            let Ok(file) = AccountFile::decode_for_join(&bytes, &id) else {
                continue;
            };
            if epoch.checked_add(1) == Some(file.ak_epoch) {
                out.push(RekeyCandidate { file, bytes });
            }
        }
        Ok(out)
    }

    pub(super) fn folder_publish_rekey(
        &mut self,
        cx: &mut dyn MaintainContext,
        publish: &RekeyPublish<'_>,
    ) -> Result<(), TransportError> {
        self.refresh_epoch()?;
        if let Some((params, wrapped, version)) = publish.account_file {
            self.write_account_file(params, wrapped, version)?;
        }
        for vault in publish.snapshot_vaults {
            let objects = cx.snapshot_objects(*vault)?;
            if objects.is_empty() {
                continue;
            }
            self.write_snapshot(*vault, objects)?;
            cx.mark_rekey_snapshot(*vault)?;
        }
        if let Some(marker) = publish.marker {
            fsutil::write_atomic(
                &self.layout.account_files(),
                &folder_names::rekey_marker_name(publish.from_epoch, self.install.as_bytes()),
                marker,
                true,
            )?;
        }
        let info = self.device.clone();
        self.write_device_info(&info)
    }
}
