//! Folder accounts on the phone: the provider sign-in, finding and joining a folder, sync,
//! password recovery, reset and sign in again, over the fake `WebDAV`, Dropbox and Graph
//! servers of `mosaic-sync` (ADR-0162). The desktop of these tests works in the directory the
//! fake server serves, like a file-sync client's copy of the folder.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mosaic_core::{AccountId, Argon2Profile, SecretKey};
use mosaic_mobile::{
    FolderProvider, ItemFilter, JoinFolderRequest, LinkKind, LinkStatus, LoginDraft, MobileError,
    OtpEdit, ResetConfirm, SessionState,
};
use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::{EngineConfig, OwnedSession, SyncEngine};
use mosaic_sync::folder::remote::credential::ProviderCredential;
use mosaic_sync::folder::remote::oauth::REDIRECT_URI;
use mosaic_sync::folder::{self, FolderConfig, FolderTransport};
use mosaic_sync::testutil::{
    AUTHORIZATION_CODE, FakeDav, FakeDropbox, FakeGraph, Faults, GRAPH_AUTHORIZATION_CODE,
};
use mosaic_sync::transport::TransportKind;
use mosaic_sync::wire::SessionCrypto;
use mosaic_sync::wire::device_info::{DeviceInfo, Platform};
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;

use crate::common::{PASSWORD, Phone, pw};

const APP_KEY: &str = "dropbox-app-key";
const MS_CLIENT: &str = "ms-client-id";
const DAV_PASSWORD: &str = "app-pass-word-123";
const NEW_PASSWORD: &str = "a completely different passphrase";

type Engine = SyncEngine<FolderTransport<SessionCrypto<OwnedSession>>>;

/// A desktop device of a folder account in `<cloud>/Mosaic`.
struct DesktopFolder {
    _dir: tempfile::TempDir,
    cloud: PathBuf,
    owned: OwnedSession,
    engine: Engine,
    account: AccountId,
    secret_key: SecretKey,
}

impl DesktopFolder {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let cloud = dir.path().join("cloud");
        let folder = cloud.join("Mosaic");
        std::fs::create_dir_all(&folder).unwrap();
        let store = Store::open(dir.path().join("desktop.db")).unwrap();
        let (mut session, created) = LockedSession::open(store)
            .create_account(&pw(PASSWORD), Argon2Profile::Baseline)
            .unwrap();
        session.create_vault("Personal").unwrap();
        let account = session.account_id();
        let install = session.install_id();
        let boot = session.account_bootstrap().unwrap();
        folder::init(&folder).unwrap();
        let owned = OwnedSession::new(session);
        let mut transport = FolderTransport::open(FolderConfig {
            root: folder,
            account_id: account,
            install_id: install,
            crypto: SessionCrypto::new(owned.clone()).unwrap(),
            clock: Arc::new(SystemClock),
            device: DeviceInfo {
                created_at_ms: 0,
                name: "Desktop".to_owned(),
                platform: Platform::Linux,
            },
        })
        .unwrap();
        transport
            .write_account_file(&boot.params, &boot.wrapped_ak, 1)
            .unwrap();
        let engine = SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Folder));
        Self {
            _dir: dir,
            cloud,
            owned,
            engine,
            account,
            secret_key: created.secret_key,
        }
    }

    fn account_hex(&self) -> String {
        self.account
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn sk_text(&self) -> String {
        self.secret_key
            .to_display()
            .unwrap()
            .expose_secret()
            .to_owned()
    }

    fn with<R>(&self, f: impl FnOnce(&mut mosaic_vault::session::UnlockedSession) -> R) -> R {
        use mosaic_sync::wire::SessionHandle as _;
        self.owned.with_session_mut(f).unwrap()
    }

    fn add_login(&mut self, title: &str) {
        let vault = self.with(|s| s.vaults()[0].vault_id);
        self.with(|s| {
            s.add_item(vault, &mosaic_core::model::ItemType::Login, |e| {
                e.set_title(title)?;
                e.set_secret(mosaic_core::model::SecretField::Password, "desk-pw-1234")?;
                Ok(())
            })
            .unwrap()
        });
        self.sync();
    }

    fn sync(&mut self) {
        let mut access = self.owned.clone();
        self.engine.sync_once(&mut access, &SystemClock).unwrap();
    }

    fn titles(&self) -> Vec<String> {
        let mut titles: Vec<String> = self.with(|s| {
            s.items(&mosaic_vault::session::ItemFilter::default())
                .iter()
                .map(|r| r.item.title().unwrap().to_owned())
                .collect()
        });
        titles.sort();
        titles
    }

    /// The desktop changes the master password: a new account file in the folder.
    fn change_password(&mut self) {
        let prepared = self
            .with(|s| {
                s.prepare_master_password_change(&pw(PASSWORD), &pw(NEW_PASSWORD), &self.secret_key)
            })
            .unwrap();
        let version = self
            .engine
            .transport_mut()
            .highest_account_version()
            .unwrap()
            + 1;
        self.engine
            .transport_mut()
            .write_account_file(prepared.new_params(), prepared.new_wrapped_ak(), version)
            .unwrap();
        self.with(|s| s.commit_master_password_change(prepared))
            .unwrap();
    }
}

/// One fake provider over the desktop's cloud directory.
enum Cloud {
    Dav(FakeDav),
    Dropbox(FakeDropbox),
    Graph(FakeGraph),
}

impl Cloud {
    fn dav(root: &Path) -> Self {
        Self::Dav(FakeDav::serve(root.to_path_buf(), "alice", DAV_PASSWORD))
    }

    fn dropbox(root: &Path) -> Self {
        let server = FakeDropbox::serve(root.to_path_buf(), "sl.access-token-1");
        server.expect_app_key(APP_KEY);
        Self::Dropbox(server)
    }

    fn graph(root: &Path) -> Self {
        let server = FakeGraph::serve(root.to_path_buf(), "ms.access-token-1");
        server.expect_client_id(MS_CLIENT);
        Self::Graph(server)
    }

    fn origin(&self) -> &str {
        match self {
            Self::Dav(s) => s.origin(),
            Self::Dropbox(s) => s.origin(),
            Self::Graph(s) => s.origin(),
        }
    }

    fn writes(&self) -> usize {
        match self {
            Self::Dav(s) => s.writes(),
            Self::Dropbox(s) => s.writes(),
            Self::Graph(s) => s.writes(),
        }
    }

    fn provider(&self) -> FolderProvider {
        match self {
            Self::Dav(_) => FolderProvider::WebDav,
            Self::Dropbox(_) => FolderProvider::Dropbox,
            Self::Graph(_) => FolderProvider::OneDrive,
        }
    }

    /// Signs the phone in (the whole flow, as the platform drives it).
    fn link(&self, phone: &Phone) {
        let core = &phone.core;
        core.test_hooks()
            .set_folder_endpoints(self.origin(), APP_KEY, MS_CLIENT);
        match self {
            Self::Dav(server) => {
                core.folder_link_password(
                    server.base_url(),
                    "alice".to_owned(),
                    DAV_PASSWORD.as_bytes().to_vec(),
                )
                .unwrap();
            }
            Self::Dropbox(_) => redirect_flow(core, FolderProvider::Dropbox, AUTHORIZATION_CODE),
            Self::Graph(_) => {
                redirect_flow(core, FolderProvider::OneDrive, GRAPH_AUTHORIZATION_CODE);
            }
        }
    }
}

fn state_of(url: &str) -> String {
    url.split(['?', '&'])
        .find_map(|p| p.strip_prefix("state="))
        .unwrap()
        .to_owned()
}

fn redirect_flow(core: &mosaic_mobile::Core, provider: FolderProvider, auth_code: &str) {
    let info = core.folder_link_begin(provider, None).unwrap();
    assert_eq!(info.kind, LinkKind::Redirect);
    let state = state_of(&info.url);
    core.folder_link_finish(format!("{REDIRECT_URI}?code={auth_code}&state={state}"))
        .unwrap();
}

fn join_request(desktop: &DesktopFolder, path: &str) -> JoinFolderRequest {
    JoinFolderRequest {
        path: path.to_owned(),
        account: desktop.account_hex(),
        password: PASSWORD.as_bytes().to_vec(),
        secret_key: desktop.sk_text().into_bytes(),
        device_name: None,
    }
}

fn titles(phone: &Phone) -> Vec<String> {
    let mut titles: Vec<String> = phone
        .core
        .items(ItemFilter::default())
        .unwrap()
        .into_iter()
        .map(|i| i.title)
        .collect();
    titles.sort();
    titles
}

/// A desktop with one item, a provider over its cloud directory, and a phone signed in and
/// joined to the folder.
fn joined(make: fn(&Path) -> Cloud) -> (DesktopFolder, Cloud, Phone) {
    let mut desktop = DesktopFolder::new();
    desktop.add_login("Desk login");
    let cloud = make(&desktop.cloud);
    let phone = Phone::android();
    cloud.link(&phone);
    let state = phone
        .core
        .join_folder(join_request(&desktop, "Mosaic"))
        .unwrap();
    assert!(matches!(state, SessionState::Unlocked { .. }), "{state:?}");
    (desktop, cloud, phone)
}

fn credential_name(desktop: &DesktopFolder) -> String {
    format!("folder-credential/{}", desktop.account_hex())
}

// ---- Task 17: the sign-in -------------------------------------------------------------------

#[test]
fn dropbox_link_begin_finish_stores_nothing_before_join() {
    let desktop = DesktopFolder::new();
    let cloud = Cloud::dropbox(&desktop.cloud);
    let phone = Phone::android();
    phone
        .core
        .test_hooks()
        .set_folder_endpoints(cloud.origin(), APP_KEY, MS_CLIENT);
    let info = phone
        .core
        .folder_link_begin(FolderProvider::Dropbox, None)
        .unwrap();
    assert_eq!(info.kind, LinkKind::Redirect);
    assert!(
        info.url.contains("code_challenge_method=S256"),
        "{}",
        info.url
    );
    assert!(info.url.contains("state="), "{}", info.url);
    assert_eq!(
        phone.core.folder_link_status().unwrap(),
        LinkStatus::Waiting
    );
    let state = state_of(&info.url);
    phone
        .core
        .folder_link_finish(format!(
            "{REDIRECT_URI}?code={AUTHORIZATION_CODE}&state={state}"
        ))
        .unwrap();
    assert_eq!(phone.core.folder_link_status().unwrap(), LinkStatus::Linked);
    // No account yet: nothing of the provider is in secure storage.
    assert!(
        phone.storage.names().is_empty(),
        "{:?}",
        phone.storage.names()
    );
}

#[test]
fn forged_callback_state_is_refused_and_drops_the_link() {
    let desktop = DesktopFolder::new();
    let cloud = Cloud::dropbox(&desktop.cloud);
    let phone = Phone::android();
    phone
        .core
        .test_hooks()
        .set_folder_endpoints(cloud.origin(), APP_KEY, MS_CLIENT);
    let info = phone
        .core
        .folder_link_begin(FolderProvider::Dropbox, None)
        .unwrap();
    let state = state_of(&info.url);
    assert_eq!(
        phone
            .core
            .folder_link_finish(format!(
                "{REDIRECT_URI}?code={AUTHORIZATION_CODE}&state=forged"
            ))
            .err(),
        Some(MobileError::FolderLinkInvalid)
    );
    // The link is gone: the right answer comes too late.
    assert_eq!(
        phone
            .core
            .folder_link_finish(format!(
                "{REDIRECT_URI}?code={AUTHORIZATION_CODE}&state={state}"
            ))
            .err(),
        Some(MobileError::FolderLinkInvalid)
    );
    // A denied sign-in ends the link too.
    phone
        .core
        .folder_link_begin(FolderProvider::Dropbox, None)
        .unwrap();
    assert_eq!(
        phone
            .core
            .folder_link_finish(format!("{REDIRECT_URI}?error=access_denied&state=x"))
            .err(),
        Some(MobileError::FolderLinkInvalid)
    );
    assert_eq!(
        phone.core.folder_link_status().err(),
        Some(MobileError::NotFound)
    );
}

#[test]
fn link_expires_after_five_minutes() {
    let desktop = DesktopFolder::new();
    let cloud = Cloud::dropbox(&desktop.cloud);
    let phone = Phone::android();
    phone
        .core
        .test_hooks()
        .set_folder_endpoints(cloud.origin(), APP_KEY, MS_CLIENT);
    let info = phone
        .core
        .folder_link_begin(FolderProvider::Dropbox, None)
        .unwrap();
    let state = state_of(&info.url);
    phone.core.test_hooks().set_clock_offset_ms(300_001);
    assert_eq!(
        phone
            .core
            .folder_link_finish(format!(
                "{REDIRECT_URI}?code={AUTHORIZATION_CODE}&state={state}"
            ))
            .err(),
        Some(MobileError::FolderLinkExpired)
    );
    // Status reports a link that ran out.
    phone.core.test_hooks().set_clock_offset_ms(0);
    phone
        .core
        .folder_link_begin(FolderProvider::Dropbox, None)
        .unwrap();
    phone.core.test_hooks().set_clock_offset_ms(300_001);
    assert_eq!(
        phone.core.folder_link_status().unwrap(),
        LinkStatus::Expired
    );
}

#[test]
fn lock_drops_link_close_keeps_it() {
    let desktop = DesktopFolder::new();
    let cloud = Cloud::dropbox(&desktop.cloud);
    let phone = Phone::android();
    cloud.link(&phone);
    phone.core.close_store();
    phone.core.reopen_store().unwrap();
    assert_eq!(phone.core.folder_link_status().unwrap(), LinkStatus::Linked);
    phone.core.lock();
    assert_eq!(
        phone.core.folder_link_status().err(),
        Some(MobileError::NotFound)
    );
    assert_eq!(
        phone.core.folder_locate(None).err(),
        Some(MobileError::NotFound)
    );
}

#[test]
fn nextcloud_poll_waits_then_links() {
    let desktop = DesktopFolder::new();
    let cloud = Cloud::dav(&desktop.cloud);
    let Cloud::Dav(server) = &cloud else {
        unreachable!()
    };
    let phone = Phone::android();
    phone
        .core
        .test_hooks()
        .set_folder_endpoints(server.origin(), APP_KEY, MS_CLIENT);
    let info = phone
        .core
        .folder_link_begin(FolderProvider::WebDav, Some(server.origin().to_owned()))
        .unwrap();
    assert_eq!(info.kind, LinkKind::Poll);
    assert!(info.url.starts_with(server.origin()), "{}", info.url);
    assert_eq!(
        phone.core.folder_link_status().unwrap(),
        LinkStatus::Waiting
    );
    server.complete_login();
    assert_eq!(phone.core.folder_link_status().unwrap(), LinkStatus::Linked);
    // Linked: the folder can be browsed.
    let found = phone.core.folder_browse(String::new()).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, "Mosaic");
    // A server that names another origin in its answer is refused, and the link is gone.
    phone.core.folder_link_cancel();
    server.set_login_lies(false, true);
    phone
        .core
        .folder_link_begin(FolderProvider::WebDav, Some(server.origin().to_owned()))
        .unwrap();
    assert_eq!(
        phone.core.folder_link_status().err(),
        Some(MobileError::FolderLinkInvalid)
    );
    assert_eq!(
        phone.core.folder_link_status().err(),
        Some(MobileError::NotFound)
    );
}

#[test]
fn provider_without_client_id_is_unsupported() {
    let desktop = DesktopFolder::new();
    let cloud = Cloud::dropbox(&desktop.cloud);
    let phone = Phone::android();
    phone
        .core
        .test_hooks()
        .set_folder_endpoints(cloud.origin(), "", "");
    for provider in [FolderProvider::Dropbox, FolderProvider::OneDrive] {
        assert_eq!(
            phone.core.folder_link_begin(provider, None).err(),
            Some(MobileError::FolderProviderUnsupported)
        );
    }
}

#[test]
fn folder_setup_code_parses_with_folder_info() {
    use mosaic_core::setup_code::{FolderProviderHint, FolderSetupCode, build_folder};
    let phone = Phone::android();
    let secret_key = SecretKey::generate().unwrap();
    let text = build_folder(&FolderSetupCode {
        account: [0xab; 16],
        provider: FolderProviderHint::WebDav,
        server: "https://cloud.example.com".to_owned(),
        path: "/Mosaic/Family".to_owned(),
        secret_key: SecretKey::parse(secret_key.to_display().unwrap().expose_secret()).unwrap(),
    })
    .unwrap();
    let code = phone
        .core
        .parse_setup_code(text.as_bytes().to_vec())
        .unwrap();
    assert_eq!(code.server, "");
    assert_eq!(code.email, "");
    assert_eq!(
        code.secret_key.expose_bytes(),
        secret_key.to_display().unwrap().expose_secret().as_bytes()
    );
    let folder = code.folder.unwrap();
    assert_eq!(folder.account, "ab".repeat(16));
    assert_eq!(folder.provider, Some(FolderProvider::WebDav));
    assert_eq!(folder.server.as_deref(), Some("https://cloud.example.com"));
    assert_eq!(folder.path.as_deref(), Some("/Mosaic/Family"));
    // "other" gives no provider and no server.
    let other = build_folder(&FolderSetupCode {
        account: [1; 16],
        provider: FolderProviderHint::Other,
        server: String::new(),
        path: String::new(),
        secret_key: SecretKey::generate().unwrap(),
    })
    .unwrap();
    let folder = phone
        .core
        .parse_setup_code(other.as_bytes().to_vec())
        .unwrap()
        .folder
        .unwrap();
    assert_eq!(
        (folder.provider, folder.server, folder.path),
        (None, None, None)
    );
}

// ---- Task 18: find, browse, join ------------------------------------------------------------

#[test]
fn join_folder_over_dav_then_items_visible() {
    let (desktop, cloud, phone) = joined(Cloud::dav);
    // The first pull ran at the join.
    assert_eq!(titles(&phone), vec!["Desk login".to_owned()]);
    let overview = phone.core.sync_overview();
    assert!(overview.configured && overview.signed_in);
    assert_eq!(overview.last_error, None);
    let folder = overview.folder.unwrap();
    assert_eq!(folder.provider, cloud.provider());
    assert_eq!(folder.path, "/Mosaic");
    assert_eq!(overview.reset_phrase, desktop.account_hex()[..8]);
    assert!(phone.storage.names().contains(&credential_name(&desktop)));
    assert!(
        phone
            .storage
            .names()
            .contains(&format!("secret-key/{}", desktop.account_hex()))
    );
    // A second join is refused.
    assert_eq!(
        phone
            .core
            .join_folder(join_request(&desktop, "Mosaic"))
            .err(),
        Some(MobileError::AccountExists)
    );
}

#[test]
fn choosing_a_wrong_folder_writes_nothing() {
    let mut desktop = DesktopFolder::new();
    desktop.add_login("Desk login");
    let other = DesktopFolder::new();
    // The served directory holds the real folder, an empty one and a folder of another account.
    std::fs::create_dir_all(desktop.cloud.join("Empty")).unwrap();
    std::fs::create_dir_all(desktop.cloud.join("Other")).unwrap();
    copy_dir(&other.cloud.join("Mosaic"), &desktop.cloud.join("Other"));
    let cloud = Cloud::dav(&desktop.cloud);
    let phone = Phone::android();
    cloud.link(&phone);
    let writes = cloud.writes();

    // An empty folder: no candidate, and a join says the folder is not there.
    assert_eq!(
        phone.core.folder_locate(Some("/Empty".to_owned())).unwrap(),
        vec![]
    );
    assert_eq!(
        phone
            .core
            .join_folder(join_request(&desktop, "Empty"))
            .err(),
        Some(MobileError::FolderNotFound)
    );
    // A Mosaic folder of another account.
    let candidates = phone.core.folder_locate(Some("/Other".to_owned())).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].accounts, vec![other.account_hex()]);
    assert_eq!(
        phone
            .core
            .join_folder(join_request(&desktop, "Other"))
            .err(),
        Some(MobileError::SyncFolderNoAccount)
    );
    // The parent of the real folder.
    assert_eq!(phone.core.folder_locate(None).unwrap(), vec![]);
    assert_eq!(
        phone.core.join_folder(join_request(&desktop, "")).err(),
        Some(MobileError::FolderNotFound)
    );
    // A folder that does not exist.
    assert_eq!(
        phone.core.folder_locate(Some("/Nope".to_owned())).unwrap(),
        vec![]
    );
    // A wrong password: the link stays usable for a retry.
    let mut wrong = join_request(&desktop, "Mosaic");
    wrong.password = b"not the password".to_vec();
    assert_eq!(
        phone.core.join_folder(wrong).err(),
        Some(MobileError::WrongCredentials)
    );
    assert_eq!(phone.core.state(), SessionState::NoAccount);
    assert_eq!(cloud.writes(), writes, "the phone wrote nothing");

    // The right folder is found, and the retry joins.
    let found = phone
        .core
        .folder_locate(Some("/Mosaic".to_owned()))
        .unwrap();
    assert_eq!(found[0].accounts, vec![desktop.account_hex()]);
    phone
        .core
        .join_folder(join_request(&desktop, "Mosaic"))
        .unwrap();
}

fn copy_dir(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn browse_lists_folders_sorted_and_caps_at_200() {
    let desktop = DesktopFolder::new();
    for i in 0..205 {
        std::fs::create_dir_all(desktop.cloud.join(format!("dir{i:03}"))).unwrap();
    }
    std::fs::write(desktop.cloud.join("a-file.txt"), b"x").unwrap();
    for make in [
        Cloud::dav as fn(&Path) -> Cloud,
        Cloud::dropbox,
        Cloud::graph,
    ] {
        let cloud = make(&desktop.cloud);
        let phone = Phone::android();
        cloud.link(&phone);
        let listed = phone.core.folder_browse(String::new()).unwrap();
        if cloud.provider() == FolderProvider::OneDrive {
            // Graph answers one page of 200 items, files included, before the phone sorts.
            assert!((190..=200).contains(&listed.len()), "{}", listed.len());
        } else {
            assert_eq!(listed.len(), 200, "{:?}", cloud.provider());
            assert_eq!(listed[0].name, "Mosaic");
        }
        assert!(
            listed
                .iter()
                .all(|e| e.name == e.path && e.name != "a-file.txt")
        );
        let sorted = {
            let mut names: Vec<_> = listed.iter().map(|e| e.name.clone()).collect();
            names.sort();
            names
        };
        assert_eq!(
            sorted,
            listed.iter().map(|e| e.name.clone()).collect::<Vec<_>>()
        );
        // A subfolder lists below its path.
        let below = phone.core.folder_browse("dir000".to_owned()).unwrap();
        assert_eq!(below, vec![]);
        assert_eq!(
            phone.core.folder_browse("a/../b".to_owned()).err(),
            Some(MobileError::InvalidInput)
        );
    }
}

#[test]
fn join_folder_without_link_is_not_found() {
    let desktop = DesktopFolder::new();
    let phone = Phone::android();
    assert_eq!(
        phone
            .core
            .join_folder(join_request(&desktop, "Mosaic"))
            .err(),
        Some(MobileError::NotFound)
    );
    assert_eq!(
        phone.core.folder_browse(String::new()).err(),
        Some(MobileError::NotFound)
    );
    assert_eq!(
        phone.core.folder_locate(None).err(),
        Some(MobileError::NotFound)
    );
}

// ---- Task 19: sync, recovery, reset, sign in again ------------------------------------------

#[test]
fn folder_sync_round_trip_dav_dropbox_graph() {
    for make in [
        Cloud::dav as fn(&Path) -> Cloud,
        Cloud::dropbox,
        Cloud::graph,
    ] {
        let (mut desktop, cloud, phone) = joined(make);
        assert_eq!(
            titles(&phone),
            vec!["Desk login".to_owned()],
            "{:?}",
            cloud.provider()
        );
        // A phone edit reaches the desktop.
        let vault = phone.vault_id();
        phone
            .core
            .save_login(LoginDraft {
                vault_id: vault,
                item_id: None,
                title: "Phone login".to_owned(),
                username: "carol".to_owned(),
                password: Some(b"phone-pw-9876".to_vec()),
                urls: vec![],
                notes: None,
                otp: OtpEdit::Keep,
                favorite: false,
            })
            .unwrap();
        let overview = phone.core.sync_now().unwrap();
        assert_eq!(overview.pending_uploads, 0);
        desktop.sync();
        assert_eq!(
            desktop.titles(),
            vec!["Desk login".to_owned(), "Phone login".to_owned()],
            "{:?}",
            cloud.provider()
        );
        // A desktop edit reaches the phone.
        desktop.add_login("Second desk login");
        phone.core.sync_now().unwrap();
        assert_eq!(titles(&phone).len(), 3);
        // A rotated refresh token of Graph is in secure storage.
        if let Cloud::Graph(server) = &cloud {
            let stored = phone
                .storage
                .map
                .lock()
                .unwrap()
                .get(&credential_name(&desktop))
                .cloned()
                .unwrap();
            let ProviderCredential::Graph { refresh_token } =
                ProviderCredential::decode(&stored).unwrap()
            else {
                panic!("not a Graph credential");
            };
            assert_eq!(*refresh_token, server.refresh_token());
        }
    }
}

#[test]
fn engine_kept_between_runs_does_not_redownload_segments() {
    let (mut desktop, cloud, phone) = joined(Cloud::dav);
    let Cloud::Dav(server) = &cloud else {
        unreachable!()
    };
    desktop.add_login("Second");
    phone.core.sync_now().unwrap();
    let gets = |server: &FakeDav| {
        server
            .requests()
            .iter()
            .filter(|r| r.method == "GET" && r.path_and_query.contains(".mseg"))
            .count()
    };
    let before = gets(server);
    assert!(before > 0);
    phone.core.sync_now().unwrap();
    assert_eq!(gets(server), before, "the second run read no segment again");
    assert_eq!(titles(&phone).len(), 2);
}

#[test]
fn lock_drops_the_kept_engine_and_access_token() {
    let (_desktop, cloud, phone) = joined(Cloud::dropbox);
    let Cloud::Dropbox(server) = &cloud else {
        unreachable!()
    };
    phone.core.sync_now().unwrap();
    phone.core.sync_now().unwrap();
    let refreshes = server.refreshes();
    // The kept engine keeps its access token: no refresh between runs.
    phone.core.sync_now().unwrap();
    assert_eq!(server.refreshes(), refreshes);
    phone.core.lock();
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    phone.core.sync_now().unwrap();
    assert!(
        server.refreshes() > refreshes,
        "the engine and its access token were dropped by the lock"
    );
}

#[test]
fn signed_out_provider_reports_and_relink_restores() {
    let (_desktop, cloud, phone) = joined(Cloud::dropbox);
    let Cloud::Dropbox(server) = &cloud else {
        unreachable!()
    };
    phone.core.sync_now().unwrap();
    server.revoke_after(0);
    assert_eq!(
        phone.core.sync_now().err(),
        Some(MobileError::FolderProviderSignedOut)
    );
    let overview = phone.core.sync_overview();
    assert!(!overview.signed_in);
    assert_eq!(
        overview.last_error,
        Some(MobileError::FolderProviderSignedOut)
    );
    // A sign in to another provider is refused.
    assert_eq!(
        phone
            .core
            .folder_link_begin(FolderProvider::OneDrive, None)
            .err(),
        Some(MobileError::InvalidInput)
    );
    // Signing in again restores the sync.
    redirect_flow(&phone.core, FolderProvider::Dropbox, AUTHORIZATION_CODE);
    assert!(phone.core.sync_overview().signed_in);
    phone.core.sync_now().unwrap();
    assert_eq!(phone.core.sync_overview().last_error, None);
}

#[test]
fn relink_to_folder_without_account_stores_nothing() {
    let (desktop, cloud, phone) = joined(Cloud::dav);
    let Cloud::Dav(server) = &cloud else {
        unreachable!()
    };
    phone.core.sync_now().unwrap();
    let before = phone.storage.names();
    // The account's folder is gone from the server.
    std::fs::rename(desktop.cloud.join("Mosaic"), desktop.cloud.join("Moved")).unwrap();
    assert_eq!(
        phone
            .core
            .folder_link_password(
                server.base_url(),
                "alice".to_owned(),
                b"other-app-password".to_vec()
            )
            .err(),
        Some(MobileError::FolderProviderSignedOut)
    );
    assert_eq!(
        phone
            .core
            .folder_link_password(
                server.base_url(),
                "alice".to_owned(),
                DAV_PASSWORD.as_bytes().to_vec()
            )
            .err(),
        Some(MobileError::SyncFolderNoAccount)
    );
    assert_eq!(phone.storage.names(), before);
    let stored = phone
        .storage
        .map
        .lock()
        .unwrap()
        .get(&credential_name(&desktop))
        .cloned()
        .unwrap();
    let ProviderCredential::WebDav { app_password, .. } =
        ProviderCredential::decode(&stored).unwrap()
    else {
        panic!("not a WebDAV credential");
    };
    assert_eq!(app_password.as_str(), DAV_PASSWORD);
    // Another server address is not the account's.
    assert_eq!(
        phone
            .core
            .folder_link_password(
                "https://other.example.com/dav".to_owned(),
                "alice".to_owned(),
                b"x".to_vec()
            )
            .err(),
        Some(MobileError::InvalidInput)
    );
}

#[test]
fn folder_password_recovery_adopts_newer_file_and_refuses_rollback() {
    let (mut desktop, _cloud, phone) = joined(Cloud::dav);
    desktop.change_password();
    // The old password does not open the new file.
    assert_eq!(
        phone
            .core
            .recover_password(PASSWORD.as_bytes().to_vec())
            .err(),
        Some(MobileError::WrongCredentials)
    );
    let overview = phone
        .core
        .recover_password(NEW_PASSWORD.as_bytes().to_vec())
        .unwrap();
    assert_eq!(overview.last_error, None);
    // The adopted version is the highest now: the same file again is a rollback.
    assert_eq!(
        phone
            .core
            .recover_password(NEW_PASSWORD.as_bytes().to_vec())
            .err(),
        Some(MobileError::SyncPasswordRollback)
    );
    // The phone unlocks with the new password.
    phone.core.lock();
    phone
        .core
        .unlock_password(NEW_PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    phone.core.sync_now().unwrap();
}

#[test]
fn folder_reset_uses_short_id_phrase_and_revokes_dropbox() {
    let (desktop, cloud, phone) = joined(Cloud::dropbox);
    let Cloud::Dropbox(server) = &cloud else {
        unreachable!()
    };
    let phrase = &desktop.account_hex()[..8];
    // Neither an email nor a wrong phrase resets.
    for typed in ["user@example.com", "00000000", ""] {
        assert_eq!(
            phone
                .core
                .reset_device(ResetConfirm {
                    typed_email: typed.to_owned()
                })
                .err(),
            Some(MobileError::InvalidInput),
            "{typed}"
        );
    }
    assert!(!server.revoked());
    let outcome = phone
        .core
        .reset_device(ResetConfirm {
            typed_email: format!(" {} ", phrase.to_uppercase()),
        })
        .unwrap();
    assert_eq!(outcome.state, SessionState::NoAccount);
    assert!(server.revoked(), "the grant was revoked at Dropbox");
    assert!(
        phone.storage.names().is_empty(),
        "{:?}",
        phone.storage.names()
    );
    assert_eq!(phone.core.sync_overview().folder, None);
}

#[test]
fn storage_full_maps_to_folder_storage_full() {
    let (_desktop, cloud, phone) = joined(Cloud::dav);
    let Cloud::Dav(server) = &cloud else {
        unreachable!()
    };
    server.set_faults(Faults {
        fail_nth: Some((1, 507, None)),
        ..Faults::default()
    });
    assert_eq!(
        phone.core.sync_now().err(),
        Some(MobileError::FolderStorageFull)
    );
    // The next run goes on.
    phone.core.sync_now().unwrap();
}

#[test]
fn synced_folder_account_survives_a_reopen_and_needs_no_relink() {
    let (_desktop, cloud, mut phone) = joined(Cloud::dav);
    phone.reopen_as(
        mosaic_mobile::Platform::Android,
        mosaic_mobile::CoreContext::App,
        &[0, 0, 0, 7],
    );
    // The core of the new process knows the credential is there.
    assert_ne!(phone.core.state(), SessionState::NoAccount);
    phone
        .core
        .test_hooks()
        .set_folder_endpoints(cloud.origin(), APP_KEY, MS_CLIENT);
    phone
        .core
        .unlock_password(PASSWORD.as_bytes().to_vec(), None)
        .unwrap();
    assert!(phone.core.sync_overview().signed_in);
    phone.core.sync_now().unwrap();
}
