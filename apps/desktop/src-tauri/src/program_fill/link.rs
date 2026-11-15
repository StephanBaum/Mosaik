//! The item write of a link (ADR-0164): one `update_item` that removes the bindings a new link
//! replaces and adds the new `winapp://` URL with its display data.

use mosaic_core::model::LoginUrl;
use mosaic_core::{ItemId, VaultId};
use mosaic_program_fill::binding::{display_for, set_program_display, superseded};
use mosaic_program_fill::identity::CapturedIdentity;
use mosaic_vault::session::UnlockedSession;

use crate::contract::errors::{IpcError, IpcResult};

/// Saves `url` (a binding for `identity`) on the login: bindings of the same program whose signer
/// or file changed are removed first, and the URL is added unless an equal one is there.
///
/// # Errors
/// `itemNotFound`, `sharingRoleForbidden` and the vault's other codes through the session error.
pub fn write(
    session: &mut UnlockedSession,
    vault: VaultId,
    item: ItemId,
    identity: &CapturedIdentity,
    mut url: LoginUrl,
) -> IpcResult<()> {
    set_program_display(&mut url, &display_for(identity));
    let current = session.item(vault, item)?;
    let (ids, saved): (Vec<_>, Vec<LoginUrl>) = current.urls().into_iter().unzip();
    let replaced: Vec<_> = superseded(&saved, identity)
        .into_iter()
        .filter_map(|i| ids.get(i).cloned())
        .collect();
    let present = saved.iter().any(|u| u.url.as_str() == url.url.as_str());
    if present && replaced.is_empty() {
        return Ok(());
    }
    session
        .update_item(vault, item, |edit| {
            for id in &replaced {
                edit.remove_url(id)?;
            }
            if !present {
                edit.add_url(&url)?;
            }
            Ok(())
        })
        .map_err(IpcError::from)
}
