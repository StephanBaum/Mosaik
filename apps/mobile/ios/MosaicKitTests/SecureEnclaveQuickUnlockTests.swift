import LocalAuthentication
import Security
import XCTest

@testable import MosaicKit

/// The simulator has no Secure Enclave, so these tests check the pure parts: the access control, the tag, the
/// key attributes and the error mapping.
final class SecureEnclaveQuickUnlockTests: XCTestCase {
    func testAccessControlFlagsAndTag() throws {
        let flags = SecureEnclaveQuickUnlock.accessControlFlags
        XCTAssertTrue(flags.contains(.privateKeyUsage))
        XCTAssertTrue(flags.contains(.biometryCurrentSet))
        // Biometry only: no passcode fallback and no "any enrolled finger" class.
        XCTAssertFalse(flags.contains(.devicePasscode))
        XCTAssertFalse(flags.contains(.userPresence))
        XCTAssertFalse(flags.contains(.biometryAny))
        XCTAssertFalse(flags.contains(.or))

        let hex = String(repeating: "ab", count: 16)
        let tag = SecureEnclaveQuickUnlock.tag(prefix: "io.github.stephanbaum.mosaic.", alias: "quick-unlock." + hex)
        XCTAssertEqual(
            String(decoding: tag, as: UTF8.self), "io.github.stephanbaum.mosaic.quick-unlock." + hex)

        let control = try SecureEnclaveQuickUnlock.makeAccessControl()
        let group = "TEAM.io.github.stephanbaum.mosaic.shared"
        let attributes = SecureEnclaveQuickUnlock.keyAttributes(
            tag: tag, accessGroup: group, accessControl: control)
        XCTAssertEqual(
            attributes[kSecAttrKeyType as String] as? String, kSecAttrKeyTypeECSECPrimeRandom as String)
        XCTAssertEqual(attributes[kSecAttrKeySizeInBits as String] as? Int, 256)
        XCTAssertEqual(
            attributes[kSecAttrTokenID as String] as? String, kSecAttrTokenIDSecureEnclave as String)
        let privateAttributes = try XCTUnwrap(attributes[kSecPrivateKeyAttrs as String] as? [String: Any])
        XCTAssertEqual(privateAttributes[kSecAttrIsPermanent as String] as? Bool, true)
        XCTAssertEqual(privateAttributes[kSecAttrApplicationTag as String] as? Data, tag)
        XCTAssertEqual(privateAttributes[kSecAttrAccessGroup as String] as? String, group)
        XCTAssertNotNil(privateAttributes[kSecAttrAccessControl as String])

        // The ECIES body is 65 + 32 + 16 bytes (CRYPTO_SPEC 6.2).
        XCTAssertEqual(SecureEnclaveQuickUnlock.ciphertextLength, 113)
    }

    func testPlatformErrorsMapToTheOutcomesTheScreenHandles() {
        XCTAssertEqual(SecureEnclaveQuickUnlock.map(laError: LAError(.userCancel)), .cancelled)
        XCTAssertEqual(SecureEnclaveQuickUnlock.map(laError: LAError(.appCancel)), .cancelled)
        XCTAssertEqual(SecureEnclaveQuickUnlock.map(laError: LAError(.biometryLockout)), .lockedOut)
        XCTAssertEqual(SecureEnclaveQuickUnlock.map(laError: LAError(.biometryNotEnrolled)), .unavailable)
        XCTAssertEqual(SecureEnclaveQuickUnlock.map(laError: LAError(.authenticationFailed)), .failed)
        let invalidated = NSError(domain: NSOSStatusErrorDomain, code: Int(errSecAuthFailed))
        XCTAssertEqual(SecureEnclaveQuickUnlock.map(keyError: invalidated), .invalidated)
        XCTAssertEqual(SecureEnclaveQuickUnlock.map(keyError: nil), .failed)
        XCTAssertEqual(ErrorText.code(of: QuickUnlockKeyError.invalidated), "quickUnlockInvalidated")
    }

    func testTheSimulatorOffersNoQuickUnlock() {
        #if targetEnvironment(simulator)
            XCTAssertFalse(SecureEnclaveQuickUnlock(tagPrefix: "x.", accessGroup: nil).isAvailable)
        #endif
    }
}
