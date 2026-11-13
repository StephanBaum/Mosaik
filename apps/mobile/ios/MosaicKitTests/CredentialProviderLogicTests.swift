import CryptoKit
import XCTest

@testable import MosaicKit

/// The extension's logic (`ProviderSession`) against a real core in the extension context: a new process of
/// the same boot opens the shared store, unlocks with the (fake) Secure Enclave key, and asks the core.
final class CredentialProviderLogicTests: XCTestCase {
    private func domain(_ value: String) -> ServiceIdentifier {
        ServiceIdentifier(kind: .domain, value: value)
    }

    private func code(of error: Error) -> String {
        TestSupport.code(error)
    }

    private func signCount(_ authenticatorData: Data) -> UInt32 {
        authenticatorData[33..<37].reduce(UInt32(0)) { ($0 << 8) | UInt32($1) }
    }

    func testPasswordFlowWithTestVault() async throws {
        let phone = try TestPhone(for: self)
        var github = ItemKey(vaultId: "", itemId: "")
        var other = ItemKey(vaultId: "", itemId: "")
        try phone.seed { core, _ in
            github = try phone.login(
                core, title: "GitHub", username: "octo", password: "gh-secret-pw", url: "https://github.com")
            other = try phone.login(
                core, title: "Example", username: "alice", password: "ex-secret-pw", url: "https://example.com")
        }
        let holder = try phone.extensionHolder()
        let session = ProviderSession(holder: holder, keys: phone.keys)

        let readiness = try await session.begin()
        XCTAssertEqual(readiness, .quickUnlock)
        // Locked: nothing is listed.
        do {
            _ = try await session.list(identifiers: [domain("github.com")])
            XCTFail("a locked vault must not list logins")
        } catch {
            XCTAssertEqual(code(of: error), "locked")
        }

        try await session.unlockQuick(reason: "test", cancelTitle: "Cancel")
        let list = try await session.list(identifiers: [domain("github.com")])
        XCTAssertEqual(list.matches.map(\.key), [github])
        XCTAssertEqual(list.others.map(\.itemId), [other.itemId])
        XCTAssertNil(list.refusalCode)

        let filled = try await session.password(for: github)
        XCTAssertEqual(filled.user, "octo")
        XCTAssertEqual(filled.password, "gh-secret-pw")
        // The core checks the match again: the login of another site is refused for this target.
        do {
            _ = try await session.password(for: other)
            XCTFail("a login saved for another site must not fill")
        } catch {
            XCTAssertEqual(code(of: error), "noMatch")
        }

        // Another site: no match, and the earlier key does not carry over to the new target.
        let foreign = try await session.list(identifiers: [domain("evil.example")])
        XCTAssertTrue(foreign.matches.isEmpty)
        XCTAssertEqual(foreign.others.count, 2)
        do {
            _ = try await session.password(for: github)
            XCTFail("a login must not fill a site it is not saved for")
        } catch {
            XCTAssertEqual(code(of: error), "noMatch")
        }

        // The end of the request: store closed, fill session gone, nothing more to ask.
        session.finish()
        guard case .storeClosed = holder.state() else {
            return XCTFail("expected a closed store, got \(holder.state())")
        }
        do {
            _ = try await session.password(for: github)
            XCTFail("a finished request must not fill")
        } catch {
            XCTAssertEqual(code(of: error), "notFound")
        }
        session.finish()

        // The next request starts again from a locked vault.
        let again = try await session.begin()
        XCTAssertEqual(again, .quickUnlock)
        session.finish()
    }

    func testRequestNeedsTheAppWithoutEnrolmentAndWithoutAccount() async throws {
        let notEnrolled = try TestPhone(for: self)
        try notEnrolled.seed(enrolQuickUnlock: false)
        let session = ProviderSession(holder: try notEnrolled.extensionHolder(), keys: notEnrolled.keys)
        let needsApp = try await session.begin()
        XCTAssertEqual(needsApp, .needsApp)
        session.finish()

        let empty = try TestPhone(for: self)
        let fresh = ProviderSession(holder: try empty.extensionHolder(), keys: empty.keys)
        let noAccount = try await fresh.begin()
        XCTAssertEqual(noAccount, .noAccount)
        fresh.finish()
    }

    func testBusyStoreIsReportedNotOpened() async throws {
        let phone = try TestPhone(for: self)
        try phone.seed()
        // The app holds the store.
        let app = try phone.openCore(.app)
        let session = ProviderSession(holder: try phone.extensionHolder(), keys: phone.keys)
        let busy = try await session.begin()
        XCTAssertEqual(busy, .busy)
        app.closeStore()
        let ready = try await session.begin()
        XCTAssertEqual(ready, .quickUnlock)
        session.finish()
    }

    func testPasskeyAssertAndRegisterHashMode() async throws {
        let phone = try TestPhone(for: self)
        try phone.seed { core, _ in try phone.enablePasskeyIdentities(core) }
        let session = ProviderSession(holder: try phone.extensionHolder(), keys: phone.keys)
        let hash = Data((0..<32).map { UInt8($0) })
        let userHandle = Data([1, 2, 3, 4])

        // Without the unlock of this request there is no user verification: refused.
        _ = try await session.begin()
        do {
            _ = try await session.registerPasskey(
                rpId: "example.com", userName: "alice", userHandle: userHandle, clientDataHash: hash,
                algorithms: [-7], excludedCredentialIds: [], vaultId: phone.vaultId)
            XCTFail("a passkey ceremony needs user verification")
        } catch {
            XCTAssertEqual(code(of: error), "reauthRequired")
        }

        try await session.unlockQuick(reason: "test", cancelTitle: "Cancel")
        let vaults = try await session.availableVaults()
        XCTAssertEqual(vaults.map(\.vaultId), [phone.vaultId])

        let made = try await session.registerPasskey(
            rpId: "example.com", userName: "alice", userHandle: userHandle, clientDataHash: hash,
            algorithms: [-7], excludedCredentialIds: [], vaultId: phone.vaultId)
        XCTAssertFalse(made.registration.credentialId.isEmpty)
        XCTAssertFalse(made.registration.attestationObject.isEmpty)
        // The identity for the system store is there because the app turned passkey identities on.
        let identity = try XCTUnwrap(made.identity)
        XCTAssertEqual(identity.rpId, "example.com")
        XCTAssertEqual(identity.credentialId, made.registration.credentialId)
        XCTAssertEqual(identity.userHandle, userHandle)
        XCTAssertEqual(
            IdentityStoreSync.parseRecordIdentifier(IdentityStoreSync.recordIdentifier(identity.record)),
            identity.record)

        // The same credential in the exclude list is refused.
        do {
            _ = try await session.registerPasskey(
                rpId: "example.com", userName: "alice", userHandle: userHandle, clientDataHash: hash,
                algorithms: [-7], excludedCredentialIds: [made.registration.credentialId], vaultId: phone.vaultId)
            XCTFail("an excluded credential must be refused")
        } catch {
            XCTAssertEqual(code(of: error), "passkeyExcluded")
        }
        // An algorithm the core does not support is refused before anything is stored.
        do {
            _ = try await session.registerPasskey(
                rpId: "example.com", userName: "bob", userHandle: userHandle, clientDataHash: hash,
                algorithms: [-257], excludedCredentialIds: [], vaultId: phone.vaultId)
            XCTFail("RS256 is not supported")
        } catch {
            XCTAssertEqual(code(of: error), "passkeyUnsupportedAlgorithm")
        }

        let candidates = try await session.passkeyCandidates(rpId: "example.com", allow: [])
        XCTAssertEqual(candidates.map(\.key), [identity.record])
        let elsewhere = try await session.passkeyCandidates(rpId: "other.example", allow: [])
        XCTAssertTrue(elsewhere.isEmpty)

        // Hash mode: the platform's hash is signed over; UP and UV are set.
        let first = try await session.assertPasskey(
            rpId: "example.com", clientDataHash: hash, credentialId: made.registration.credentialId)
        XCTAssertEqual(first.credentialId, made.registration.credentialId)
        XCTAssertEqual(first.userHandle, userHandle)
        XCTAssertFalse(first.signature.isEmpty)
        XCTAssertGreaterThanOrEqual(first.authenticatorData.count, 37)
        XCTAssertEqual(first.authenticatorData.prefix(32), Data(SHA256.hash(data: Data("example.com".utf8))))
        XCTAssertEqual(first.authenticatorData[32] & 0x05, 0x05, "user present and user verified")
        let second = try await session.assertPasskey(
            rpId: "example.com", clientDataHash: hash, candidate: identity.record)
        // The count of a synced passkey may stay 0 (CRYPTO_SPEC 16); it never goes down.
        XCTAssertGreaterThanOrEqual(signCount(second.authenticatorData), signCount(first.authenticatorData))

        // The wrong relying party finds no passkey.
        do {
            _ = try await session.assertPasskey(
                rpId: "other.example", clientDataHash: hash, credentialId: made.registration.credentialId)
            XCTFail("a passkey must not answer another relying party")
        } catch {
            XCTAssertEqual(code(of: error), "passkeyNotFound")
        }
        // A hash that is not 32 bytes is refused.
        do {
            _ = try await session.assertPasskey(
                rpId: "example.com", clientDataHash: Data([1, 2, 3]), credentialId: made.registration.credentialId)
            XCTFail("the client data hash is 32 bytes")
        } catch {
            XCTAssertEqual(code(of: error), "invalidInput")
        }

        // After the request there is no verification left to sign with.
        session.finish()
        do {
            _ = try await session.assertPasskey(
                rpId: "example.com", clientDataHash: hash, credentialId: made.registration.credentialId)
            XCTFail("a finished request must not sign")
        } catch {
            XCTAssertEqual(code(of: error), "reauthRequired")
        }
    }

    func testPasskeyIdentityIsLeftOutWhenTheSettingIsOff() async throws {
        let phone = try TestPhone(for: self)
        try phone.seed()
        let session = ProviderSession(holder: try phone.extensionHolder(), keys: phone.keys)
        _ = try await session.begin()
        try await session.unlockQuick(reason: "test", cancelTitle: "Cancel")
        let made = try await session.registerPasskey(
            rpId: "example.com", userName: "alice", userHandle: Data([7]),
            clientDataHash: Data(repeating: 1, count: 32), algorithms: [-7], excludedCredentialIds: [],
            vaultId: phone.vaultId)
        XCTAssertNil(made.identity)
        session.finish()
    }
}
