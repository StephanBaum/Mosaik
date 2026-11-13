import Foundation

/// A failure that has an error code of the core's catalog but no `MobileError` of its own, for example the
/// memory preflight. `ErrorText` maps it to the same strings as the core's errors.
public struct CodedError: Error, Sendable, Equatable {
    public let code: String

    public init(_ code: String) {
        self.code = code
    }
}

/// Opens the core for the app (CRYPTO_SPEC 6.9, spec 3.8).
public enum CoreFactory {
    /// `<app group>/Library/Application Support/Mosaic`, created with complete file protection and excluded
    /// from backups. Nil when the App Group container is missing; on a device that is an error, in the
    /// unsigned simulator builds of CI it falls back to the app's own Application Support directory.
    public static func storeDirectory(identity: AppIdentity, fileManager: FileManager = .default) -> URL? {
        let group = fileManager.containerURL(forSecurityApplicationGroupIdentifier: identity.appGroup)
        #if targetEnvironment(simulator)
            let base = group ?? fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
        #else
            let base = group
        #endif
        guard let base else { return nil }
        return prepare(
            base.appendingPathComponent("Library", isDirectory: true)
                .appendingPathComponent("Application Support", isDirectory: true)
                .appendingPathComponent("Mosaic", isDirectory: true),
            fileManager: fileManager)
    }

    /// Creates `directory` (complete protection) and marks it excluded from backups. A directory that exists
    /// already (an older build, or the system) gets the protection class too.
    static func prepare(_ directory: URL, fileManager: FileManager) -> URL? {
        do {
            try fileManager.createDirectory(
                at: directory, withIntermediateDirectories: true,
                attributes: [.protectionKey: FileProtectionType.complete])
            #if targetEnvironment(simulator)
                // The simulator has no data protection: it does not matter there, so a refusal is ignored.
                try? fileManager.setAttributes(
                    [.protectionKey: FileProtectionType.complete], ofItemAtPath: directory.path)
            #else
                try fileManager.setAttributes(
                    [.protectionKey: FileProtectionType.complete], ofItemAtPath: directory.path)
            #endif
            var url = directory
            var values = URLResourceValues()
            values.isExcludedFromBackup = true
            try url.setResourceValues(values)
            return url
        } catch {
            return nil
        }
    }

    /// The configuration of a core in this process.
    public static func config(storeDirectory: URL, context: CoreContext, bootId: Data = BootId.current()) -> CoreConfig {
        CoreConfig(
            storeDir: storeDirectory.path, platform: .ios, context: context, bootId: bootId,
            trustAnchorsDer: [])
    }

    /// Opens the core of the app process: the app context, the Keychain secure store and this boot's ID.
    public static func openApp(identity: AppIdentity = .from()) throws -> CoreHolder {
        guard let directory = storeDirectory(identity: identity) else {
            throw CodedError("storeUnavailable")
        }
        let core = try MosaicCore.open(
            config: config(storeDirectory: directory, context: .app), storage: identity.appSecureStorage())
        return CoreHolder(core: core)
    }

    /// Opens the core for one request of the AutoFill extension: the extension context (no password unlock, no
    /// sync, no changes), a secure store that refuses every call (it never needs the Secret Key or the token) and
    /// this boot's ID. The store is open only while the request
    /// runs; `ProviderSession.finish()` closes it.
    public static func openExtension(identity: AppIdentity = .from()) throws -> CoreHolder {
        guard let directory = storeDirectory(identity: identity) else {
            throw CodedError("storeUnavailable")
        }
        let core = try MosaicCore.open(
            config: config(storeDirectory: directory, context: .iosAutofillExtension),
            storage: RefusingSecureStorage())
        return CoreHolder(core: core)
    }
}
