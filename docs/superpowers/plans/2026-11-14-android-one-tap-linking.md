# Android One-Tap Linking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When an Android app with no linked login asks Mosaic to fill, offer up to three "Link <title> and fill" entries for strong candidates; one explicit tap plus one biometric confirmation writes the cert-bound `androidapp://` URL and fills.

**Architecture:** The Rust core (`crates/mobile`) computes the strong candidates from the **fill session** that `fill_candidates` already holds (package and the one current signer, read by Kotlin from `PackageManager`, never from the platform request), and links only through that session. Kotlin adds the entries to the Autofill response as per-dataset-authenticated, value-less datasets, and `FillAuthActivity` gets a `link` mode: system biometric prompt as the confirmation, then link, then fill; without a strong biometric, a confirm screen that links only (no fill), as the existing link screen does.

**Tech Stack:** Rust (`mosaic-origin`, `mosaic-sync`, `mosaic-mobile`, `mosaic-mobile-ffi`, UniFFI 0.32.2), Kotlin + Jetpack Compose (`apps/mobile/android`), framework `BiometricPrompt`, TypeScript catalogs (`packages/i18n`).

**Spec:** `docs/superpowers/specs/2026-11-14-autofill-everywhere-design.md` §5.5.2, §6.2 "Android linking", §6.3 "Android", builder check C1. ADR-0092 (on `main` after M10 merges, `docs/DECISIONS.md`).

**Base:** `main` after PR #69 (`feat/m10-mobile`) is merged. Branch `feat/android-one-tap-link`. Every path below exists only after that merge.

## Global Constraints

- ADR-0092's rules stay: no automatic linking, the choice is made on one specific login, no "fill anyway", browsers are never apps (M10-G3-5), exactly one current signer (G1-9).
- Linking happens only after the user taps one specific "Link <title> and fill" dataset **and** confirms (biometric prompt, or the confirm button of the fallback screen).
- The link stores the package and exactly one current signer certificate taken from the Rust fill session (`Resolved::App`). The new core calls take a `session_id` and an `ItemKey`, never an `AndroidCaller` or a certificate.
- A login linked to one app never matches another package: package comparison is byte-exact, no case folding, no prefix or similarity rule.
- Link and fill in one step only on the biometric path (spec §5.5.2: "One biometric confirm links … and fills"). The fallback screen links only; the user taps the field again to fill.
- Datasets in a response carry `null` values and an authentication intent (ADR-0092). No value before the confirmation.
- At most 3 link entries per response (spec). The asset-links path never fetches a host whose registrable domain differs from the package's web domain (Task 1), checks at most 2 logins and makes at most 1 uncached fetch per request, with a 2 s timeout.
- Rust: `#![forbid(unsafe_code)]` stays; library code returns `thiserror` results, no `unwrap`/`expect`/`panic`/`println!`; errors are unit variants of `MobileError` (no message, no value).
- Secrets: the password reaches Kotlin only through `fill_values` (`SecretValue`, wiped after use), as today.
- UI strings only through `packages/i18n` (EN and DE with informal "du"); the five generated mobile files are regenerated, never hand-edited. Compose styling only through the existing theme (`MosaicTheme`, `MosaicSpace`, components in `ui/components`), which comes from `packages/design-tokens`.
- No new dependency (Rust, Gradle or npm).
- Verification commands (Git Bash, from the worktree root), always with the shared target dir:
  `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p <crate> <filter>`;
  format with `cargo fmt -p <crate> -- --config newline_style=Auto`;
  lint with `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo clippy -p <crate> --all-targets -- -D warnings`.
  Kotlin builds and runs only in CI (`.github/workflows/mobile.yml`): commit, push, and let the orchestrator's wait loop read the result. Never build Gradle locally.
- Agent guard (`~/.claude/hooks/agent-guard.mjs`): write only inside the worktree and its `.agent-scope`; no searches outside the repo, `node_modules` or the cargo registry; at most 3 full-workspace runs per hour (prefix a deliberate final gate with `FULL_RUN_OK=1`); never `git worktree remove` (use `~/.claude/hooks/worktree.sh`).
- Conventional Commits. No new dependency means no justification line is needed.

## Review Focus

1. **Many same-domain logins** (five `google.com` logins and the Gmail app `com.google.android.gm`): the user expects a short, stable list and a fast response, not one network fetch per login. Test `link_offer_bounds_checks_and_fetches` (Task 2).
2. **Network down or slow** while the app asks: the user expects the normal "No login for X" entry quickly, and a later request to try again. Test `unreachable_asset_links_give_no_candidate_and_are_not_cached` (Task 2).
3. **The app is re-signed or replaced between the list and the tap** (another signer at tap time): the user expects the link to name the signer of the app that is in front of them at the confirmation, or nothing. Test `one_tap_link_writes_the_session_signer` (Task 2) and the tap-time re-read in Task 5.
4. **A login that was trashed or deleted after the list was shown**, or a hardware-key vault: never linked. Test `link_offer_skips_trashed_and_deleted_logins` (Task 2). (No fixture for a shared viewer vault exists in `crates/mobile/tests`; the writable-role filter of Task 2 step 3 is checked by review.)
5. **An app that already has a linked login** but also carries an old imported URL: the user expects the normal fill entry, not a second link offer. Test `link_offer_is_empty_when_a_login_already_matches` (Task 2).

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `docs/DECISIONS.md` | modify (ADR-0092) | Amendment "one-tap linking" |
| `docs/THREAT_MODEL.md` | modify (§4.8, §5) | Row update and one residual |
| `crates/origin/src/app.rs`, `crates/origin/src/mobile_tests.rs` | modify | `package_web_domain` |
| `crates/sync/src/http/assetlinks.rs` | modify | `fetch_quick` (2 s) |
| `crates/mobile/src/autofill.rs` | modify | `LinkReason`, `LinkCandidate`, `LinkOffer`, `fill_link_offer`, `link_app_in_fill_session`, shared `write_app_link`, cache purpose |
| `crates/mobile/src/lib.rs` | modify | re-exports |
| `crates/mobile/tests/it/link_offer.rs`, `crates/mobile/tests/it/main.rs` | create / modify | integration tests |
| `crates/mobile-ffi/src/{types.rs,convert.rs,lib.rs}` | modify | mirrors and two methods |
| `packages/i18n/src/catalog/{en,de}/mobile.ts` + 5 generated files | modify | strings |
| `apps/mobile/android/.../autofill/{FillIntents,FillPlanner,FillResponseFactory,MosaicAutofillService,FillAuthActivity,LinkFlow}.kt` | modify / create | response entries and the link mode |
| `apps/mobile/android/.../security/BiometricGate.kt` | modify | `confirm` without a crypto object |
| `apps/mobile/android/.../ui/link/{LinkAppScreen,AppIdentityPanel}.kt` | modify / create | shared identity panel |
| `apps/mobile/android/app/src/test/.../autofill/LinkFlowTest.kt` | create | JVM unit test |
| `apps/mobile/android/app/src/androidTest/.../autofill/{AutofillTestKit,OneTapLinkInstrumentedTest,AutofillInstrumentedTest}.kt` | create / modify | instrumented tests |
| `docs/MOBILE.md` | modify (§3 table row "Autofill", §7) | behaviour and manual checks |

Kotlin paths below abbreviate `apps/mobile/android/app/src/main/kotlin/io/github/stephanbaum/mosaic/` as `K/`.

---

### Task 1: ADR amendment, package web domain, quick asset-links fetch

**Files:**
- Modify: `docs/DECISIONS.md` (end of ADR-0092's Decision list, after the last amendment)
- Modify: `docs/THREAT_MODEL.md` (§4.8 row "A look-alike app …"; §5 new residual row)
- Modify: `crates/origin/src/app.rs`
- Test: `crates/origin/src/mobile_tests.rs`
- Modify: `crates/sync/src/http/assetlinks.rs` (+ its existing `#[cfg(test)]` module)

**Interfaces:**
- Consumes: `mosaic_origin::registrable_domain(host: &str) -> Option<String>`.
- Produces:
  - `pub fn mosaic_origin::app::package_web_domain(package: &str) -> Option<String>`
  - `pub fn mosaic_sync::http::assetlinks::fetch_quick(host: &str) -> Result<Vec<u8>, TransportError>` (same rules as `fetch`, timeout `QUICK_TIMEOUT = Duration::from_secs(2)`)

- [ ] **Step 1: Write the ADR-0092 amendment and the threat rows**

Append to ADR-0092's Decision list:

```markdown
  - **Amendment (one-tap linking, autofill-everywhere spec §5.5.2).** When a fill request names an app that no login matches, the core offers at most 3 **strong candidates** from the fill session (`fill_link_offer`): a login with `androidapp://<package>` without `cert` whose package equals the caller's byte for byte ("Imported for this app"), or a login with an `https` URL whose host's `assetlinks.json` lists the package and the session's single current signer with `get_login_creds` ("Verified by <host>"). Only hosts whose registrable domain equals the package's web domain (`package_web_domain`: the first three labels reversed, reduced to the registrable domain) are fetched, at most 2 logins are checked and at most 1 uncached fetch (2 s timeout) runs per request; answers are cached 24 hours, transport failures are not. Each candidate is a value-less, authenticated dataset "Link <title> and fill". A tap opens `FillAuthActivity` in `link` mode, which reads the caller again, opens a new fill session and requires the tapped login to be in the new offer. The confirmation is the system biometric prompt (strong biometric), whose title and subtitle name the login, the app label, the package, the first 8 bytes of the signer fingerprint and the reason; on success `link_app_in_fill_session` writes the cert-bound URL with the **session's** signer and `fill_values` fills. Without a strong biometric, a confirm screen with the same identity panel links only and does not fill. Weak cases ("Not verified", "Could not check", no candidate) keep the "No login for <app>" flow. Browsers never get an offer. Alternative rejected: an offer computed from every https login of the vault (one network request per saved host, and the network learns the vault's site list).
```

THREAT_MODEL §4.8, row "A look-alike app (same package name, sideloaded, another key)": append to the mitigation, before `**Residual:**`: "One-tap linking offers only an imported URL of the exact package or a login whose site lists the app and signer in `assetlinks.json`, and the biometric prompt names the app, the package and the signer (ADR-0092 amendment, one-tap)."

THREAT_MODEL §5: add a row with the next free `R` number on `main` (`grep -o "^| R[0-9]*" docs/THREAT_MODEL.md | sort -t R -k2 -n | tail -1`, plus one):
`| R<n> | A fill request in an app with no linked login makes Mosaic fetch \`assetlinks.json\` from a host of a saved login whose registrable domain equals the package's web domain. The network learns that the phone has a login at that site when the app is opened (at most once per 24 hours per host, package and signer). The one-tap link widens R136 slightly: a site that lists a malicious app gets a one-tap offer for it | accepted (ADR-0092 amendment, one-tap) |`

- [ ] **Step 2: Write the failing tests for `package_web_domain`**

In `crates/origin/src/mobile_tests.rs` (import `crate::app::package_web_domain`):

```rust
#[test]
fn package_web_domain_reverses_up_to_three_labels() {
    assert_eq!(package_web_domain("com.github.android").as_deref(), Some("github.com"));
    assert_eq!(package_web_domain("com.google.android.gm").as_deref(), Some("google.com"));
    assert_eq!(package_web_domain("uk.co.bbc.iplayer").as_deref(), Some("bbc.co.uk"));
    assert_eq!(package_web_domain("com.example").as_deref(), Some("example.com"));
    assert_eq!(package_web_domain("de.dkb.portalapp").as_deref(), Some("dkb.de"));
    assert_eq!(package_web_domain("Com.GitHub.Android").as_deref(), Some("github.com"));
}

#[test]
fn package_web_domain_refuses_what_is_no_domain() {
    for p in ["com", "", "com.my_app.x", "co.uk", "com..x", "bad package!"] {
        assert_eq!(package_web_domain(p), None, "{p}");
    }
    // A look-alike package points at its own domain, never at the real one.
    assert_eq!(package_web_domain("com.githuh.android").as_deref(), Some("githuh.com"));
}
```

- [ ] **Step 3: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-origin package_web_domain`
Expected: compile error, `package_web_domain` not found.

- [ ] **Step 4: Implement `package_web_domain(package: &str) -> Option<String>` in `crates/origin/src/app.rs`**

`None` unless `is_valid_package_name(package)` and it has at least 2 labels, none empty and none containing `_`. Take the first `min(3, n)` labels, reverse, join with `.`, ASCII-lowercase, return `registrable_domain(&joined)`. Doc comment: the result only chooses which hosts may be asked; it never decides a match.

- [ ] **Step 5: Implement `fetch_quick` in `crates/sync/src/http/assetlinks.rs`**

`pub fn fetch_quick(host: &str) -> Result<Vec<u8>, TransportError>` = `fetch_with(&format!("https://{host}"), false, QUICK_TIMEOUT)` with `const QUICK_TIMEOUT: Duration = Duration::from_secs(2);`. Extend the module doc ("a 10 s timeout, or 2 s for `fetch_quick`, used on the Autofill request path"). No new test here: the real fetch is network, and Task 2's tests replace both fetches through the test hook.

- [ ] **Step 6: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-origin package_web_domain` then `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-sync assetlinks`
Expected: PASS. Then `cargo fmt -p mosaic-origin -p mosaic-sync -- --config newline_style=Auto` and clippy for both crates: no warning.

- [ ] **Step 7: Commit**

```bash
git add docs/DECISIONS.md docs/THREAT_MODEL.md crates/origin/src/app.rs crates/origin/src/mobile_tests.rs crates/sync/src/http/assetlinks.rs
git commit -m "feat(origin,sync): package web domain and quick asset-links fetch for one-tap linking (ADR-0092 amendment)"
```

---

### Task 2: Core link offer and session-bound link

**Files:**
- Modify: `crates/mobile/src/autofill.rs`, `crates/mobile/src/lib.rs` (re-exports next to `AssetLinksStatus`)
- Create: `crates/mobile/tests/it/link_offer.rs`; Modify: `crates/mobile/tests/it/main.rs` (`mod link_offer;`)
- Modify: `crates/mobile-ffi/src/types.rs`, `crates/mobile-ffi/src/convert.rs`, `crates/mobile-ffi/src/lib.rs` (autofill section, after `fill_otp`)

**Interfaces:**
- Consumes: Task 1 `package_web_domain`, `fetch_quick`; existing `Core::fill_candidates`, `Core::fill_values`, `fill_session_target`, `usable_vaults`, `match_item`, `single_signer`, `AndroidAppUrl`, `parse_asset_links`, `app_is_delegated(.., Relation::GetLoginCreds)`, `UnlockedSession::vault_own_role(VaultId) -> Result<mosaic_core::model::Role>`.
- Produces (in `mosaic_mobile`, re-exported from the crate root, mirrored 1:1 in `mosaic-mobile-ffi`):

```rust
pub const MAX_LINK_CANDIDATES: usize = 3;

/// Why a login is a strong one-tap candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkReason {
    /// The login has `androidapp://<package>` without `cert` (an import).
    ImportedForApp,
    /// `host`'s assetlinks.json lists the package and signer (get_login_creds).
    VerifiedBy { host: String },
}

/// A login the user may link to the app with one tap. No secret value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkCandidate {
    pub key: ItemKey,
    pub title: String,
    pub username: Option<String>,
    pub reason: LinkReason,
}

/// What the fill session's app may be linked to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkOffer {
    /// The session's package (what is linked).
    pub package_name: String, // not `package`: a Kotlin keyword
    /// First 8 bytes of the session's signer SHA-256, `AA:BB:CC:DD:EE:FF:00:11`.
    pub signer_prefix: String,
    /// At most MAX_LINK_CANDIDATES, imports first, then by title.
    pub candidates: Vec<LinkCandidate>,
}

impl Core {
    pub fn fill_link_offer(&self, session_id: String) -> Result<LinkOffer>;
    pub fn link_app_in_fill_session(&self, session_id: String, candidate: ItemKey) -> Result<()>;
}
```

FFI (`MosaicCore`): `fill_link_offer(session_id: String) -> Result<LinkOffer, MobileError>` and `link_app_in_fill_session(session_id: String, candidate: ItemKey) -> Result<(), MobileError>`; records `LinkOffer`, `LinkCandidate`, enum `LinkReason { ImportedForApp, VerifiedBy { host } }`. Kotlin sees `fillLinkOffer`, `linkAppInFillSession`, `LinkOffer(packageName, signerPrefix, candidates)`, `LinkCandidate(key, title, username, reason)`, `LinkReason.ImportedForApp`, `LinkReason.VerifiedBy(host)`.

- [ ] **Step 1: Write the failing integration tests**

Create `crates/mobile/tests/it/link_offer.rs`. Copy the helpers `url`, `login`, `app_target`, `assetlinks_json`, `CERT`, `OTHER_CERT` from `tests/it/autofill.rs` (or move them to `tests/it/common/mod.rs` and import them in both). Helper:

```rust
fn offer(phone: &Phone, package: &str, cert: [u8; 32]) -> Result<LinkOffer, MobileError> {
    let session = phone.core.fill_candidates(app_target(package, cert))?.session_id;
    phone.core.fill_link_offer(session)
}
fn imported(phone: &Phone, title: &str, package: &str) -> ItemKey {
    login(phone, title, vec![url(&format!("androidapp://{package}"), false, false)])
}
```

Tests (all on `Phone::android_with_account()`; the default fetch is replaced in every test with `phone.core.test_hooks().set_asset_links_fetch(..)` that counts calls in an `Arc<Mutex<Vec<String>>>`):

```rust
#[test]
fn one_tap_link_offered_only_for_verified_or_exact_package_candidates() {
    // Imported for the package; verified by github.com; an unrelated https login; an http login of the domain.
    // fetch: "github.com" -> assetlinks_json("com.github.android", &CERT); anything else -> Err(NotFound)
    // offer("com.github.android", CERT).candidates, as (title, reason):
    //   [("Imported", ImportedForApp), ("GitHub", VerifiedBy { host: "github.com" })]
    // the unrelated and the http login are absent; fetched hosts == ["github.com"]
    // signer_prefix == "11:11:11:11:11:11:11:11"; package_name == "com.github.android"
}

#[test]
fn browser_package_never_gets_a_link_offer() {
    // imported(&phone, "Chrome", "com.android.chrome"):
    // fill_candidates(app_target("com.android.chrome", CERT)) fails or the offer fails:
    // assert!(offer(&phone, "com.android.chrome", CERT).map(|o| o.candidates.is_empty()).unwrap_or(true));
    // and link_app_in_fill_session on any session for it is Err(InvalidInput) or Err(NoMatch); urls unchanged
}

#[test]
fn link_offer_never_matches_a_lookalike_package() {
    // imported(&phone, "Example", "com.example.app")
    // for p in ["com.example.app2", "com.example.ap", "com.Example.app", "com.examp1e.app"]:
    //     offer(p, CERT).unwrap().candidates.is_empty()
    // link it for "com.example.app" (offer + link_app_in_fill_session), then
    // fill_candidates(app_target("com.example.app2", CERT)).candidates.is_empty()
}

#[test]
fn one_tap_link_writes_the_session_signer() {
    // imported "Example" for com.example.app; session S from app_target("com.example.app", CERT)
    // link_app_in_fill_session(S, key) == Ok(())
    // item urls contain exactly one androidapp URL == format!("androidapp://com.example.app?cert={}", b64url(&CERT))
    // fill_candidates(app_target("com.example.app", OTHER_CERT)).candidates.is_empty()
    // a new session with CERT: fill_values(session, key).password is Some (the link made it a match)
}

#[test]
fn link_app_in_fill_session_refuses_what_is_not_a_strong_candidate() {
    // plain login "https://unrelated.example": Err(NoMatch), urls unchanged
    // web session (web("https://example.com")): Err(InvalidInput)
    // session with two certs (FillTargetKind::AndroidApp { certs: vec![CERT, OTHER_CERT] }): offer and link Err(InvalidInput)
    // unknown session id "00": Err(NotFound); after lock(): Err(Locked)
}

#[test]
fn link_offer_is_empty_when_a_login_already_matches() {
    // one login bound with ?cert=CERT for com.example.app and one imported for com.example.app:
    // offer("com.example.app", CERT).candidates.is_empty(); no fetch happened
}

#[test]
fn link_offer_bounds_checks_and_fetches() {
    // four imported logins for com.google.android.gm  -> candidates.len() == 3 (MAX_LINK_CANDIDATES)
    // fresh phone: five https logins a.google.com .. e.google.com, fetch answers "listed" for every host:
    //   first offer: fetched hosts.len() == 1, candidates.len() == 1
    //   second offer: fetched hosts.len() == 2 in total, candidates.len() == 2 (cache + one new fetch)
    //   third offer: no new fetch (only MAX 2 logins are checked), candidates.len() == 2
}

#[test]
fn unreachable_asset_links_give_no_candidate_and_are_not_cached() {
    // https login github.com, fetch -> Err(TransportError::Unreachable)
    // offer("com.github.android", CERT).candidates.is_empty(); a second offer fetches again (2 calls)
    // fetch -> Err(TransportError::NotFound): candidates empty; a second offer does not fetch (cached)
}

#[test]
fn link_offer_skips_trashed_and_deleted_logins() {
    // imported "A" trashed with phone.core.trash_item; offer empty;
    // link_app_in_fill_session(session, key_of_A) == Err(NotFound)
}
```

Write each body with real assertions as commented. `link_app_in_fill_session` never returns values: the tests reach the password only through `fill_values`.

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-mobile --test it link_offer`
Expected: compile errors (`LinkOffer`, `fill_link_offer` unknown).

- [ ] **Step 3: Implement in `crates/mobile/src/autofill.rs`**

- Types and constants of the Interfaces block, plus private `const MAX_ASSET_LINK_CHECKS: usize = 2;` and `const MAX_UNCACHED_FETCHES: usize = 1;`.
- Cache purpose: `#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] enum AssetLinksPurpose { Passkeys, Login }`; `type AssetLinksKey = (String, String, [u8; 32], AssetLinksPurpose);`. `check_asset_links` uses `Passkeys`. `AssetLinks` gets a second field `quick: Mutex<AssetLinksFetch>` (default `Arc::new(mosaic_sync::http::assetlinks::fetch_quick)`); `replace_fetch` replaces both and clears the cache.
- Refactor: move the URL edit of `link_android_app` (keep one equal binding, remove the package's other URLs, add the new one) into `fn write_app_link(s: &mut UnlockedSession, vault: VaultId, id: ItemId, package: &str, cert: [u8; 32]) -> Result<()>`; `link_android_app` calls it unchanged in behaviour (its tests in `tests/it/autofill.rs` must still pass).
- `fill_link_offer(session_id)`: `require_app()`, `require_platform(Platform::Android)`; `fill_session_target` must be `Resolved::App { package, certs }` (else `InvalidInput`); `is_browser_package(&package)` → `InvalidInput`; `certs` must be exactly one (else `InvalidInput`). Inside `with_session`: candidate logins are `ItemType::Login`, in `usable_vaults`, `vault_own_role` is `Owner | Manage | Write`, not deleted, not trashed. If any candidate login `match_item`es the target → return an empty offer without fetching. Imports: a login with an `AndroidAppUrl` whose `package()` equals the session package byte for byte and whose `cert_sha256()` is `None`. Probes: other logins with an `https` saved origin whose `registrable_domain(host_str)` equals `package_web_domain(&package)`; the probe host is the smallest such `host[:port]` string (the same form `asset_links_status` builds); sort probes by (lowercase title, item id) and keep the first `MAX_ASSET_LINK_CHECKS`. Leave the session (`with_session` returns the lists), then resolve probes: cached verdict for `(host, package, cert, Login)` if not expired; else, while fewer than `MAX_UNCACHED_FETCHES` fetches ran, `quick` fetch → `parse_asset_links` → `app_is_delegated(.., Relation::GetLoginCreds)`; remember `true`/`false` for 24 h (`NotFound` = `false`); any other `TransportError` is not remembered and gives no candidate. Order: imports by (lowercase title, item id), then verified ones the same way; truncate to `MAX_LINK_CANDIDATES`. `signer_prefix`: first 8 bytes, `{:02X}` joined by `:`.
- `link_app_in_fill_session(session_id, candidate)`: same session, platform, browser and single-signer checks; `parse_item_key`; the item must pass the same filters (else `NotFound` for trashed/deleted, `NoMatch` for the rest) and be a strong candidate **for this session** (import of the exact package, or a cached-or-quick-fetched `Login` verdict `true` for one of its probe hosts), else `NoMatch`; then `write_app_link(s, vault, id, &package, cert)`. Returns `()`; never reads a secret.
- Doc comments on both calls state the security rule: the certificate comes from the session, never from the caller of this function.

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-mobile --test it link_offer` then `... cargo test -p mosaic-mobile --test it autofill`
Expected: PASS (new tests and the unchanged `link_android_app_*` tests).

- [ ] **Step 5: FFI mirrors**

In `crates/mobile-ffi`: records/enums in `types.rs` like `AssetLinksStatus`; `Conv` impls in `convert.rs` both ways where a type crosses inward (`ItemKey` already does); the two `MosaicCore` methods in `lib.rs`, one line each via `lift(self.inner.…)`. `lib.rs` must stay under the 700-line cap of `ffi_crate_has_no_logic`.

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-mobile-ffi`
Expected: PASS. Then `cargo fmt -p mosaic-mobile -p mosaic-mobile-ffi -- --config newline_style=Auto` and clippy for both: no warning. `bash scripts/mobile/check-ffi-crate.sh` passes.

- [ ] **Step 6: Commit**

```bash
git add crates/mobile crates/mobile-ffi
git commit -m "feat(mobile): one-tap link offer and session-bound link for Android apps (ADR-0092 amendment)"
```

---

### Task 3: Strings (EN and DE)

**Files:**
- Modify: `packages/i18n/src/catalog/en/mobile.ts`, `packages/i18n/src/catalog/de/mobile.ts` (next to the `mobile.link.*` and `mobile.autofill.*` keys)
- Regenerate: `apps/mobile/android/app/src/main/res/values/strings.xml`, `.../res/values-de/strings.xml`, `K/ui/l10n/L10n.kt`, `apps/mobile/ios/Resources/Localizable.xcstrings`, `apps/mobile/ios/MosaicKit/L10n/L10n.swift`

**Interfaces:**
- Produces (Kotlin accessors generated from the keys; arguments in this order):
  - `L10n.autofillLinkEntry(r, login)`, `L10n.linkImported(r)`, `L10n.linkConfirmTitle(r, login, app)`, `L10n.linkConfirmSubtitle(r, appId, fingerprint)`, `L10n.linkConfirmFallbackHint(r)`.
  - Reused, unchanged: `L10n.linkVerified(r, host)`, `L10n.linkConfirm(r, login)`, `L10n.linkDone(r, login)`, `L10n.linkTitle(r)`, `L10n.commonCancel(r)`, `L10n.commonDone(r)`, `L10n.autofillNoLogin(r, app)`.

- [ ] **Step 1: Add the keys**

| Key | EN | DE |
|---|---|---|
| `mobile.autofill.linkEntry` | `Link {login} and fill` | `{login} verknüpfen und ausfüllen` |
| `mobile.link.imported` | `Imported for this app` | `Für diese App importiert` |
| `mobile.link.confirmTitle` | `Link {login} to {app}?` | `{login} mit {app} verknüpfen?` |
| `mobile.link.confirmSubtitle` | `App ID {appId} · Signature {fingerprint}` | `App-ID {appId} · Signatur {fingerprint}` |
| `mobile.link.confirmFallbackHint` | `After linking, tap the field again to fill it.` | `Tippe nach dem Verknüpfen noch einmal auf das Feld, um es auszufüllen.` |

- [ ] **Step 2: Regenerate and check**

Run: `pnpm --filter @mosaic/i18n export:mobile` then `pnpm --filter @mosaic/i18n test` then `pnpm --filter @mosaic/i18n typecheck`
Expected: tests PASS; `git status` shows only the two catalogs and the five generated files; `L10n.kt` contains `fun linkConfirmTitle(r: Resources, login: String, app: String)`.

- [ ] **Step 3: Commit**

```bash
git add packages/i18n apps/mobile/android/app/src/main/res apps/mobile/android/app/src/main/kotlin/io/github/stephanbaum/mosaic/ui/l10n/L10n.kt apps/mobile/ios/Resources/Localizable.xcstrings apps/mobile/ios/MosaicKit/L10n/L10n.swift
git commit -m "feat(i18n): one-tap link strings (EN, DE)"
```

---

### Task 4: Link entries in the Autofill response

**Files:**
- Modify: `K/autofill/FillIntents.kt`, `K/autofill/FillPlanner.kt`, `K/autofill/FillResponseFactory.kt`, `K/autofill/MosaicAutofillService.kt`, `K/autofill/FillAuthActivity.kt` (only the `MODE_LIST` branch of `work`)
- Create: `apps/mobile/android/app/src/androidTest/kotlin/io/github/stephanbaum/mosaic/autofill/AutofillTestKit.kt`
- Create: `apps/mobile/android/app/src/androidTest/kotlin/io/github/stephanbaum/mosaic/autofill/OneTapLinkInstrumentedTest.kt`
- Modify: `apps/mobile/android/app/src/androidTest/kotlin/io/github/stephanbaum/mosaic/autofill/AutofillInstrumentedTest.kt` (use the kit)

**Interfaces:**
- Consumes: Task 2 FFI (`fillLinkOffer`, `LinkOffer`, `LinkCandidate`, `LinkReason`); Task 3 accessors.
- Produces:
  - `FillIntents.MODE_LINK = "link"`; datasets use `FillIntents.withItem(info.toIntent(context, FillIntents.MODE_LINK), candidate.key, FillIntents.KIND_LOGIN)`.
  - `suspend fun FillPlanner.linkOffer(sessionId: String): LinkOffer?` (lenient: any exception except `CancellationException` → `null`).
  - `FillResponseFactory.candidatesResponse(info: FillRequestInfo, candidates: List<FillCandidate>, warning: Boolean, noLoginLabel: String?, linkCandidates: List<LinkCandidate> = emptyList()): FillResponse?`
  - `DatasetAudit` kind `"link"` for a link entry.
  - `AutofillTestKit(instrumentation)` with the helpers now private in `AutofillInstrumentedTest`: `shell`, `findText`, `waitFor`, `tap`, `showClientAndFocusUsername`, `screenTexts`, `resetAccount`, `draft`, moved without behaviour change.

- [ ] **Step 1: Move the instrumented helpers into `AutofillTestKit` and write the failing instrumented test**

`OneTapLinkInstrumentedTest.setUp`: the same device and account setup as `AutofillInstrumentedTest`, but the only login is `"GitHub"` / `"octocat"` / `"hunter2"` with URL `"androidapp://$clientPackage"` (no `cert`, so nothing matches).

```kotlin
@Test
fun noValuesBeforeConfirm() {
    kit.showClientAndFocusUsername()
    val datasets = kit.waitFor(30_000) { FillAudit.last.takeIf { list -> list.any { it.kind == "link" } } }
    assertNotNull("no link entry; the screen shows: ${kit.screenTexts()}", datasets)
    for (dataset in datasets!!) {
        assertTrue(dataset.authenticated)
        assertEquals(0, dataset.nonNullValues)
    }
    assertEquals(1, datasets.count { it.kind == "link" })
    assertEquals(1, datasets.count { it.kind == "noLogin" })
    assertNotNull(kit.waitFor(10_000) { kit.findText(L10n.autofillLinkEntry(context.resources, "GitHub")) })
    assertTrue(kit.findText("client-username")?.text.isNullOrEmpty())
    assertTrue(kit.findText("client-password")?.text.isNullOrEmpty())
}
```

- [ ] **Step 2: Implement**

- `FillIntents`: `MODE_LINK`, KDoc "A tap on a link entry: confirm, link the login to the app, then fill (biometric) or link only (fallback)".
- `FillPlanner.linkOffer`.
- `FillResponseFactory.candidatesResponse`: when no login dataset was added and `noLoginLabel != null`: for each of `linkCandidates.take(3)` and only if `info.fields.hasLogin`, add `authenticated(listOfNotNull(info.fields.username, info.fields.password), L10n.autofillLinkEntry(resources, c.title), reasonText(c.reason), intent)` with the `MODE_LINK` intent above, audit `DatasetAudit("link", true, 0)`; then the existing "No login for <app>" dataset as the last entry, audit `"noLogin"`; record all audits together. `reasonText`: `VerifiedBy` → `L10n.linkVerified(resources, host)`, `ImportedForApp` → `L10n.linkImported(resources)`.
- `MosaicAutofillService.respond` and the `MODE_LIST` branch of `FillAuthActivity.work`: after `planner.candidates(target)`, when `candidates.candidates.isEmpty()` and `label != null`, `val links = planner.linkOffer(candidates.sessionId)?.candidates.orEmpty()`, passed as `linkCandidates`.

- [ ] **Step 3: Verify in CI**

Commit, push the branch, and leave the CI result to the orchestrator: `mobile.yml` jobs "Android build and JVM tests" and "Android instrumented (API 34 emulator)" must pass, including `OneTapLinkInstrumentedTest.noValuesBeforeConfirm` and every existing `AutofillInstrumentedTest` test.

- [ ] **Step 4: Commit**

```bash
git add apps/mobile/android
git commit -m "feat(android): offer one-tap link entries in the Autofill response"
```

---

### Task 5: Link mode in `FillAuthActivity` (confirm, link, fill)

**Files:**
- Create: `K/autofill/LinkFlow.kt`
- Modify: `K/security/BiometricGate.kt`, `K/autofill/FillAuthActivity.kt`
- Create: `K/ui/link/AppIdentityPanel.kt`; Modify: `K/ui/link/LinkAppScreen.kt` (use the panel)
- Create: `apps/mobile/android/app/src/test/kotlin/io/github/stephanbaum/mosaic/autofill/LinkFlowTest.kt`
- Modify: `apps/mobile/android/app/src/androidTest/kotlin/io/github/stephanbaum/mosaic/autofill/OneTapLinkInstrumentedTest.kt`
- Modify: `docs/MOBILE.md` (§3 table row "Autofill"; §7 Android check list)

**Interfaces:**
- Consumes: Task 2 FFI (`fillLinkOffer`, `linkAppInFillSession`, `fillValues`); Task 4 `FillIntents.MODE_LINK`, `FillPlanner.target`, `FillPlanner.candidates`; Task 3 accessors; existing `CallerReader.labelOf`, `appDetails(context, pkg)?.installer`, `FillResponseFactory.valuesDataset`.
- Produces:

```kotlin
enum class LinkConfirm { Biometric, Screen }
object LinkFlow {
    fun confirmMode(strongBiometricAvailable: Boolean): LinkConfirm
    /** Only the biometric confirmation fills in the same step (spec §5.5.2). */
    fun fillsAfterLink(mode: LinkConfirm): Boolean
}
// BiometricGate
suspend fun confirm(context: Context, text: PromptText, description: String): Boolean
// ui/link
@Composable fun AppIdentityPanel(label: String, packageName: String, installer: String?, fingerprint: String)
```

- [ ] **Step 1: Write the failing JVM test**

```kotlin
class LinkFlowTest {
    @Test fun aStrongBiometricConfirmsWithThePrompt() =
        assertEquals(LinkConfirm.Biometric, LinkFlow.confirmMode(strongBiometricAvailable = true))
    @Test fun withoutOneTheScreenConfirms() =
        assertEquals(LinkConfirm.Screen, LinkFlow.confirmMode(strongBiometricAvailable = false))
    @Test fun onlyTheBiometricPathFills() {
        assertTrue(LinkFlow.fillsAfterLink(LinkConfirm.Biometric))
        assertFalse(LinkFlow.fillsAfterLink(LinkConfirm.Screen))
    }
}
```

- [ ] **Step 2: Write the failing instrumented test**

In `OneTapLinkInstrumentedTest` (the CI emulator has no enrolled biometric, so the screen path runs):

```kotlin
@Test
fun oneTapLinkNeedsBiometricConfirm() {
    kit.showClientAndFocusUsername()
    kit.tap(checkNotNull(kit.waitFor(30_000) { kit.findText(L10n.autofillLinkEntry(res, "GitHub")) }))
    val confirm = kit.waitFor(30_000) { kit.findText(L10n.linkConfirm(res, "GitHub")) }
    assertNotNull("no confirmation; the screen shows: ${kit.screenTexts()}", confirm)
    assertFalse("linked before the confirm", boundUrlPresent())
    kit.tap(checkNotNull(kit.findText(L10n.commonCancel(res))))
    assertFalse("linked after Cancel", boundUrlPresent())
    // Again, and confirm this time.
    kit.showClientAndFocusUsername()
    kit.tap(checkNotNull(kit.waitFor(30_000) { kit.findText(L10n.autofillLinkEntry(res, "GitHub")) }))
    kit.tap(checkNotNull(kit.waitFor(30_000) { kit.findText(L10n.linkConfirm(res, "GitHub")) }))
    assertNotNull(kit.waitFor(30_000) { kit.findText(L10n.linkDone(res, "GitHub")) })
    assertTrue(boundUrlPresent())
    // The screen path links only: nothing was filled.
    assertTrue(kit.findText("client-password")?.text.isNullOrEmpty())
}
```

`boundUrlPresent()`: the item's URLs (via `app.holder.call { it.item(vault, item) }`) contain `"androidapp://$clientPackage?cert=" + base64url(signer)`, where `signer` is `CallerReader.signersOf(context, clientPackage)!!.single()`.

- [ ] **Step 3: Implement**

- `LinkFlow`: `confirmMode` = `Biometric` iff the argument is true; `fillsAfterLink` = `mode == Biometric`.
- `BiometricGate.confirm`: framework `BiometricPrompt` without a `CryptoObject`; title, subtitle, `setDescription(description)`, negative button `text.negativeButton`; API 30+ `setAllowedAuthenticators(BIOMETRIC_STRONG)`. `true` only from `onAuthenticationSucceeded`; cancel, negative button and every error give `false`. Main thread, cancellable like `authenticate`.
- `AppIdentityPanel`: the `Panel` block of `LinkAppScreen` (app, package, installer or `linkInstallerUnknown`, certificate) moved into its own composable; `LinkAppScreen` calls it with `chosen.certificateHex`.
- `FillAuthActivity`:
  - `onCreate`: `MODE_LINK` is a valid mode.
  - `work()` for `MODE_LINK`: `target = planner.target(this, info) ?: return null` (re-reads the caller's signers now); `val found = planner.candidates(target)`; if `found.candidates` is not empty → `throw NoLongerMatches()` (the app has a match now; the normal entry fills); `val offer = app.holder.call { it.fillLinkOffer(found.sessionId) }` (strict: errors stop); the tapped key (`EXTRA_ITEM_VAULT`, `EXTRA_ITEM_ID`) must be in `offer.candidates`, else `NoLongerMatches`; then `throw NeedsLinkConfirm(LinkPending(found.sessionId, offer, chosen, CallerReader.labelOf(this, info.callerPackage)))`.
  - `Runner` keeps `LinkPending` in screen memory only (like the address warning). `LinkConfirm.Biometric` (from `BiometricGate().isAvailable(context)`): a `LaunchedEffect` shows `confirm` with title `L10n.linkConfirmTitle(r, chosen.title, label)`, subtitle `L10n.linkConfirmSubtitle(r, offer.packageName, offer.signerPrefix)`, description = the reason text, negative button `L10n.commonCancel(r)`; `false` → `abandon(app)`; `true` → `linkAppInFillSession(sessionId, key)`, then `fillValues(sessionId, key)` → `factory.valuesDataset(info, KIND_LOGIN, username, secret)` (wipe the password bytes exactly as `fill` does) → `finishWith(app, MODE_LINK, data)`. `LinkConfirm.Screen`: `ScreenFrame` with `TitleText(L10n.linkTitle(r))`, `AppIdentityPanel(label, offer.packageName, appDetails(this, offer.packageName)?.installer, offer.signerPrefix)`, `Line(reason text)`, `BodyText(L10n.linkConfirmFallbackHint(r))`, `PrimaryButton(L10n.linkConfirm(r, chosen.title))` → `linkAppInFillSession` → `Notice(L10n.linkDone(r, chosen.title))` + `PrimaryButton(L10n.commonDone(r)) { abandon(app) }`; `QuietButton(L10n.commonCancel(r)) { abandon(app) }`. The screen path never calls `fillValues`.
  - `finishWith`: close the provider flow for `MODE_FILL` and `MODE_LINK`.
  - Errors from `linkAppInFillSession` show the existing error screen (`ErrorCodes.of(e)`).
  - Never call the old `linkAndroidApp(vault, item, AndroidCaller)` in this flow.
- `docs/MOBILE.md` §3 "Autofill" row: add "one-tap linking of strong candidates from the fill session, confirmed by the biometric prompt (ADR-0092 amendment)". §7 Android list: add
  `- [ ] One-tap linking: an imported login and a site with assetlinks.json each offer "Link <title> and fill"; the biometric prompt names the login, app, App ID, signature start and the reason, and is readable on a small screen at 200 % font (ux-reviewer); Cancel links nothing; confirm links and fills; a sideloaded copy with another key gets no offer for a verified site.`

- [ ] **Step 4: Verify in CI**

Commit, push, and leave the result to the orchestrator: `mobile.yml` must pass, including `LinkFlowTest`, `OneTapLinkInstrumentedTest` (both tests), all existing tests, `check-secure-windows.sh` and `lintDebug`.

- [ ] **Step 5: Commit**

```bash
git add apps/mobile/android docs/MOBILE.md
git commit -m "feat(android): one-tap link confirmation with the biometric prompt (spec 5.5.2)"
```

- [ ] **Step 6: Hand-off checks (orchestrator)**

Run the `ux-reviewer` agent on the biometric prompt text and the fallback screen (DE and EN), and the `test-engineer` agent on `crates/mobile/src/autofill.rs` new paths, before the milestone commit.

---

## Spec coverage (self-review)

| Spec item | Task |
|---|---|
| §5.5.2 strong candidates (imported exact package; asset links Verified with `get_login_creds`) | 2 (core), 1 (domain rule, fetch) |
| §5.5.2 at most 3, value-less datasets, per-dataset authentication | 2 (`MAX_LINK_CANDIDATES`), 4 |
| §5.5.2 tap opens `FillAuthActivity`; biometric prompt is the confirmation naming label, package, signer prefix, reason; links and fills | 5 |
| §5.5.2 master-password case shows the existing link screen | 5 (screen path with the same identity panel; links only) |
| §5.5.2 weak cases keep the full flow; browsers excluded | 4 (no entry), 2 (`InvalidInput`) |
| §4 table: ADR-0092 amendment | 1 |
| §6.2 Android linking row, R136 | 1 |
| §6.3 `one_tap_link_offered_only_for_verified_or_exact_package_candidates`, `browser_package_never_gets_a_link_offer`, `oneTapLinkNeedsBiometricConfirm`, `noValuesBeforeConfirm` | 2, 2, 5, 4 |
| §5.5.2 "Not verified" prompt text on small screens | 5 (manual check, ux-reviewer) |
