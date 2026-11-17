//! The vault-backed source against a real temporary store (design §6; builder checks B1, B4;
//! review focus 3).

use mosaic_core::model::{
    AgentBasicUser, AgentBindingRecord, AgentInject, CustomField, CustomFieldValue, ElementId,
    ItemType, SecretField, Text, TextField,
};
use mosaic_core::{
    AccountId, Argon2Profile, ItemId, MasterPassword, SecretKey, SecretString, VaultId,
};
use mosaic_vault::refs::RefCandidate;
use mosaic_vault::session::{LockedSession, UnlockedSession};
use mosaic_vault::storage::Store;

use crate::error::AgentErrorCode;
use crate::source::{ResolveError, SecretSource};
use crate::vault_source::VaultSource;

const VALUE: &str = "SK-TEST-CANARY-0123456789abcdef";
const ORIGIN: &str = "https://api.example.com";

fn password() -> MasterPassword {
    MasterPassword::new("unit test password".to_owned())
}

fn account() -> (UnlockedSession, SecretKey) {
    let (session, created) = LockedSession::open(Store::open_in_memory().unwrap())
        .create_account(&password(), Argon2Profile::Baseline)
        .unwrap();
    (session, created.secret_key)
}

fn record(field: &str) -> AgentBindingRecord {
    AgentBindingRecord::new(field, &[ORIGIN], AgentInject::Bearer)
}

/// An API key item with its secret and one binding; returns the item and the binding's ID.
fn api_item(session: &mut UnlockedSession, vault: VaultId, title: &str) -> (ItemId, ElementId) {
    let item = session
        .add_item(vault, &ItemType::ApiKey, |e| {
            e.set_title(title)?;
            e.set_secret(SecretField::ApiKeySecret, VALUE)?;
            e.add_agent_binding(&record("api_key.secret"))?;
            Ok(())
        })
        .unwrap();
    let id = session.item(vault, item).unwrap().agent_bindings()[0]
        .0
        .clone();
    (item, id)
}

fn confirm(session: &mut UnlockedSession, vault: VaultId, item: ItemId, id: &ElementId) {
    let rec = session
        .item(vault, item)
        .unwrap()
        .agent_bindings()
        .into_iter()
        .find(|(i, _)| i == id)
        .unwrap()
        .1;
    let canonical = rec.canonical_json().unwrap();
    session
        .confirm_agent_mark(vault, item, id.as_str(), canonical.expose_secret())
        .unwrap();
}

fn reference(vault: VaultId, item: ItemId, field: &str) -> String {
    format!("mosaic://{vault}/{item}/{field}")
}

#[test]
fn shared_vault_bindings_are_not_usable() {
    let (mut session, secret_key) = account();
    let own = session.create_vault("Own").unwrap();
    let shared = session.create_vault("Shared").unwrap();
    let foreign = session.create_vault("Foreign").unwrap();
    let (own_item, _) = api_item(&mut session, own, "Key");
    let (shared_item, _) = api_item(&mut session, shared, "Key");
    let (foreign_item, _) = api_item(&mut session, foreign, "Key");
    let me: AccountId = session.account_id();

    // The same bindings in a vault this account shares, and one that someone else owns.
    let mut store = session.lock().into_store();
    store.set_vault_sharing(&shared, 1, &me, false).unwrap();
    store
        .set_vault_sharing(&foreign, 3, &AccountId::new_v7().unwrap(), false)
        .unwrap();
    let session = LockedSession::open(store)
        .unlock(&password(), &secret_key)
        .ok()
        .unwrap();
    let source = VaultSource::new(&session);
    let usable = source.usable_bindings();
    assert_eq!(usable.len(), 1);
    assert_eq!((usable[0].vault_id, usable[0].item_id), (own, own_item));
    for (vault, item) in [(shared, shared_item), (foreign, foreign_item)] {
        assert_eq!(
            source
                .resolve(&reference(vault, item, "api_key.secret"))
                .err(),
            Some(ResolveError::NotUsable)
        );
    }
    // The shared vault's name does not resolve a reference either.
    assert_eq!(
        source.resolve("mosaic://Shared/Key/api_key.secret").err(),
        Some(ResolveError::NotUsable)
    );
    // And its value is not readable through a binding built elsewhere.
    let mut forged = usable[0].clone();
    forged.vault_id = shared;
    forged.item_id = shared_item;
    assert_eq!(
        source.read_value(&forged).err(),
        Some(AgentErrorCode::NotAgentUsable)
    );
}

#[test]
fn unconfirmed_binding_is_listed_but_not_confirmed() {
    let (mut session, _) = account();
    let vault = session.create_vault("V").unwrap();
    let (item, id) = api_item(&mut session, vault, "Key");
    {
        let source = VaultSource::new(&session);
        let listed = source.usable_bindings();
        assert_eq!(listed.len(), 1);
        assert!(!listed[0].confirmed);
        assert_eq!(listed[0].field_label, "api_key.secret");
        assert_eq!(listed[0].vault_name, "V");
        assert_eq!(listed[0].item_title, "Key");
        assert!(
            !source
                .resolve(&reference(vault, item, "api_key.secret"))
                .unwrap()
                .confirmed
        );
    }
    confirm(&mut session, vault, item, &id);
    assert!(VaultSource::new(&session).usable_bindings()[0].confirmed);

    // A changed binding (a new destination) is no longer confirmed.
    session
        .update_item(vault, item, |e| {
            let changed = AgentBindingRecord::new(
                "api_key.secret",
                &[ORIGIN, "https://other.example.com"],
                AgentInject::Bearer,
            );
            e.set_agent_binding(&id, &changed)
        })
        .unwrap();
    assert!(!VaultSource::new(&session).usable_bindings()[0].confirmed);
}

#[test]
fn text_fields_and_missing_fields_are_not_usable() {
    let (mut session, _) = account();
    let vault = session.create_vault("V").unwrap();
    let item = session
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title("Login")?;
            e.set_text(TextField::Username, "alice")?;
            e.set_secret(SecretField::Password, VALUE)?;
            e.add_agent_binding(&record("login.username"))?; // a text field
            e.add_agent_binding(&record("login.password"))?;
            e.add_agent_binding(&record("api_key.secret"))?; // no such value on this item
            e.add_agent_binding(&record("field:nothing"))?; // no such custom field
            Ok(())
        })
        .unwrap();
    let source = VaultSource::new(&session);
    let usable = source.usable_bindings();
    assert_eq!(usable.len(), 1, "only the password binding is usable");
    assert_eq!(usable[0].field_label, "login.password");
    for field in [
        "username",
        "login.username",
        "api_key.secret",
        "field:nothing",
        "nope",
        "notes",
    ] {
        assert_eq!(
            source.resolve(&reference(vault, item, field)).err(),
            Some(ResolveError::NotUsable),
            "{field}"
        );
    }
    // The CLI's alias names the login password; the full key does too.
    for field in ["password", "login.password"] {
        assert!(
            source.resolve(&reference(vault, item, field)).is_ok(),
            "{field}"
        );
    }
    // The field part is required.
    assert_eq!(
        source.resolve(&format!("mosaic://{vault}/{item}")).err(),
        Some(ResolveError::Invalid)
    );
    assert_eq!(source.resolve("Login").err(), Some(ResolveError::Invalid));
}

#[test]
fn hidden_custom_field_by_label_and_by_element_id() {
    let (mut session, _) = account();
    let vault = session.create_vault("V").unwrap();
    let hidden = |label: &str| {
        CustomField::new(
            Text::new(label),
            CustomFieldValue::Hidden(SecretString::from_str_copy(VALUE).unwrap()),
        )
    };
    let item = session
        .add_item(vault, &ItemType::SecureNote, |e| e.set_title("Tokens"))
        .unwrap();
    let mut ids = Vec::new();
    session
        .update_item(vault, item, |e| {
            ids.push(e.add_custom_field(&hidden("Token"))?);
            ids.push(e.add_custom_field(&hidden("Other"))?);
            ids.push(e.add_custom_field(&hidden("Other"))?);
            ids.push(e.add_custom_field(&CustomField::new(
                Text::new("Plain"),
                CustomFieldValue::Text(Text::new("not secret")),
            ))?);
            for id in &ids {
                e.add_agent_binding(&record(&format!("field:{id}")))?;
            }
            Ok(())
        })
        .unwrap();
    let source = VaultSource::new(&session);
    let usable = source.usable_bindings();
    assert_eq!(usable.len(), 3, "the text field is not usable");
    // By element ID, as `listSecrets` names it.
    let by_id = source
        .resolve(&reference(vault, item, &format!("field:{}", ids[0])))
        .unwrap();
    assert_eq!(by_id.field_label, "Token");
    // By label.
    let by_label = source
        .resolve(&reference(vault, item, "field:Token"))
        .unwrap();
    assert_eq!(by_label.element_id, by_id.element_id);
    // Two fields with one label: ambiguous by label, exact by ID.
    match source.resolve(&reference(vault, item, "field:Other")) {
        Err(ResolveError::Ambiguous(c)) => assert_eq!(c.len(), 2),
        other => panic!("expected ambiguous, got {other:?}"),
    }
    assert!(
        source
            .resolve(&reference(vault, item, &format!("field:{}", ids[1])))
            .is_ok()
    );
    // The plain text custom field is not usable, by label or by ID.
    for field in ["field:Plain".to_owned(), format!("field:{}", ids[3])] {
        assert_eq!(
            source.resolve(&reference(vault, item, &field)).err(),
            Some(ResolveError::NotUsable)
        );
    }
    // The value comes from the field.
    let value = source.read_value(&by_id).unwrap();
    assert_eq!(value.expose_secret(), VALUE);
}

#[test]
fn binding_follows_rename_and_refuses_move_and_short_value() {
    let (mut session, _) = account();
    let vault = session.create_vault("V").unwrap();
    let other = session.create_vault("Elsewhere").unwrap();
    let (item, id) = api_item(&mut session, vault, "Old name");
    confirm(&mut session, vault, item, &id);
    let uuid_ref = reference(vault, item, "api_key.secret");

    // Renamed: the UUID reference still resolves, the old name does not.
    session
        .update_item(vault, item, |e| e.set_title("New name"))
        .unwrap();
    {
        let source = VaultSource::new(&session);
        let resolved = source.resolve(&uuid_ref).unwrap();
        assert_eq!(resolved.item_title, "New name");
        assert!(resolved.confirmed, "a rename does not change the binding");
        assert_eq!(
            source.resolve("mosaic://V/Old%20name/api_key.secret").err(),
            Some(ResolveError::NotUsable)
        );
        assert!(
            source
                .resolve("mosaic://V/New%20name/api_key.secret")
                .is_ok()
        );
    }

    // A value of 11 bytes, or one with a newline, is refused at read time, never sent partly.
    for bad in ["short-11-by", "twelve-bytes-\nand-a-newline"] {
        session
            .update_item(vault, item, |e| {
                e.set_secret(SecretField::ApiKeySecret, bad)
            })
            .unwrap();
        let source = VaultSource::new(&session);
        let resolved = source.resolve(&uuid_ref).unwrap();
        assert_eq!(
            source.read_value(&resolved).err(),
            Some(AgentErrorCode::NotAgentUsable),
            "{bad:?}"
        );
    }
    session
        .update_item(vault, item, |e| {
            e.set_secret(SecretField::ApiKeySecret, VALUE)
        })
        .unwrap();
    {
        let source = VaultSource::new(&session);
        let resolved = source.resolve(&uuid_ref).unwrap();
        assert_eq!(source.read_value(&resolved).unwrap().expose_secret(), VALUE);
        // A binding resolved before an edit does not read the edited binding's value.
        stale_binding_is_refused(&session, &resolved);
    }

    // Moved to another vault: the old reference is not usable, the copy needs review there.
    let moved = session.move_item(vault, item, other).unwrap();
    let source = VaultSource::new(&session);
    assert_eq!(
        source.resolve(&uuid_ref).err(),
        Some(ResolveError::NotUsable)
    );
    let at_new = source
        .resolve(&reference(other, moved, "api_key.secret"))
        .unwrap();
    assert!(
        !at_new.confirmed,
        "a moved item is a new record: review again"
    );
}

/// Reading with a stale `SourceBinding` after the policy changed is refused.
fn stale_binding_is_refused(session: &UnlockedSession, resolved: &crate::source::SourceBinding) {
    let mut stale = resolved.clone();
    stale.binding.digest = [0; 32];
    assert_eq!(
        VaultSource::new(session).read_value(&stale).err(),
        Some(AgentErrorCode::NotAgentUsable)
    );
}

#[test]
fn trashed_items_are_not_usable() {
    let (mut session, _) = account();
    let vault = session.create_vault("V").unwrap();
    let (item, _) = api_item(&mut session, vault, "Key");
    let uuid_ref = reference(vault, item, "api_key.secret");
    let resolved = VaultSource::new(&session).resolve(&uuid_ref).unwrap();
    session.trash_item(vault, item).unwrap();
    {
        let source = VaultSource::new(&session);
        assert_eq!(source.usable_bindings().len(), 0);
        assert_eq!(
            source.resolve(&uuid_ref).err(),
            Some(ResolveError::NotUsable)
        );
        assert_eq!(
            source.read_value(&resolved).err(),
            Some(AgentErrorCode::NotAgentUsable)
        );
    }
    session.restore_item(vault, item).unwrap();
    assert_eq!(VaultSource::new(&session).usable_bindings().len(), 1);
    session.purge_item(vault, item).unwrap();
    let source = VaultSource::new(&session);
    assert_eq!(source.usable_bindings().len(), 0);
    assert_eq!(
        source.resolve(&uuid_ref).err(),
        Some(ResolveError::NotUsable)
    );
}

#[test]
fn ambiguous_candidates_only_name_usable_items() {
    let (mut session, _) = account();
    let vault = session.create_vault("V").unwrap();
    let (first, _) = api_item(&mut session, vault, "Key");
    let (second, _) = api_item(&mut session, vault, "Key");
    // A third item with the same title and no binding, and a fourth whose binding is unusable.
    let plain = session
        .add_item(vault, &ItemType::ApiKey, |e| {
            e.set_title("Key")?;
            e.set_secret(SecretField::ApiKeySecret, VALUE)
        })
        .unwrap();
    let broken = session
        .add_item(vault, &ItemType::ApiKey, |e| {
            e.set_title("Key")?;
            e.set_secret(SecretField::ApiKeySecret, VALUE)?;
            e.add_agent_binding(&AgentBindingRecord::new(
                "api_key.secret",
                &["http://api.example.com"], // not https
                AgentInject::Bearer,
            ))?;
            Ok(())
        })
        .unwrap();
    let source = VaultSource::new(&session);
    match source.resolve("mosaic://V/Key/api_key.secret") {
        Err(ResolveError::Ambiguous(candidates)) => {
            let mut ids: Vec<String> = candidates
                .iter()
                .map(|c: &RefCandidate| c.id.to_string())
                .collect();
            ids.sort();
            let mut want = vec![first.to_string(), second.to_string()];
            want.sort();
            assert_eq!(ids, want, "only usable items are named");
            assert!(!ids.contains(&plain.to_string()) && !ids.contains(&broken.to_string()));
        }
        other => panic!("expected ambiguous, got {other:?}"),
    }
}

#[test]
fn basic_user_comes_from_the_binding_or_a_text_field() {
    let (mut session, _) = account();
    let vault = session.create_vault("V").unwrap();
    let basic = |user: AgentBasicUser| {
        AgentBindingRecord::new("login.password", &[ORIGIN], AgentInject::Basic { user })
    };
    let make = |session: &mut UnlockedSession, user: AgentBasicUser, name: &str| {
        session
            .add_item(vault, &ItemType::Login, |e| {
                e.set_title(name)?;
                e.set_text(TextField::Username, name)?;
                e.set_secret(SecretField::Password, VALUE)?;
                e.add_agent_binding(&basic(user))?;
                Ok(())
            })
            .unwrap()
    };
    let literal = make(
        &mut session,
        AgentBasicUser::Literal(Text::new("svc")),
        "alice",
    );
    let field = make(
        &mut session,
        AgentBasicUser::Field(Text::new(TextField::Username.key())),
        "bob",
    );
    let colon = make(
        &mut session,
        AgentBasicUser::Field(Text::new(TextField::Username.key())),
        "a:b",
    );
    let secret_user = make(&mut session, AgentBasicUser::SecretIsUser, "carol");
    let source = VaultSource::new(&session);
    let user_of = |item: ItemId| {
        let b = source
            .resolve(&reference(vault, item, "login.password"))
            .unwrap();
        source.basic_user(&b)
    };
    assert_eq!(user_of(literal), Ok(Some("svc".to_owned())));
    assert_eq!(user_of(field), Ok(Some("bob".to_owned())));
    assert_eq!(user_of(colon), Err(AgentErrorCode::NotAgentUsable));
    assert_eq!(user_of(secret_user), Ok(None));
}

/// A vault of mixed items lists the same bindings, in item ID order, and every reference
/// resolves as it did when each request listed all items first.
#[test]
fn a_mixed_vault_lists_and_resolves_the_same_bindings() {
    let (mut session, _) = account();
    let vault = session.create_vault("Mixed").unwrap();
    let mut usable: Vec<(ItemId, ElementId)> = Vec::new();
    let mut unusable: Vec<ItemId> = Vec::new();
    for n in 0..12 {
        // Items without a binding, some with custom fields (skipped before decoding them).
        let plain = session
            .add_item(vault, &ItemType::Login, |e| {
                e.set_title(&format!("Plain {n}"))?;
                e.add_custom_field(&CustomField::new(
                    Text::new("Note"),
                    CustomFieldValue::Text(Text::new("x")),
                ))?;
                Ok(())
            })
            .unwrap();
        unusable.push(plain);
        usable.push(api_item(&mut session, vault, &format!("Key {n}")));
        // A binding to a field the item does not have.
        let missing = session
            .add_item(vault, &ItemType::ApiKey, |e| {
                e.set_title(&format!("Empty {n}"))?;
                e.add_agent_binding(&record("api_key.secret"))?;
                Ok(())
            })
            .unwrap();
        unusable.push(missing);
    }
    // A trashed item with a usable binding.
    let (trashed, _) = api_item(&mut session, vault, "Trashed");
    session.trash_item(vault, trashed).unwrap();
    unusable.push(trashed);

    let source = VaultSource::new(&session);
    let listed: Vec<(ItemId, ElementId)> = source
        .usable_bindings()
        .into_iter()
        .map(|b| (b.item_id, b.element_id))
        .collect();
    // In ascending item ID order, which is the order of the ID strings.
    usable.sort_by_key(|(item, _)| item.to_string());
    assert_eq!(listed, usable);
    assert!(
        listed
            .windows(2)
            .all(|w| w[0].0.to_string() < w[1].0.to_string())
    );

    for (item, element) in &usable {
        let resolved = source
            .resolve(&reference(vault, *item, "api_key.secret"))
            .unwrap();
        assert_eq!(
            (resolved.item_id, resolved.element_id),
            (*item, element.clone())
        );
    }
    for item in &unusable {
        assert_eq!(
            source
                .resolve(&reference(vault, *item, "api_key.secret"))
                .err(),
            Some(ResolveError::NotUsable)
        );
    }
}
