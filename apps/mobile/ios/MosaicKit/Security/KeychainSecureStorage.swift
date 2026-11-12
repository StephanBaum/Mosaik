import Foundation
import Security

/// `SecureStorage` on the iOS Keychain (CRYPTO_SPEC 6.9.1, ADR-0090). Rust decides the names and the
/// lifetimes; this type stores one generic password per name.
///
/// Items are `kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly` (no secret is stored on a device without a
/// passcode, ADR-0090 amendment) and never synchronizable, so they stay out of iCloud Keychain and out of
/// backups. The access group is the app-private group: the AutoFill extension is
/// not in it and never needs the Secret Key or the session token.
public final class KeychainSecureStorage: SecureStorage {
    /// The accessibility class of new items. Only tests on the simulator (which has no passcode) use the
    /// other one.
    enum Accessibility: Sendable {
        case passcodeSetThisDeviceOnly
        case unlockedThisDeviceOnlyForTests

        var value: CFString {
            switch self {
            case .passcodeSetThisDeviceOnly: kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly
            case .unlockedThisDeviceOnlyForTests: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
            }
        }
    }

    private let service: String
    private let accessGroup: String?
    private let accessibility: Accessibility

    public init(service: String, accessGroup: String?) {
        self.service = service
        self.accessGroup = accessGroup
        self.accessibility = .passcodeSetThisDeviceOnly
    }

    init(service: String, accessGroup: String?, accessibility: Accessibility) {
        self.service = service
        self.accessGroup = accessGroup
        self.accessibility = accessibility
    }

    public convenience init(identity: AppIdentity) {
        self.init(service: identity.secureStoreService, accessGroup: identity.keychainGroup)
    }

    /// The query that names one item. Also the base of every call.
    static func itemQuery(service: String, accessGroup: String?, name: String) -> [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: name,
            kSecAttrSynchronizable as String: false,
        ]
        if let accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        return query
    }

    /// The attributes of a new item: the query, the value, and the accessibility class.
    static func newItemAttributes(
        service: String, accessGroup: String?, name: String, value: Data,
        accessibility: Accessibility = .passcodeSetThisDeviceOnly
    ) -> [String: Any] {
        var attributes = itemQuery(service: service, accessGroup: accessGroup, name: name)
        attributes[kSecValueData as String] = value
        attributes[kSecAttrAccessible as String] = accessibility.value
        return attributes
    }

    public func get(name: String) throws -> Data? {
        var query = Self.itemQuery(service: service, accessGroup: accessGroup, name: name)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        switch status {
        case errSecSuccess:
            guard let data = result as? Data else { throw StorageError.Failed }
            return data
        case errSecItemNotFound:
            return nil
        default:
            throw StorageError.Failed
        }
    }

    /// Presence only: the attributes are read, never the value (G1-2).
    public func contains(name: String) throws -> Bool {
        var query = Self.itemQuery(service: service, accessGroup: accessGroup, name: name)
        query[kSecReturnAttributes as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        switch status {
        case errSecSuccess:
            return true
        case errSecItemNotFound:
            return false
        default:
            throw StorageError.Failed
        }
    }

    public func put(name: String, value: Data) throws {
        let query = Self.itemQuery(service: service, accessGroup: accessGroup, name: name)
        var status = SecItemUpdate(query as CFDictionary, [kSecValueData as String: value] as CFDictionary)
        if status == errSecItemNotFound {
            let attributes = Self.newItemAttributes(
                service: service, accessGroup: accessGroup, name: name, value: value,
                accessibility: accessibility)
            status = SecItemAdd(attributes as CFDictionary, nil)
        }
        guard status == errSecSuccess else { throw StorageError.Failed }
    }

    public func delete(name: String) throws {
        let query = Self.itemQuery(service: service, accessGroup: accessGroup, name: name)
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else { throw StorageError.Failed }
    }
}
