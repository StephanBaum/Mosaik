//! M9 T12 (sync-chaos): oversized vault-metadata merges through the real
//! session path (CRYPTO_SPEC §7.1, ADR-0085). Three devices of one account
//! receive three long concurrent metadata versions, sealed by peers that
//! ignore the write caps, in different orders and with replays. They must
//! store the same canonical bytes and version vector, report the same
//! dropped fields, persist what they hold, and ignore every replay.

#![expect(clippy::unwrap_used, reason = "integration tests")]

use crate::common::{PASSWORD, join, new_account, pw};
use mosaic_core::model::{DroppedMeta, Timestamp, VaultMeta};
use mosaic_core::{DeviceId, UnlockMethod, VaultId, VaultKey, VaultMetaCiphertext};
use mosaic_vault::session::{ApplyOutcome, UnlockedSession};

fn vault_key(a: &UnlockedSession, secret_key: &mosaic_core::SecretKey, vault: VaultId) -> VaultKey {
    let params = a.store().account().unwrap().unwrap();
    let wrapped_ak = a
        .store()
        .unlock_method(&UnlockMethod::Password)
        .unwrap()
        .unwrap()
        .wrapped_account_key;
    let ak =
        mosaic_core::unlock_with_password(&pw(PASSWORD), secret_key, &params, &wrapped_ak).unwrap();
    let (generation, wrapped) = a
        .store()
        .vault_keys(&vault)
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    wrapped
        .unwrap_vault_key(&ak, &params.account_id, &vault, generation)
        .unwrap()
}

fn plaintext(m: &VaultMeta) -> Vec<u8> {
    m.to_json().unwrap().expose_secret().to_vec()
}

/// A version of `base` written by a fresh writer that ignores the write
/// caps: name and description of the given lengths.
fn long(
    base: &VaultMeta,
    vk: &VaultKey,
    vault: &VaultId,
    tag: char,
    name: usize,
    desc: usize,
) -> VaultMetaCiphertext {
    let mut m = VaultMeta::from_json(&plaintext(base)).unwrap();
    {
        let mut e = m.edit(DeviceId::new_v7().unwrap(), Timestamp::from_unix_millis(7));
        e.set_name("@N@").unwrap();
        e.set_description(Some("@D@")).unwrap();
    }
    let text = String::from_utf8(plaintext(&m))
        .unwrap()
        .replace("@N@", &format!("{tag}{}", "n".repeat(name)))
        .replace("@D@", &format!("{tag}{}", "d".repeat(desc)));
    VaultMeta::from_json(text.as_bytes())
        .unwrap()
        .seal(vk, vault)
        .unwrap()
}

fn merge_dropped(into: &mut DroppedMeta, from: DroppedMeta) {
    into.name |= from.name;
    into.icon |= from.icon;
    into.description |= from.description;
}

#[test]
fn three_devices_store_identical_bytes_in_every_delivery_order() {
    let (mut a, created) = new_account();
    let vault = a.create_vault("Shared").unwrap();
    let b = join(
        &a,
        &created.secret_key,
        mosaic_vault::storage::Store::open_in_memory().unwrap(),
    );
    let c = join(
        &a,
        &created.secret_key,
        mosaic_vault::storage::Store::open_in_memory().unwrap(),
    );
    let vk = vault_key(&a, &created.secret_key, vault);
    let base = VaultMeta::from_json(&plaintext(a.vault_meta(vault).unwrap().unwrap())).unwrap();

    // Each version alone fits, two together do not.
    let writes = [
        long(&base, &vk, &vault, 'x', 20_000, 20_000),
        long(&base, &vk, &vault, 'y', 21_000, 19_000),
        long(&base, &vk, &vault, 'z', 22_000, 18_000),
    ];
    let mut devices = [a, b, c];
    let orders = [[0, 1, 2], [2, 0, 1], [1, 2, 0]];
    let mut dropped = [DroppedMeta::default(); 3];
    for (d, order) in orders.iter().enumerate() {
        for &w in order {
            match devices[d]
                .apply_remote_vault_meta(vault, &writes[w])
                .unwrap()
            {
                ApplyOutcome::MetaValueDropped { vault: v, fields } => {
                    assert_eq!(v, vault);
                    merge_dropped(&mut dropped[d], fields);
                }
                ApplyOutcome::Merged { .. } | ApplyOutcome::FastForward => {}
                other => panic!("device {d}: unexpected {other:?}"),
            }
            let stored = devices[d].vault_meta(vault).unwrap().unwrap();
            assert!(
                stored.to_json().is_ok(),
                "device {d} stored an oversized record"
            );
        }
    }

    // Same bytes, same version vector, same report, on every device.
    let reference = plaintext(devices[0].vault_meta(vault).unwrap().unwrap());
    let reference_vv = devices[0]
        .vault_meta(vault)
        .unwrap()
        .unwrap()
        .version()
        .clone();
    assert!(!dropped[0].is_empty(), "the reduction must be reported");
    for d in 1..3 {
        let m = devices[d].vault_meta(vault).unwrap().unwrap();
        assert!(plaintext(m) == reference, "device {d} stored other bytes");
        assert_eq!(m.version(), &reference_vv, "device {d} version vector");
        assert_eq!(dropped[d], dropped[0], "device {d} dropped-field report");
    }
    assert!(
        devices[0]
            .vault_meta(vault)
            .unwrap()
            .unwrap()
            .conflicts()
            .is_empty()
    );

    // What is persisted (and queued for upload) is what is held.
    for (d, device) in devices.iter().enumerate() {
        let ct = device.store().vault_meta(&vault).unwrap().unwrap();
        let opened = VaultMeta::open(&ct, &vk, &vault).unwrap();
        assert!(
            plaintext(&opened) == reference,
            "device {d} persisted other bytes"
        );
    }

    // Replays of every record, and of each device's stored record, change
    // nothing.
    for d in 0..3 {
        for w in &writes {
            assert_eq!(
                devices[d].apply_remote_vault_meta(vault, w).unwrap(),
                ApplyOutcome::Ignored,
                "device {d}"
            );
        }
        for from in 0..3 {
            let ct = devices[from].store().vault_meta(&vault).unwrap().unwrap();
            assert_eq!(
                devices[d].apply_remote_vault_meta(vault, &ct).unwrap(),
                ApplyOutcome::Ignored,
                "device {d} receiving device {from}'s stored record"
            );
        }
    }
}
