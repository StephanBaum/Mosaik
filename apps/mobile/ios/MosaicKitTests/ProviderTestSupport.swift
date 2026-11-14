import Foundation
import XCTest

@testable import MosaicKit

/// A Secure Enclave stand-in: it keeps `W` by alias and hands it back without a prompt.
final class FakeQuickUnlockKeys: QuickUnlockKeys, @unchecked Sendable {
    private let lock = NSLock()
    private var secrets: [String: Data] = [:]

    var isAvailable: Bool { true }

    private func store(_ secret: Data, for alias: String) {
        lock.lock()
        defer { lock.unlock() }
        secrets[alias] = secret
    }

    func secret(for alias: String) -> Data? {
        lock.lock()
        defer { lock.unlock() }
        return secrets[alias]
    }

    private func remove(_ alias: String) {
        lock.lock()
        defer { lock.unlock() }
        secrets[alias] = nil
    }

    func enroll(alias: String, secret: Data) throws -> Data {
        store(secret, for: alias)
        // The core only checks the length of the body (113 bytes of ECIES for the Secure Enclave row).
        return Data(repeating: 9, count: SecureEnclaveQuickUnlock.ciphertextLength)
    }

    func decrypt(alias: String, ciphertext: Data, reason: String, cancelTitle: String) async throws -> Data {
        guard let secret = secret(for: alias) else { throw QuickUnlockKeyError.invalidated }
        return secret
    }

    func deleteKey(alias: String) throws {
        remove(alias)
    }
}

/// A phone for the extension tests: one store directory and one secure storage that the app side and the
/// extension side share, like the real App Group and Keychain.
final class TestPhone {
    static let password = "correct horse battery staple 42"

    let directory: URL
    let storage = MemorySecureStorage()
    let keys = FakeQuickUnlockKeys()
    private(set) var vaultId = ""

    init(for test: XCTestCase) throws {
        directory = try TestSupport.storeDirectory(for: test)
    }

    func openCore(_ context: CoreContext) throws -> MosaicCore {
        try MosaicCore.open(
            config: CoreFactory.config(storeDirectory: directory, context: context), storage: storage)
    }

    /// The app side: a local account with one vault, then `build` adds items, and the store is closed (the app
    /// went to the background). With `enrolQuickUnlock` the fake key is enrolled, as after "Unlock with Face
    /// ID next time".
    func seed(enrolQuickUnlock: Bool = true, build: (MosaicCore, String) throws -> Void = { _, _ in }) throws {
        let core = try openCore(.app)
        let account = try MosaicTestHooks(core: core).createLocalAccount(
            email: "t@example.com", password: Data(Self.password.utf8))
        account.secretKey.wipe()
        vaultId = try XCTUnwrap(try core.vaults().first(where: { $0.available })?.vaultId)
        try build(core, vaultId)
        if enrolQuickUnlock {
            try QuickUnlockFlow.enrollOrThrow(core: core, password: Data(Self.password.utf8), keys: keys)
        }
        core.closeStore()
    }

    /// The extension side: a new process of the same boot, with the extension context.
    func extensionHolder() throws -> CoreHolder {
        CoreHolder(core: try openCore(.iosAutofillExtension))
    }

    /// The key alias of the quick-unlock enrolment, read by a core that is closed again at once.
    func enrolledAlias() throws -> String {
        let core = try openCore(.iosAutofillExtension)
        defer { core.closeStore() }
        guard case let .locked(_, _, availability) = core.state(), case let .available(_, alias, _, _) = availability
        else {
            throw CodedError("notFound")
        }
        return alias
    }

    /// The extension side with a core whose clock is `days` ahead: an enrolment older than its maximum age (30
    /// days at most) has run out for this core.
    func extensionHolder(clockAheadDays days: Int64) throws -> CoreHolder {
        let core = try openCore(.iosAutofillExtension)
        MosaicTestHooks(core: core).setClockOffsetMs(offset: days * 24 * 60 * 60 * 1000)
        return CoreHolder(core: core)
    }

    func login(
        _ core: MosaicCore, title: String, username: String, password: String, url: String
    ) throws -> ItemKey {
        try core.saveLogin(
            draft: LoginDraft(
                vaultId: vaultId, itemId: nil, title: title, username: username, password: Data(password.utf8),
                urls: [UrlDraft(url: url, matchEtld1: false, allowHttp: false)], notes: nil, otp: .keep,
                favorite: false))
    }

    func enablePasskeyIdentities(_ core: MosaicCore) throws {
        _ = try core.updateSettings(
            update: SettingsUpdate(
                autoLockBackgroundSeconds: nil, idleLockMinutes: nil, clipboardClearSeconds: nil,
                quickUnlockMaxAgeDays: nil, passkeyIdentities: true, passwordIdentities: nil,
                reauthPassword: Data(Self.password.utf8)))
    }
}
