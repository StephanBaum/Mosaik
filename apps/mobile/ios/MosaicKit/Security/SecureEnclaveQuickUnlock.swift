import Foundation
import LocalAuthentication
import Security

/// Why a quick-unlock key operation did not give a result.
public enum QuickUnlockKeyError: Error, Sendable, Equatable {
    /// No Secure Enclave, or no biometry set up.
    case unavailable
    /// The user cancelled the prompt.
    case cancelled
    /// Too many failed attempts: the system asks for the passcode before biometry works again.
    case lockedOut
    /// The key is gone or unusable (the enrolled faces or fingers changed). The vault asks for the password.
    case invalidated
    case failed
}

/// The platform half of quick unlock: a key of the Secure Enclave and the biometric prompt.
public protocol QuickUnlockKeys: Sendable {
    /// Whether this device can enrol a key (a Secure Enclave and enrolled biometry).
    var isAvailable: Bool { get }
    /// Creates the key `alias` and returns `secret` (the 32 bytes of `W`) encrypted to its public key.
    func enroll(alias: String, secret: Data) throws -> Data
    /// Opens `ciphertext` with the key `alias` after a biometric prompt.
    func decrypt(alias: String, ciphertext: Data, reason: String, cancelTitle: String) async throws -> Data
    /// Deletes the key `alias`. A missing key is not an error.
    func deleteKey(alias: String) throws
}

/// Quick unlock with a Secure Enclave P-256 key (CRYPTO_SPEC 6.2 iOS row, ADR-0090). The reference is the
/// desktop's Touch ID code (ADR-0028):
///
/// - the key is `kSecAttrTokenIDSecureEnclave`, permanent, in the shared keychain group so the AutoFill
///   extension uses the same key, with `.privateKeyUsage` and `.biometryCurrentSet` access control (biometry
///   only, no passcode fallback; a change of enrolled faces or fingers invalidates it);
/// - `W` is encrypted to the public key with `eciesEncryptionCofactorX963SHA256AESGCM` (113 bytes), which
///   needs no prompt;
/// - opening it needs the private key, so one prompt per unlock: an `LAContext` evaluates
///   `.deviceOwnerAuthenticationWithBiometrics` and goes to the keychain as `kSecUseAuthenticationContext`.
public struct SecureEnclaveQuickUnlock: QuickUnlockKeys {
    public static let accessControlFlags: SecAccessControlCreateFlags = [.privateKeyUsage, .biometryCurrentSet]
    /// 65-byte ephemeral key, 32 bytes of ciphertext, 16 bytes of GCM tag.
    public static let ciphertextLength = 113

    private let tagPrefix: String
    private let accessGroup: String?

    public init(identity: AppIdentity) {
        self.tagPrefix = identity.quickUnlockTagPrefix
        self.accessGroup = identity.sharedKeychainGroup
    }

    init(tagPrefix: String, accessGroup: String?) {
        self.tagPrefix = tagPrefix
        self.accessGroup = accessGroup
    }

    /// `kSecAttrApplicationTag`: `<bundle id>.quick-unlock.<hex nonce>` (the prefix and the core's alias).
    static func tag(prefix: String, alias: String) -> Data {
        Data((prefix + alias).utf8)
    }

    /// The attributes of the key to create. A pure builder; the simulator has no Secure Enclave.
    static func keyAttributes(tag: Data, accessGroup: String?, accessControl: SecAccessControl) -> [String: Any] {
        var privateAttributes: [String: Any] = [
            kSecAttrIsPermanent as String: true,
            kSecAttrApplicationTag as String: tag,
            kSecAttrAccessControl as String: accessControl,
        ]
        if let accessGroup {
            privateAttributes[kSecAttrAccessGroup as String] = accessGroup
        }
        return [
            kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom,
            kSecAttrKeySizeInBits as String: 256,
            kSecAttrTokenID as String: kSecAttrTokenIDSecureEnclave,
            kSecPrivateKeyAttrs as String: privateAttributes,
        ]
    }

    static func makeAccessControl() throws -> SecAccessControl {
        var error: Unmanaged<CFError>?
        guard
            let control = SecAccessControlCreateWithFlags(
                nil, kSecAttrAccessibleWhenUnlockedThisDeviceOnly, accessControlFlags, &error)
        else {
            error?.release()
            throw QuickUnlockKeyError.unavailable
        }
        return control
    }

    /// The query for the private key with this tag.
    static func keyQuery(tag: Data, accessGroup: String?) -> [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassKey,
            kSecAttrApplicationTag as String: tag,
            kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom,
        ]
        if let accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        return query
    }

    public var isAvailable: Bool {
        #if targetEnvironment(simulator)
            return false
        #else
            var error: NSError?
            return LAContext().canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error)
        #endif
    }

    public func enroll(alias: String, secret: Data) throws -> Data {
        let tag = Self.tag(prefix: tagPrefix, alias: alias)
        let control = try Self.makeAccessControl()
        let attributes = Self.keyAttributes(tag: tag, accessGroup: accessGroup, accessControl: control)
        var error: Unmanaged<CFError>?
        guard let privateKey = SecKeyCreateRandomKey(attributes as CFDictionary, &error) else {
            error?.release()
            throw QuickUnlockKeyError.unavailable
        }
        guard let publicKey = SecKeyCopyPublicKey(privateKey) else {
            try? deleteKey(alias: alias)
            throw QuickUnlockKeyError.failed
        }
        guard
            let encrypted = SecKeyCreateEncryptedData(
                publicKey, .eciesEncryptionCofactorX963SHA256AESGCM, secret as CFData, &error)
        else {
            error?.release()
            try? deleteKey(alias: alias)
            throw QuickUnlockKeyError.failed
        }
        let ciphertext = encrypted as Data
        guard ciphertext.count == Self.ciphertextLength else {
            try? deleteKey(alias: alias)
            throw QuickUnlockKeyError.failed
        }
        return ciphertext
    }

    public func decrypt(
        alias: String, ciphertext: Data, reason: String, cancelTitle: String
    ) async throws -> Data {
        let tag = Self.tag(prefix: tagPrefix, alias: alias)
        let context = LAContext()
        // The authentication is over when this call ends, so a retained context cannot be reused.
        defer { context.invalidate() }
        // Biometry only: an empty fallback title hides "Enter Password".
        context.localizedFallbackTitle = ""
        context.localizedCancelTitle = cancelTitle
        do {
            _ = try await context.evaluatePolicy(
                .deviceOwnerAuthenticationWithBiometrics, localizedReason: reason)
        } catch {
            throw Self.map(laError: error)
        }
        var query = Self.keyQuery(tag: tag, accessGroup: accessGroup)
        query[kSecReturnRef as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        query[kSecUseAuthenticationContext as String] = context
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess, let item, CFGetTypeID(item) == SecKeyGetTypeID() else {
            throw status == errSecItemNotFound ? QuickUnlockKeyError.invalidated : QuickUnlockKeyError.failed
        }
        let key = unsafeDowncast(item, to: SecKey.self)
        var error: Unmanaged<CFError>?
        guard
            let plain = SecKeyCreateDecryptedData(
                key, .eciesEncryptionCofactorX963SHA256AESGCM, ciphertext as CFData, &error)
        else {
            let failure = error?.takeRetainedValue()
            throw Self.map(keyError: failure.map { $0 as Error })
        }
        return plain as Data
    }

    public func deleteKey(alias: String) throws {
        let tag = Self.tag(prefix: tagPrefix, alias: alias)
        let status = SecItemDelete(Self.keyQuery(tag: tag, accessGroup: accessGroup) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else { throw QuickUnlockKeyError.failed }
    }

    static func map(laError error: Error) -> QuickUnlockKeyError {
        guard let la = error as? LAError else { return .failed }
        switch la.code {
        case .userCancel, .systemCancel, .appCancel:
            return .cancelled
        case .biometryLockout:
            return .lockedOut
        case .biometryNotAvailable, .biometryNotEnrolled:
            return .unavailable
        default:
            return .failed
        }
    }

    static func map(keyError error: Error?) -> QuickUnlockKeyError {
        guard let error else { return .failed }
        let ns = error as NSError
        if ns.domain == LAError.errorDomain {
            return map(laError: error)
        }
        switch OSStatus(truncatingIfNeeded: ns.code) {
        case errSecUserCanceled:
            return .cancelled
        // errSecAuthFailed can also come from a transient failure. It still maps to `invalidated`: that
        // fails safe (the user enters the master password and enrols again), at the cost of availability.
        case errSecAuthFailed, errSecItemNotFound:
            return .invalidated
        default:
            return .failed
        }
    }
}
