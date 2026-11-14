import XCTest

@testable import MosaicKit

/// M10-G3 red team (iOS): hostile service identifiers, record hints and lifecycle races against the
/// extension's logic (`ProviderSession`, `ProviderFillPolicy`, `IdentityStoreSync`, `RequestLifecycle`) on a
/// real core in the extension context. The tests pin defenses that held and the fixes of the holes found.
/// The core's own decisions (identifier disguises, rp IDs, the fill session)
/// are pinned in `crates/mobile/tests/it/g3i_redteam.rs`.
final class RedTeamG3iTests: XCTestCase {
    private func domain(_ value: String) -> ServiceIdentifier {
        ServiceIdentifier(kind: .domain, value: value)
    }

    private func pageUrl(_ value: String) -> ServiceIdentifier {
        ServiceIdentifier(kind: .url, value: value)
    }

    private func unlockedSession(_ phone: TestPhone) async throws -> ProviderSession {
        let session = ProviderSession(holder: try phone.extensionHolder(), keys: phone.keys)
        _ = try await session.begin()
        try await session.unlockQuick(reason: "test", cancelTitle: "Cancel")
        return session
    }

    // MARK: Service identifiers

    /// Attack: several identifiers, a refused one or another site first. Defense: the first identifier the
    /// core accepts is the target; the logins of a later identifier neither match nor fill.
    func testOnlyTheFirstAcceptedIdentifierIsTheTarget() async throws {
        let phone = try TestPhone(for: self)
        var github = ItemKey(vaultId: "", itemId: "")
        try phone.seed { core, _ in
            github = try phone.login(
                core, title: "GitHub", username: "octo", password: "gh-secret-pw", url: "https://github.com")
        }
        let session = try await unlockedSession(phone)
        for identifiers in [
            [domain("10.0.0.1"), domain("evil.example"), domain("github.com")],
            [pageUrl("javascript:alert(1)"), pageUrl("https://github.com@evil.example/"), domain("github.com")],
            [domain("github.com:443"), domain("evil.example"), pageUrl("https://github.com/")],
        ] {
            let list = try await session.list(identifiers: identifiers)
            XCTAssertEqual(list.site, "evil.example")
            XCTAssertTrue(list.matches.isEmpty)
            XCTAssertEqual(list.others.map(\.itemId), [github.itemId], "read-only, no select action")
            do {
                _ = try await session.password(for: github)
                XCTFail("a later identifier must not make its login fill")
            } catch {
                XCTAssertEqual(TestSupport.code(error), "noMatch")
            }
        }
        // Only refused identifiers: no target, the refusal is named, nothing fills.
        let none = try await session.list(identifiers: [domain("127.0.0.1"), pageUrl("file:///etc/passwd")])
        XCTAssertNil(none.target)
        XCTAssertNil(none.site)
        XCTAssertEqual(none.refusalCode, "insecureOrigin")
        XCTAssertTrue(none.matches.isEmpty)
        do {
            _ = try await session.password(for: github)
            XCTFail("without a target nothing fills")
        } catch {
            // The fill session of the earlier list is still the current one (a list without a target does not
            // release it; see the report), and the core refuses the key against it.
            XCTAssertTrue(["noMatch", "notFound"].contains(TestSupport.code(error)), TestSupport.code(error))
        }
        session.finish()
    }

    /// The confirm step names the site the core then fills: for the identities Mosaic writes (the core's
    /// ASCII host as a `.domain` identifier), `savedSite` and the core's target agree, punycode included.
    func testTheConfirmedSiteIsTheSiteTheCoreTargetsForADomainIdentifier() throws {
        let directory = try TestSupport.storeDirectory(for: self)
        let core = try TestSupport.openCore(in: directory, context: .iosAutofillExtension)
        defer { core.closeStore() }
        for host in ["github.com", "xn--bcher-kva.example", "login.example.co.uk"] {
            let identifiers = [domain(host)]
            XCTAssertEqual(ProviderFillPolicy.savedSite(of: identifiers), host)
            let target = try core.fillTargetIos(identifier: identifiers[0])
            XCTAssertEqual(target.display, host)
        }
    }

    /// M10-G3i-3. `ProviderFillPolicy.savedSite` reads a `.URL` identifier with Foundation's parser
    /// (`URL(string:).host`), the core with the WHATWG parser. They disagree on userinfo tricks (a backslash
    /// before `@`), on case and on IDNs, so the confirm step could name one site while the core fills
    /// another. Not reachable today, because the identities Mosaic writes are `.domain` identifiers; the
    /// fix is to name the core's `fillTargetIos(...).display` and never parse in Swift.
    func testTheConfirmedSiteIsNeverParsedInSwiftForAUrlIdentifier() throws {
        for page in ["https://evil.example\\@github.com/", "https://GitHub.com/", "https://bücher.example/"] {
            XCTAssertNil(ProviderFillPolicy.savedSite(of: [pageUrl(page)]), page)
        }
        XCTAssertEqual(ProviderFillPolicy.savedSite(of: [domain("github.com")]), "github.com")
    }

    // MARK: The record hint

    /// The record ID is `<vault>/<item>` and nothing else; anything malformed is no hint at all.
    func testMalformedRecordIdentifiersAreNoHint() {
        let bad: [String?] = [nil, "", "v", "/i", "v/", "v/i/x", "//", "v//i", "/"]
        for value in bad {
            XCTAssertNil(IdentityStoreSync.parseRecordIdentifier(value), String(describing: value))
        }
        XCTAssertEqual(IdentityStoreSync.parseRecordIdentifier("v/i"), ItemKey(vaultId: "v", itemId: "i"))
    }

    /// A QuickType identity whose record names another login (another site, another vault) never selects it,
    /// even after the user confirmed, and the core refuses the password for that fill session.
    func testATamperedRecordNeverFillsAnotherLoginEvenAfterTheConfirm() async throws {
        let phone = try TestPhone(for: self)
        var bank = ItemKey(vaultId: "", itemId: "")
        var evil = ItemKey(vaultId: "", itemId: "")
        try phone.seed { core, _ in
            bank = try phone.login(
                core, title: "Bank", username: "alice", password: "bank-secret-pw", url: "https://bank.example")
            evil = try phone.login(
                core, title: "Evil", username: "mallory", password: "evil-pw", url: "https://evil.example")
        }
        let session = try await unlockedSession(phone)
        let list = try await session.list(identifiers: [domain("evil.example")])
        let matches = list.matches.map(\.key)
        XCTAssertEqual(matches, [evil])
        XCTAssertNil(ProviderFillPolicy.loginToFill(record: bank, matches: matches, confirmed: true))
        XCTAssertNil(ProviderFillPolicy.loginToFill(record: evil, matches: matches, confirmed: false))
        XCTAssertNil(ProviderFillPolicy.loginToFill(record: nil, matches: matches, confirmed: true))
        let foreignVault = ItemKey(vaultId: "00000000-0000-7000-8000-00000000beef", itemId: bank.itemId)
        XCTAssertNil(ProviderFillPolicy.loginToFill(record: foreignVault, matches: matches, confirmed: true))
        XCTAssertEqual(ProviderFillPolicy.loginToFill(record: evil, matches: matches, confirmed: true), evil)
        do {
            _ = try await session.password(for: bank)
            XCTFail("the bank's login must not fill evil.example")
        } catch {
            XCTAssertEqual(TestSupport.code(error), "noMatch")
        }
        do {
            _ = try await session.password(for: foreignVault)
            XCTFail("a key of another vault must not fill")
        } catch {
            XCTAssertNotEqual(TestSupport.code(error), "")
        }
        session.finish()
    }

    /// A tap on a QuickType identity, end to end: the identity names the host of the login's URL, the request
    /// targets that host, and the identity's record fills only when the login still matches it exactly. A
    /// login on another port gets an identity that never fills (never another login's password either).
    func testAQuickTypeIdentityFillsOnlyItsOwnLoginOnItsOwnSite() async throws {
        let phone = try TestPhone(for: self)
        var ok = ItemKey(vaultId: "", itemId: "")
        var ported = ItemKey(vaultId: "", itemId: "")
        var identities: [PasswordIdentity] = []
        try phone.seed { core, _ in
            ok = try phone.login(
                core, title: "OK", username: "dave", password: "ok-pw", url: "https://user:pw@ok.example/login?x")
            ported = try phone.login(
                core, title: "Ported", username: "bob", password: "ported-pw", url: "https://ok.example:8443")
            _ = try core.updateSettings(
                update: SettingsUpdate(
                    autoLockBackgroundSeconds: nil, idleLockMinutes: nil, clipboardClearSeconds: nil,
                    quickUnlockMaxAgeDays: nil, passkeyIdentities: nil, passwordIdentities: true,
                    reauthPassword: Data(TestPhone.password.utf8)))
            identities = try core.passwordIdentities()
        }
        XCTAssertEqual(Set(identities.map(\.domain)), ["ok.example"], "both logins name the same host")
        XCTAssertEqual(
            identities.map(\.record).sorted { $0.itemId < $1.itemId }, [ok, ported].sorted { $0.itemId < $1.itemId })
        let session = try await unlockedSession(phone)
        for identity in identities {
            // What `prepareInterfaceToProvideCredential` gets: the identity's domain and its record ID.
            let identifiers = [domain(identity.domain)]
            let record = IdentityStoreSync.parseRecordIdentifier(IdentityStoreSync.recordIdentifier(identity.record))
            XCTAssertEqual(record, identity.record)
            XCTAssertEqual(ProviderFillPolicy.savedSite(of: identifiers), "ok.example")
            let list = try await session.list(identifiers: identifiers)
            XCTAssertEqual(list.site, "ok.example")
            XCTAssertEqual(list.matches.map(\.key), [ok], "only the exact origin matches")
            let chosen = ProviderFillPolicy.loginToFill(
                record: record, matches: list.matches.map(\.key), confirmed: true)
            if identity.record == ok {
                XCTAssertEqual(chosen, ok)
                let filled = try await session.password(for: ok)
                XCTAssertEqual(filled.user, "dave")
                XCTAssertEqual(filled.password, "ok-pw")
            } else {
                XCTAssertNil(chosen, "the ported login is not a match for https://ok.example")
                do {
                    _ = try await session.password(for: ported)
                    XCTFail("a login on another port must not fill from its identity")
                } catch {
                    XCTAssertEqual(TestSupport.code(error), "noMatch")
                }
            }
        }
        session.finish()
    }

    // MARK: Lifecycle

    /// M10-G3i-1. The request ends (a dismissal, the host going away) while the core runs `unlock_quick`:
    /// `finish()` calls `closeStore()`, which finds the phase `Unlocking`, only requests a lock and does not
    /// close. When the unlock ends the store is `Locked` and open, its App Group file lock held, and nobody
    /// will close it again. The same happens to the app when it goes to the background during a master
    /// password unlock (`0xdead10cc`). The core test is `g3i_1_close_store_during_an_unlock_ends_closed`.
    func testAFinishDuringTheQuickUnlockClosesTheStore() async throws {
        let phone = try TestPhone(for: self)
        try phone.seed()
        let core = try phone.openCore(.iosAutofillExtension)
        MosaicTestHooks(core: core).setWorkDelayMs(ms: 400)
        let holder = CoreHolder(core: core)
        let session = ProviderSession(holder: holder, keys: phone.keys)
        let lifecycle = RequestLifecycle { session.finish() }
        _ = try await session.begin()
        let unlock = Task { try await session.unlockQuick(reason: "test", cancelTitle: "Cancel") }
        // The unlock is past `take_locked` once the core reports a running long call.
        let start = Date()
        while true {
            if case .busy = holder.state() { break }
            XCTAssertLessThan(Date().timeIntervalSince(start), 10, "the core never got busy")
            try await Task.sleep(for: .milliseconds(5))
        }
        XCTAssertTrue(lifecycle.end())
        _ = try? await unlock.value
        guard case .storeClosed = holder.state() else {
            return XCTFail("a finished request must leave the store closed, got \(holder.state())")
        }
    }

    /// M10-G3i-2. `ProviderModel.start` checks `isFinished` only before `await session.begin()`. When the app
    /// held the store, `begin()` reopens it on the core's queue; a dismissal (or a newer request) that ends the
    /// request meanwhile closes nothing (the phase is still `Closed`), and the reopen then takes the store and
    /// its App Group file lock for a request nobody will finish again.
    func testADismissalWhileBeginReopensTheStoreLeavesItClosed() async throws {
        let phone = try TestPhone(for: self)
        try phone.seed()
        // The app holds the store, so the extension's begin() has to reopen it.
        let app = try phone.openCore(.app)
        let holder = try phone.extensionHolder()
        let session = ProviderSession(holder: holder, keys: phone.keys)
        let lifecycle = RequestLifecycle { session.finish() }
        // A running call keeps the core's serial queue busy, so the reopen waits behind it.
        let blocker = Task { try await holder.call { _ in Thread.sleep(forTimeInterval: 1.5) } }
        try await Task.sleep(for: .milliseconds(200))
        app.closeStore()
        XCTAssertFalse(lifecycle.isFinished)
        let begin = Task { try await session.begin() }
        try await Task.sleep(for: .milliseconds(300))
        // The sheet is dismissed while begin() waits.
        XCTAssertTrue(lifecycle.end())
        try await blocker.value
        _ = try await begin.value
        guard case .storeClosed = holder.state() else {
            return XCTFail("a finished request must leave the store closed, got \(holder.state())")
        }
    }

    /// A `finish()` that comes before `begin()` ends the request that `begin()` starts: it reports `.busy` and
    /// leaves the store closed. The next `begin()` is a new request.
    func testAFinishBeforeBeginEndsTheRequestItBegins() async throws {
        let phone = try TestPhone(for: self)
        try phone.seed()
        let holder = try phone.extensionHolder()
        let session = ProviderSession(holder: holder, keys: phone.keys)
        session.finish()
        let ended = try await session.begin()
        XCTAssertEqual(ended, .busy)
        guard case .storeClosed = holder.state() else {
            return XCTFail("a finished request must leave the store closed, got \(holder.state())")
        }
        let next = try await session.begin()
        XCTAssertEqual(next, .quickUnlock)
        session.finish()
    }

    /// Held: a second request while the first is unlocked finds the store busy, inherits no verification and
    /// no fill session, and opens the store only after the first one finished; the first one's handles are
    /// dead from then on.
    func testASecondRequestFindsTheStoreBusyAndInheritsNothing() async throws {
        let phone = try TestPhone(for: self)
        var github = ItemKey(vaultId: "", itemId: "")
        try phone.seed { core, _ in
            github = try phone.login(
                core, title: "GitHub", username: "octo", password: "gh-secret-pw", url: "https://github.com")
        }
        let first = try await unlockedSession(phone)
        let firstList = try await first.list(identifiers: [domain("github.com")])
        XCTAssertEqual(firstList.matches.map(\.key), [github])

        let secondHolder = try phone.extensionHolder()
        let second = ProviderSession(holder: secondHolder, keys: phone.keys)
        let busy = try await second.begin()
        XCTAssertEqual(busy, .busy)
        do {
            _ = try await second.list(identifiers: [domain("github.com")])
            XCTFail("a busy store lists nothing")
        } catch {
            XCTAssertNotEqual(TestSupport.code(error), "")
        }
        do {
            _ = try await second.assertPasskey(
                rpId: "example.com", clientDataHash: Data(repeating: 1, count: 32), credentialId: Data([1]))
            XCTFail("the second request has no verification of its own")
        } catch {
            XCTAssertEqual(TestSupport.code(error), "reauthRequired")
        }

        first.finish()
        let again = try await second.begin()
        XCTAssertEqual(again, .quickUnlock, "locked: the second request verifies itself")
        do {
            _ = try await first.password(for: github)
            XCTFail("the first request's fill session is gone")
        } catch {
            XCTAssertEqual(TestSupport.code(error), "notFound")
        }
        try await second.unlockQuick(reason: "test", cancelTitle: "Cancel")
        let list = try await second.list(identifiers: [domain("github.com")])
        XCTAssertEqual(list.matches.map(\.key), [github])
        let filled = try await second.password(for: github)
        XCTAssertEqual(filled.password, "gh-secret-pw")
        second.finish()
        guard case .storeClosed = secondHolder.state() else {
            return XCTFail("the second request must close the store, got \(secondHolder.state())")
        }
    }
}
