import XCTest

@testable import MosaicKit

/// A store that records what the sync asked of it.
private final class RecordingStore: CredentialIdentityStoring, @unchecked Sendable {
    private let lock = NSLock()
    private let enabled: Bool
    private var log: [String] = []

    init(enabled: Bool) {
        self.enabled = enabled
    }

    var calls: [String] {
        lock.lock()
        defer { lock.unlock() }
        return log
    }

    private func record(_ entry: String) {
        lock.lock()
        defer { lock.unlock() }
        log.append(entry)
    }

    func isEnabled() async -> Bool { enabled }

    func replaceAll(_ plan: IdentityPlan) async throws {
        record("replace passkeys=\(plan.passkeys.count) passwords=\(plan.passwords.count)")
    }

    func removeAll() async throws {
        record("removeAll")
    }
}

final class IdentityStoreSyncTests: XCTestCase {
    private func settings(passkeys: Bool, passwords: Bool) -> MobileSettings {
        MobileSettings(
            autoLockBackgroundSeconds: 0, idleLockMinutes: 5, clipboardClearSeconds: 30, quickUnlockMaxAgeDays: 7,
            passkeyIdentities: passkeys, passwordIdentities: passwords)
    }

    private let key = ItemKey(vaultId: "v1", itemId: "i1")

    private var passkey: PasskeyIdentity {
        PasskeyIdentity(
            rpId: "example.com", userName: "alice", credentialId: Data([1, 2]), userHandle: Data([3]), record: key)
    }

    private var password: PasswordIdentity {
        PasswordIdentity(domain: "example.com", userName: "alice", record: key)
    }

    func testPasswordIdentitiesOnlyWhenOptedIn() {
        let off = IdentityStoreSync.plan(
            settings: settings(passkeys: true, passwords: false), passkeys: [passkey], passwords: [password])
        XCTAssertEqual(off.passkeys.count, 1)
        XCTAssertTrue(off.passwords.isEmpty)

        let on = IdentityStoreSync.plan(
            settings: settings(passkeys: true, passwords: true), passkeys: [passkey], passwords: [password])
        XCTAssertEqual(on.passkeys.count, 1)
        XCTAssertEqual(on.passwords.count, 1)

        let none = IdentityStoreSync.plan(
            settings: settings(passkeys: false, passwords: false), passkeys: [passkey], passwords: [password])
        XCTAssertTrue(none.isEmpty)
    }

    func testRecordIdentifierNamesTheItem() {
        XCTAssertEqual(IdentityStoreSync.recordIdentifier(key), "v1/i1")
    }

    func testApplyClearsWhenEmptyWritesWhenNotAndSkipsADisabledStore() async {
        let empty = IdentityPlan(passkeys: [], passwords: [])
        let full = IdentityPlan(passkeys: [passkey], passwords: [password])

        let enabled = RecordingStore(enabled: true)
        await IdentityStoreSync.apply(empty, to: enabled)
        await IdentityStoreSync.apply(full, to: enabled)
        XCTAssertEqual(enabled.calls, ["removeAll", "replace passkeys=1 passwords=1"])

        let disabled = RecordingStore(enabled: false)
        await IdentityStoreSync.apply(full, to: disabled)
        XCTAssertTrue(disabled.calls.isEmpty)
    }

    func testCurrentPlanOfAFreshVaultIsEmptyBecauseBothSettingsStartOff() async throws {
        let directory = try TestSupport.storeDirectory(for: self)
        let core = try TestSupport.openCore(in: directory)
        let account = try MosaicTestHooks(core: core).createLocalAccount(
            email: "t@example.com", password: Data("correct horse battery staple 42".utf8))
        account.secretKey.wipe()
        let vault = try XCTUnwrap(try core.vaults().first(where: { $0.available })?.vaultId)
        _ = try core.saveLogin(
            draft: LoginDraft(
                vaultId: vault, itemId: nil, title: "Example", username: "alice",
                password: Data("pw-1234567890".utf8),
                urls: [UrlDraft(url: "https://example.com", matchEtld1: false, allowHttp: false)],
                notes: nil, otp: .keep, favorite: false))
        let holder = CoreHolder(core: core)
        let plan = await IdentityStoreSync.currentPlan(holder: holder)
        XCTAssertNotNil(plan)
        XCTAssertTrue(plan?.isEmpty ?? false)
        // Locked: the store is left as it is.
        holder.lock()
        let locked = await IdentityStoreSync.currentPlan(holder: holder)
        XCTAssertNil(locked)
    }
}
