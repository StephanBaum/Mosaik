//! M3 red team, second pass (2026-11-04): the Tauri surface around the
//! commands, not the commands themselves (those are in `redteam_tests`).
//! Each test names what it attempts; all of them must hold:
//! - no origin but the bundled app, and no webview but `main`, reaches IPC;
//! - no core or plugin command is reachable, and a forged invoke key is refused;
//! - no webview navigates away from the app (the CSP does not cover that);
//! - the config and the dependency list keep devtools, remote IPC, the asset
//!   protocol and URL-opening plugins out of the build;
//! - tagged request values refuse unknown fields like every other request.
//!
//! Limits: the mock runtime never calls navigation handlers, so the guard is
//! called through its `Plugin` hook, and its registration in `run()` is
//! checked in the source.

use mosaic_core::Argon2Profile;
use mosaic_vault::session::LockedSession;
use mosaic_vault::storage::Store;
use serde_json::{Value, json};
use tauri::plugin::Plugin as _;
use tauri::test::{INVOKE_KEY, MockRuntime};
use tauri::{App, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::keystore::MemoryKeystore;
use crate::state::{AppState, Backend, Slot};

/// The origin the bundled frontend is served from on this OS.
const LOCAL: &str = if cfg!(windows) {
    "http://tauri.localhost/"
} else {
    "tauri://localhost/"
};

fn mock_app() -> App<MockRuntime> {
    let store = Store::open_in_memory().unwrap();
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(store)),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    let app = tauri::test::mock_builder()
        .invoke_handler(crate::commands::handler())
        .build(crate::context())
        .unwrap();
    tauri::Manager::manage(&app, AppState::new(backend));
    app
}

fn window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::default())
        .build()
        .unwrap()
}

fn invoke(
    webview: &WebviewWindow<MockRuntime>,
    cmd: &str,
    url: &str,
    body: Value,
    key: &str,
) -> Result<tauri::ipc::InvokeResponseBody, Value> {
    tauri::test::get_ipc_response(
        webview,
        tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: url.parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: tauri::http::HeaderMap::default(),
            invoke_key: key.to_owned(),
        },
    )
}

/// Refused by Tauri before any Mosaic code ran: a plain string, not a
/// contract error with a `code`.
fn assert_refused(what: &str, result: Result<tauri::ipc::InvokeResponseBody, Value>) {
    match result {
        Ok(_) => panic!("{what}: reached the command"),
        Err(error) => {
            let text = error.as_str().unwrap_or_default();
            assert!(
                text.contains("not allowed") || text.contains("not found"),
                "{what}: {error}"
            );
        }
    }
}

#[test]
fn remote_origins_and_other_webviews_get_no_ipc() {
    // Attempt: a page the main window was sent to, or a second webview (a popup, an injected
    // window), calls Mosaic's commands.
    let _kdf = crate::test_support::kdf_permit();
    let app = mock_app();
    let main = window(&app, "main");
    assert!(
        invoke(&main, "app_status", LOCAL, json!({}), INVOKE_KEY).is_ok(),
        "the control call from the app itself must work"
    );
    for url in [
        "https://evil.example/",
        "http://evil.example/tauri.localhost",
        "http://tauri.localhost.evil.example/",
        "https://localhost.evil.example/",
        "http://127.0.0.1/",
        "http://localhost:1421/",
        "http://ipc.localhost/",
        "file:///C:/Users/Public/evil.html",
        "data:text/html,<script>0</script>",
    ] {
        for cmd in ["app_status", "session_state", "generate_password", "lock"] {
            assert_refused(
                &format!("{cmd} from {url}"),
                invoke(&main, cmd, url, json!({}), INVOKE_KEY),
            );
        }
    }
    let popup = window(&app, "popup");
    for cmd in ["app_status", "session_state", "list_items", "reveal_secret"] {
        assert_refused(
            &format!("{cmd} from the webview `popup`"),
            invoke(&popup, cmd, LOCAL, json!({}), INVOKE_KEY),
        );
    }
}

#[test]
fn core_and_plugin_commands_are_refused_in_the_main_window() {
    // Attempt: XSS in the app turns off screen-capture protection, opens a window or devtools,
    // listens to app events, or reaches a plugin's JavaScript API, through Tauri's own commands.
    let _kdf = crate::test_support::kdf_permit();
    let app = mock_app();
    let main = window(&app, "main");
    let guard_cmd = format!("plugin:{}|navigate", crate::navigation::PLUGIN);
    let attempts = [
        (
            "plugin:window|set_content_protected",
            json!({ "label": "main", "value": false }),
        ),
        ("plugin:window|hide", json!({ "label": "main" })),
        ("plugin:window|close", json!({ "label": "main" })),
        (
            "plugin:window|set_always_on_top",
            json!({ "label": "main", "value": true }),
        ),
        (
            "plugin:window|set_ignore_cursor_events",
            json!({ "label": "main", "value": true }),
        ),
        (
            "plugin:webview|create_webview_window",
            json!({ "options": { "label": "evil", "url": "https://evil.example" } }),
        ),
        (
            "plugin:webview|internal_toggle_devtools",
            json!({ "label": "main" }),
        ),
        (
            "plugin:event|listen",
            json!({ "event": "tauri://drag-drop", "target": { "kind": "Any" }, "handler": 7 }),
        ),
        (
            "plugin:event|emit",
            json!({ "event": "quick-search", "payload": null }),
        ),
        ("plugin:app|version", json!({})),
        ("plugin:app|app_hide", json!({})),
        ("plugin:path|resolve_directory", json!({ "directory": 18 })),
        ("plugin:resources|close", json!({ "rid": 0 })),
        ("plugin:image|from_path", json!({ "path": "C:/x.png" })),
        ("plugin:menu|new", json!({ "kind": "Menu", "options": {} })),
        (
            "plugin:global-shortcut|register",
            json!({ "shortcuts": ["Ctrl+A"], "handler": 1 }),
        ),
        ("plugin:global-shortcut|unregister_all", json!({})),
        ("plugin:single-instance|focus", json!({})),
        (guard_cmd.as_str(), json!({})),
        (
            "plugin:opener|open_url",
            json!({ "url": "ms-settings:privacy" }),
        ),
        (
            "plugin:shell|open",
            json!({ "path": r"\\evil\share\x.exe" }),
        ),
        (
            "plugin:fs|read_file",
            json!({ "path": "C:/Windows/win.ini" }),
        ),
        ("plugin:dialog|open", json!({ "options": {} })),
        ("plugin:clipboard-manager|read_text", json!({})),
    ];
    for (cmd, body) in attempts {
        assert_refused(cmd, invoke(&main, cmd, LOCAL, body, INVOKE_KEY));
    }
}

#[test]
fn a_forged_or_missing_invoke_key_is_refused() {
    // Attempt: a script that did not get Tauri's per-launch invoke key (for example in a frame
    // or a page served elsewhere) posts IPC requests with a guessed key.
    let _kdf = crate::test_support::kdf_permit();
    let app = mock_app();
    let main = window(&app, "main");
    let generate = json!({ "request": {
        "kind": "password", "length": 20, "lowercase": true, "uppercase": true,
        "digits": true, "symbols": false, "excludeAmbiguous": false,
    } });
    for key in ["", "0", "__TAURI_INVOKE_KEY__", &INVOKE_KEY[1..]] {
        // Tauri drops such a request without an answer; the mock's helper then panics
        // waiting for one. Either way nothing may come back and the command must not run.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            invoke(&main, "generate_password", LOCAL, generate.clone(), key)
        }));
        assert!(!matches!(outcome, Ok(Ok(_))), "key {key:?} was accepted");
    }
    let state = tauri::Manager::state::<AppState>(&app);
    assert!(
        state.backend().last_generated.lock().unwrap().is_none(),
        "a request with a forged key generated a password"
    );
    // The control: the real key runs the command.
    assert!(invoke(&main, "generate_password", LOCAL, generate, INVOKE_KEY).is_ok());
    assert!(state.backend().last_generated.lock().unwrap().is_some());
}

#[test]
fn no_webview_navigates_away_from_the_app() {
    // Attempt: a script, a link or a dropped URL sends the vault window to a remote page (to
    // phish the master password or carry data out in the address), to a loopback server on
    // `tauri.localhost:<port>` (which Tauri's IPC counts as local), to a file, or to an OS
    // scheme handler. The CSP stops subresources, not top-level navigations.
    let app = mock_app();
    let main = window(&app, "main");
    let mut guard = crate::navigation::guard::<MockRuntime>();
    let mut allows = |url: &str| guard.on_navigation(main.as_ref(), &Url::parse(url).unwrap());

    let app_pages = if cfg!(windows) {
        [
            "http://tauri.localhost/",
            "http://tauri.localhost/index.html#main",
        ]
    } else {
        ["tauri://localhost/", "tauri://localhost/index.html#main"]
    };
    for url in app_pages {
        assert!(allows(url), "{url} must load");
    }
    // `tauri dev` loads the Vite server; a release build never does.
    assert_eq!(
        allows("http://localhost:1420/"),
        !cfg!(feature = "custom-protocol")
    );

    for url in [
        "https://evil.example/?vault=secret",
        "http://evil.example/",
        "http://tauri.localhost:8080/",
        "http://tauri.localhost:1/",
        "https://tauri.localhost/",
        "http://tauri.localhost.evil.example/",
        "http://evil.example/http://tauri.localhost/",
        "http://user:pass@tauri.localhost/",
        "http://user@localhost:1420/",
        "tauri://localhost.evil.example/",
        "tauri://localhost:8080/",
        "http://localhost:1421/",
        "http://127.0.0.1:1420/",
        "http://localhost.evil.example:1420/",
        "http://ipc.localhost/",
        "http://xn--tauri-localhost-9ib.example/",
        "file:///C:/Windows/win.ini",
        "file://evil-host/share/payload.html",
        "javascript:alert(1)",
        "data:text/html,<h1>Mosaic</h1>",
        "blob:http://tauri.localhost/0b6f6c2e-1",
        "about:blank",
        "ms-settings:privacy",
        "search-ms:query=x&crumb=location:%5C%5Cevil%5Cshare",
        "mailto:a@evil.example",
        "ssh://evil.example",
    ] {
        assert!(!allows(url), "{url} must not load");
    }
    assert!(
        crate::navigation::is_app_url(
            &Url::parse(app_pages[0]).unwrap(),
            Some(&Url::parse("http://localhost:1420").unwrap())
        ),
        "the bundled page loads whatever the dev URL says"
    );
}

#[test]
fn the_app_builder_registers_the_navigation_guard() {
    // Attempt: the guard exists but is not wired into `run()`, so it never runs (the mock
    // runtime cannot show that, so the source is checked).
    let source = include_str!("lib.rs");
    let run = source
        .split("pub fn run()")
        .nth(1)
        .and_then(|rest| rest.split(".setup(").next())
        .unwrap();
    assert!(
        run.contains(".plugin(navigation::guard())"),
        "run() must register navigation::guard() before setup"
    );
}

/// The `tauri` dependency line of the workspace manifest.
fn workspace_tauri_line() -> &'static str {
    include_str!("../../../../Cargo.toml")
        .lines()
        .find(|line| line.starts_with("tauri = "))
        .unwrap()
}

#[test]
fn the_build_has_no_devtools_remote_ipc_asset_protocol_or_webview_url_opener() {
    // Attempt: a config or dependency change that hands the webview devtools, a remote origin,
    // the asset protocol, a looser origin, extra browser flags or a plugin that lets the webview
    // open URLs (M9: Rust opens one page itself, the security-key handoff: see
    // `opener_has_one_call_site`).
    let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let security = config["app"]["security"].as_object().unwrap();
    let mut keys: Vec<&str> = security.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["capabilities", "csp", "devCsp", "freezePrototype"],
        "no assetProtocol, pattern, headers or dangerous* switch"
    );
    let windows = config["app"]["windows"].as_array().unwrap();
    assert_eq!(windows.len(), 1, "one window");
    let main = windows[0].as_object().unwrap();
    assert_eq!(main["label"], json!("main"));
    for key in [
        "url",
        "devtools",
        "useHttpsScheme",
        "additionalBrowserArgs",
        "proxyUrl",
        "browserExtensionsEnabled",
        "javascriptDisabled",
        "parent",
    ] {
        assert!(!main.contains_key(key), "window option {key}");
    }
    assert_ne!(
        main.get("dragDropEnabled"),
        Some(&json!(false)),
        "a drop must not navigate the webview"
    );
    // The production CSP has no navigation directive that could stand in for the guard,
    // and no source for frames or workers.
    let csp = security["csp"].as_str().unwrap();
    for absent in [
        "frame-src",
        "child-src",
        "worker-src",
        "blob:",
        "data:",
        "'wasm",
    ] {
        assert!(!csp.contains(absent), "CSP has {absent}");
    }

    let tauri = workspace_tauri_line();
    assert!(tauri.contains("default-features = false"), "{tauri}");
    for feature in ["devtools", "protocol-asset", "dynamic-acl", "unstable"] {
        assert!(!tauri.contains(&format!("\"{feature}\"")), "{feature}");
    }
    let plugins: Vec<&str> = include_str!("../Cargo.toml")
        .lines()
        .filter_map(|line| line.split(['.', ' ']).next())
        .filter(|name| name.starts_with("tauri-plugin-"))
        .collect();
    assert_eq!(
        plugins,
        [
            "tauri-plugin-global-shortcut",
            "tauri-plugin-single-instance"
        ],
        "no opener or shell plugin: the webview never opens a URL"
    );
    // The one crate that opens the system browser is a plain dependency of Rust code.
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("open = \"="), "`open` is pinned exactly");
    let capability: Value =
        serde_json::from_str(include_str!("../capabilities/main.json")).unwrap();
    for permission in capability["permissions"].as_array().unwrap() {
        let name = permission.as_str().unwrap();
        for denied in ["opener", "shell", "open-url"] {
            assert!(!name.contains(denied), "capability grants {name}");
        }
    }
}

/// The `.rs` files under `dir`, with their text.
fn sources(dir: &std::path::Path, out: &mut Vec<(std::path::PathBuf, String)>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push((path.clone(), std::fs::read_to_string(&path).unwrap()));
        }
    }
}

#[test]
fn opener_has_one_call_site() {
    // Attempt: a second place that opens the system browser (a command that takes a URL from the
    // webview, a "help" link, a shell-out). The security-key handoff page is the only one, and
    // its URL is built in Rust from the stored origin and a server-made handoff ID.
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&src, &mut files);
    let count = |needle: &str| -> Vec<String> {
        files
            .iter()
            .filter(|(path, _)| !path.ends_with("redteam_surface_tests.rs"))
            .flat_map(|(path, text)| {
                text.lines()
                    .filter(|l| !l.trim_start().starts_with("//") && l.contains(needle))
                    .map(|l| format!("{}: {}", path.display(), l.trim()))
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    // The crate's entry point (`open` + `::that`, split so this file does not match itself).
    let crate_calls = count(&["open", "::that"].concat());
    assert_eq!(crate_calls.len(), 1, "{crate_calls:?}");
    assert!(
        crate_calls[0].contains("webauthn_flow.rs"),
        "{crate_calls:?}"
    );
    // Everything goes through the backend's opener, from `open_handoff` only.
    let opener_calls = count(&[".opener", ".open("].concat());
    assert_eq!(opener_calls.len(), 1, "{opener_calls:?}");
    assert!(
        opener_calls[0].contains("webauthn_flow.rs"),
        "{opener_calls:?}"
    );
    // No process spawns a browser or a shell command either.
    for needle in ["xdg-open", "ShellExecute", "cmd /c start", "explorer.exe"] {
        assert!(count(needle).is_empty(), "{needle}: {:?}", count(needle));
    }
}

#[test]
fn tagged_request_values_refuse_unknown_fields() {
    // Attempt: a request smuggles extra keys inside a tagged value (a copy source, a secret
    // reference, a generator request, a draft operation), which every other request refuses.
    use crate::contract::clipboard::CopySource;
    use crate::contract::generator::GenerateRequest;
    use crate::contract::items::{CustomFieldChange, OtpChange, SecretRef, UrlChange};

    fn check<T: serde::de::DeserializeOwned>(what: &str, valid: &Value) {
        assert!(
            serde_json::from_value::<T>(valid.clone()).is_ok(),
            "{what}: the valid sample must parse"
        );
        for extra in ["vaultId", "__proto__", "allowHttp", "isAdmin"] {
            let mut value = valid.clone();
            if value.get(extra).is_some() {
                continue;
            }
            value[extra] = json!(true);
            assert!(
                serde_json::from_value::<T>(value).is_err(),
                "{what}: unknown field {extra} was accepted"
            );
        }
    }

    let vault = "019a2b3c-0000-7000-8000-000000000001";
    let item = "019a2b3c-0000-7000-8000-000000000002";
    // Unit variants (`generated`, `password`, `remove`, ...) carry nothing an extra key could
    // change, and serde cannot refuse extra keys on internally tagged unit variants; only the
    // variants with fields are checked.
    check::<CopySource>(
        "copy pending",
        &json!({ "kind": "pendingOtp", "pendingId": "ab" }),
    );
    check::<CopySource>(
        "copy item",
        &json!({ "kind": "item", "vaultId": vault, "itemId": item, "field": { "kind": "password" } }),
    );
    // The nested field selector too.
    let nested = json!({
        "kind": "item", "vaultId": vault, "itemId": item,
        "field": { "kind": "secret", "field": "cardNumber", "elementId": "x" },
    });
    assert!(serde_json::from_value::<CopySource>(nested).is_err());
    check::<SecretRef>(
        "reveal field",
        &json!({ "kind": "field", "field": "password" }),
    );
    check::<SecretRef>("reveal history", &json!({ "kind": "history", "index": 0 }));
    check::<GenerateRequest>(
        "generate",
        &json!({ "kind": "passphrase", "words": 5, "separator": "-", "capitalize": false, "appendDigit": false }),
    );
    check::<UrlChange>(
        "url add",
        &json!({ "op": "add", "url": "https://a.example", "matchMode": "exactOrigin", "allowHttp": false }),
    );
    check::<UrlChange>("url remove", &json!({ "op": "remove", "elementId": "e" }));
    check::<CustomFieldChange>(
        "custom add",
        &json!({ "op": "add", "label": "PIN", "kind": "hidden", "value": "1234" }),
    );
    check::<OtpChange>(
        "otp pending",
        &json!({ "op": "setPending", "pendingId": "ab", "issuer": null, "accountName": null }),
    );
}

#[test]
fn attachment_requests_refuse_path_keys() {
    // Attempt: the webview names a file by path (to read or to overwrite one) or smuggles any
    // other key into an attachment request. Paths come only from the native dialogs Rust opens.
    use crate::contract::attachments::AttachmentRequest;
    use crate::contract::items::ItemRequest;

    let ids = json!({
        "vaultId": "019a2b3c-0000-7000-8000-000000000001",
        "itemId": "019a2b3c-0000-7000-8000-000000000002",
    });
    let with_attachment = json!({
        "vaultId": "019a2b3c-0000-7000-8000-000000000001",
        "itemId": "019a2b3c-0000-7000-8000-000000000002",
        "attachmentId": "019a2b3c-0000-7000-8000-000000000003",
    });
    assert!(serde_json::from_value::<ItemRequest>(ids.clone()).is_ok());
    assert!(serde_json::from_value::<AttachmentRequest>(with_attachment.clone()).is_ok());
    for key in ["path", "filePath", "fileName", "uri", "token", "isAdmin"] {
        for valid in [&ids, &with_attachment] {
            let mut value = valid.clone();
            value[key] = json!("C:/Users/victim/.ssh/id_ed25519");
            assert!(
                serde_json::from_value::<ItemRequest>(value.clone()).is_err()
                    && serde_json::from_value::<AttachmentRequest>(value).is_err(),
                "{key} was accepted"
            );
        }
    }
}

#[test]
fn credential_rotation_requests_refuse_extra_keys_urls_and_values_in_the_wrong_place() {
    // Attempt: the webview smuggles a URL to open, a file path, a hash or a pending password into a
    // rotation or breach request. The change page is only ever resolved in Rust and handed back
    // as a link to copy; the new password travels only in `rotation_begin`.
    use crate::contract::credential_rotation::{
        HistoryRestoreRequest, RotationAttemptRequest, RotationBeginRequest, RotationCloseRequest,
        RotationPolicyRequest, RotationSnoozeRequest,
    };

    fn check<T: serde::de::DeserializeOwned>(what: &str, valid: &Value) {
        assert!(
            serde_json::from_value::<T>(valid.clone()).is_ok(),
            "{what}: the valid sample must parse"
        );
        for extra in [
            "url",
            "changeUrl",
            "path",
            "hash",
            "prefix",
            "newPassword",
            "pending",
            "isAdmin",
        ] {
            let mut value = valid.clone();
            if value.get(extra).is_some() {
                continue;
            }
            value[extra] = json!("https://evil.example/");
            assert!(
                serde_json::from_value::<T>(value).is_err(),
                "{what}: unknown field {extra} was accepted"
            );
        }
    }

    let ids = json!({
        "vaultId": "019a2b3c-0000-7000-8000-000000000001",
        "itemId": "019a2b3c-0000-7000-8000-000000000002",
    });
    let with = |extra: Value| {
        let mut value = ids.clone();
        for (key, v) in extra.as_object().unwrap() {
            value[key] = v.clone();
        }
        value
    };
    check::<RotationBeginRequest>(
        "rotation_begin",
        &with(json!({ "newPassword": "q7#Vt2-new", "rulesSource": null })),
    );
    check::<RotationAttemptRequest>("rotation_reveal", &with(json!({ "attemptId": "a.b.0" })));
    check::<RotationCloseRequest>(
        "rotation_close",
        &with(json!({ "attemptId": "a.b.0", "outcome": "failed" })),
    );
    check::<crate::contract::browser::BrowserRotationRespondRequest>(
        "browser_rotation_respond",
        &json!({ "requestId": "7c6b5a4d3e2f1a0b9c8d7e6f5a4b3c2d", "decision": { "kind": "allow" } }),
    );
    // The answer to a browser dialog carries no password and no address: a decision with an extra
    // key, or an unknown kind, is refused.
    for hostile in [
        json!({ "kind": "allow", "newPassword": "q7#Vt2-new" }),
        json!({ "kind": "addOrigin", "origin": "https://evil.example" }),
        json!({ "kind": "confirm", "changeUrl": "https://evil.example/" }),
        json!({ "kind": "openUrl" }),
    ] {
        let request =
            json!({ "requestId": "7c6b5a4d3e2f1a0b9c8d7e6f5a4b3c2d", "decision": hostile });
        assert!(
            serde_json::from_value::<crate::contract::browser::BrowserRotationRespondRequest>(
                request
            )
            .is_err(),
            "{hostile} was accepted"
        );
    }
    check::<HistoryRestoreRequest>("restore", &with(json!({ "historyId": "a.b.0" })));
    check::<RotationSnoozeRequest>("snooze", &with(json!({ "days": 7 })));
    check::<RotationPolicyRequest>("policy", &with(json!({ "everyDays": null })));
    // The breach commands take no argument at all, so they cannot be handed a prefix, a hash or a
    // host: the host of the request is a constant of `mosaic-sync` (api.pwnedpasswords.com).
    assert!(
        crate::commands::names::COMMANDS.contains(&"breach_scan_start")
            && crate::commands::names::COMMANDS.contains(&"breach_scan_cancel")
    );
}
