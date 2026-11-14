import Security
import XCTest

@testable import MosaicKit

final class KeychainSecureStorageTests: XCTestCase {
    func testRoundTripDeleteAccessibilityAndNoSync() throws {
        let service = "io.github.stephanbaum.mosaic.tests.\(UUID().uuidString)"
        let name = "secret-key/00112233445566778899aabbccddeeff"
        // The simulator has no passcode, so the class that needs one cannot be stored there.
        let storage = KeychainSecureStorage(
            service: service, accessGroup: nil, accessibility: .unlockedThisDeviceOnlyForTests)
        addTeardownBlock { try? storage.delete(name: name) }

        XCTAssertFalse(try storage.contains(name: name))
        XCTAssertNil(try storage.get(name: name))

        try storage.put(name: name, value: Data([1, 2, 3]))
        XCTAssertTrue(try storage.contains(name: name))
        XCTAssertEqual(try storage.get(name: name), Data([1, 2, 3]))

        // A second put replaces the value.
        try storage.put(name: name, value: Data([4, 5]))
        XCTAssertEqual(try storage.get(name: name), Data([4, 5]))

        // Another service does not see the item.
        let other = KeychainSecureStorage(service: service + ".other", accessGroup: nil)
        XCTAssertFalse(try other.contains(name: name))

        // The stored item is this-device-only, in the class the storage was given, and not synchronizable.
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: name,
            kSecAttrSynchronizable as String: kSecAttrSynchronizableAny,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var result: CFTypeRef?
        XCTAssertEqual(SecItemCopyMatching(query as CFDictionary, &result), errSecSuccess)
        let attributes = try XCTUnwrap(result as? [String: Any])
        XCTAssertEqual(
            attributes[kSecAttrAccessible as String] as? String,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly as String)
        XCTAssertNotEqual(attributes[kSecAttrSynchronizable as String] as? Bool, true)

        try storage.delete(name: name)
        XCTAssertFalse(try storage.contains(name: name))
        XCTAssertNil(try storage.get(name: name))
        // Deleting a missing item is not an error.
        XCTAssertNoThrow(try storage.delete(name: name))
    }

    func testAttributesNameTheServiceTheGroupAndNoSync() {
        let attributes = KeychainSecureStorage.newItemAttributes(
            service: "svc", accessGroup: "TEAM.group", name: "n", value: Data([9]))
        XCTAssertEqual(attributes[kSecAttrService as String] as? String, "svc")
        XCTAssertEqual(attributes[kSecAttrAccount as String] as? String, "n")
        XCTAssertEqual(attributes[kSecAttrAccessGroup as String] as? String, "TEAM.group")
        XCTAssertEqual(attributes[kSecAttrSynchronizable as String] as? Bool, false)
        // The default needs a passcode (no secret on a device without one).
        XCTAssertEqual(
            attributes[kSecAttrAccessible as String] as? String,
            kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly as String)
        let query = KeychainSecureStorage.itemQuery(service: "svc", accessGroup: nil, name: "n")
        XCTAssertNil(query[kSecAttrAccessGroup as String])
    }
}
