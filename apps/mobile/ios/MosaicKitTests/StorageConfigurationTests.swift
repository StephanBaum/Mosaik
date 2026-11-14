import XCTest

@testable import MosaicKit

/// G2 Info items: a device build without its keychain group fails closed, and the extension's secure store
/// refuses every call.
final class StorageConfigurationTests: XCTestCase {
    func testMissingKeychainGroupRefusesToOpenWhenRequired() {
        let missing = AppIdentity(bundleID: "io.example.app", keychainGroup: nil, sharedKeychainGroup: nil)
        XCTAssertThrowsError(try missing.appSecureStorage(requireGroup: true)) { error in
            XCTAssertEqual(TestSupport.code(error), "storeUnavailable")
        }
        XCTAssertNoThrow(try missing.appSecureStorage(requireGroup: false))
        let named = AppIdentity(bundleID: "io.example.app", keychainGroup: "TEAM.io.example.app", sharedKeychainGroup: nil)
        XCTAssertNoThrow(try named.appSecureStorage(requireGroup: true))
    }

    func testTheGroupIsOnlyRequiredOnADeviceBuildOutsideDebug() {
        // The tests run in Debug on the simulator.
        XCTAssertFalse(AppIdentity.requiresKeychainGroup)
    }

    func testTheExtensionStorageRefusesEverything() {
        let storage = RefusingSecureStorage()
        XCTAssertThrowsError(try storage.get(name: "secret-key/00"))
        XCTAssertThrowsError(try storage.put(name: "secret-key/00", value: Data([1])))
        XCTAssertThrowsError(try storage.delete(name: "secret-key/00"))
        // The core reads a failed `contains` as "not stored" (`unwrap_or(false)` in stored_flags).
        XCTAssertThrowsError(try storage.contains(name: "secret-key/00"))
    }

    func testAnExtensionCoreOpensOnTheRefusingStorage() throws {
        let directory = try TestSupport.storeDirectory(for: self)
        let core = try MosaicCore.open(
            config: CoreFactory.config(storeDirectory: directory, context: .iosAutofillExtension),
            storage: RefusingSecureStorage())
        guard case .noAccount = core.state() else {
            return XCTFail("a fresh store has no account, got \(core.state())")
        }
        core.closeStore()
    }

    func testProtectionIsAppliedToAnExistingDirectoryToo() throws {
        let directory = try TestSupport.storeDirectory(for: self).appendingPathComponent("existing")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        XCTAssertNotNil(CoreFactory.prepare(directory, fileManager: .default))
    }
}
