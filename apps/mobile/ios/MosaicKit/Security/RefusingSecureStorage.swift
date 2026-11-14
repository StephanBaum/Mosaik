import Foundation

/// The `SecureStorage` of the AutoFill extension: it refuses every call. The extension never needs the Secret
/// Key or the session token (quick unlock opens the vault from the Secure Enclave key), so it is given no
/// way to read them. `contains` throws like the rest; the core reads a failed `contains` as "not stored", the
/// same answer the app-private group gave the extension before.
public final class RefusingSecureStorage: SecureStorage {
    public init() {}

    public func get(name: String) throws -> Data? { throw StorageError.Failed }
    public func contains(name: String) throws -> Bool { throw StorageError.Failed }
    public func put(name: String, value: Data) throws { throw StorageError.Failed }
    public func delete(name: String) throws { throw StorageError.Failed }
}
