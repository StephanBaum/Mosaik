import Foundation

/// The identifiers of the installed app (OD-M10-1): bundle ID, app group and keychain groups.
///
/// The values come from the `Info.plist` of the app and the extension (`MosaicBundleID`,
/// `MosaicKeychainGroup`, `MosaicSharedKeychainGroup`, all set from the build settings), so the app and
/// the extension agree on them and a different owner changes them in one place.
public struct AppIdentity: Sendable, Equatable {
    public let bundleID: String
    /// `<team prefix><bundle id>`: the app-private group of the secure store. Nil: the app's default group.
    public let keychainGroup: String?
    /// `<team prefix><bundle id>.shared`: the group of the quick-unlock key, shared with the extension.
    public let sharedKeychainGroup: String?

    public init(bundleID: String, keychainGroup: String?, sharedKeychainGroup: String?) {
        self.bundleID = bundleID
        self.keychainGroup = keychainGroup
        self.sharedKeychainGroup = sharedKeychainGroup
    }

    /// The App Group that holds the store (`group.<bundle id>`).
    public var appGroup: String { "group." + bundleID }

    /// `kSecAttrService` of the secure store (CRYPTO_SPEC 6.9.1).
    public var secureStoreService: String { bundleID + ".secure-store" }

    /// The prefix of the quick-unlock key tag; the core's key alias follows (`quick-unlock.<hex>`).
    public var quickUnlockTagPrefix: String { bundleID + "." }

    /// Whether a missing keychain group is a fatal configuration error: a device build outside Debug. The
    /// simulator and Debug builds are unsigned and use the default group.
    public static var requiresKeychainGroup: Bool {
        #if targetEnvironment(simulator) || DEBUG
            return false
        #else
            return true
        #endif
    }

    /// The secure store of the app. A device build without its keychain group refuses to open (a build
    /// configuration error), rather than store the Secret Key in an unnamed group.
    public func appSecureStorage(requireGroup: Bool = AppIdentity.requiresKeychainGroup) throws -> any SecureStorage {
        if requireGroup && keychainGroup == nil {
            throw CodedError("storeUnavailable")
        }
        return KeychainSecureStorage(identity: self)
    }

    /// Reads the identity from the `Info.plist` of `bundle`.
    ///
    /// Simulator builds are unsigned in CI and have no team prefix or keychain entitlement, so they use the
    /// app's default keychain group instead of an explicit one. A device build always names its groups.
    public static func from(bundle: Bundle = .main) -> AppIdentity {
        let info = bundle.infoDictionary ?? [:]
        let bundleID = (info["MosaicBundleID"] as? String).flatMap { $0.isEmpty ? nil : $0 }
            ?? bundle.bundleIdentifier
            ?? "io.github.stephanbaum.mosaic"
        #if targetEnvironment(simulator)
            return AppIdentity(bundleID: bundleID, keychainGroup: nil, sharedKeychainGroup: nil)
        #else
            let group = (info["MosaicKeychainGroup"] as? String).flatMap { $0.isEmpty ? nil : $0 }
            let shared = (info["MosaicSharedKeychainGroup"] as? String).flatMap { $0.isEmpty ? nil : $0 }
            return AppIdentity(bundleID: bundleID, keychainGroup: group, sharedKeychainGroup: shared)
        #endif
    }
}
