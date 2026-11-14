import Foundation

#if DEBUG
    /// A local account for the UI tests (`MosaicUITests`). Honored only in a Debug build, and the test hooks
    /// exist only in a library built with `--features test-hooks`: a release build has neither. The vault is
    /// locked at launch, so the tests type the password.
    public enum UITestAccount {
        public static let launchArgument = "-mosaic-test-account"
        public static let email = "ui-test@example.com"
        public static let password = "correct horse battery staple 42"

        public static var isRequested: Bool {
            ProcessInfo.processInfo.arguments.contains(launchArgument)
        }

        /// Opens the core of the app on test storage (the simulator has no passcode, so the Keychain class that
        /// needs one cannot be used) and creates the account with a few items the first time.
        public static func openApp(identity: AppIdentity) throws -> CoreHolder {
            guard let directory = CoreFactory.storeDirectory(identity: identity) else {
                throw CodedError("storeUnavailable")
            }
            let storage = KeychainSecureStorage(
                service: identity.secureStoreService, accessGroup: nil,
                accessibility: .unlockedThisDeviceOnlyForTests)
            let core = try MosaicCore.open(
                config: CoreFactory.config(storeDirectory: directory, context: .app), storage: storage)
            if case .noAccount = core.state() {
                try seed(core)
            }
            return CoreHolder(core: core)
        }

        private static func seed(_ core: MosaicCore) throws {
            let hooks = MosaicTestHooks(core: core)
            let account = try hooks.createLocalAccount(email: email, password: Data(password.utf8))
            account.secretKey.wipe()
            guard let vault = try core.vaults().first(where: { $0.available })?.vaultId else {
                core.lock()
                return
            }
            let preview = try? core.otpFromText(
                text: Data("otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example".utf8))
            var otp = OtpEdit.keep
            if let preview {
                otp = OtpEdit.setPending(id: preview.pendingId)
            }
            _ = try core.saveLogin(
                draft: LoginDraft(
                    vaultId: vault, itemId: nil, title: "Example", username: "alice@example.com",
                    password: Data("tr0ub4dor&3-horse-staple".utf8),
                    urls: [UrlDraft(url: "https://example.com", matchEtld1: false, allowHttp: false)],
                    notes: nil, otp: otp, favorite: true))
            _ = try core.saveLogin(
                draft: LoginDraft(
                    vaultId: vault, itemId: nil, title: "Mosaic Sync", username: "bob@example.org",
                    password: Data("a-different-long-password-7".utf8),
                    urls: [UrlDraft(url: "https://sync.example.org", matchEtld1: false, allowHttp: false)],
                    notes: nil, otp: OtpEdit.keep, favorite: false))
            _ = try core.saveNote(
                draft: NoteDraft(
                    vaultId: vault, itemId: nil, title: "Wi-Fi at home",
                    notes: Data("Network: Home\nKeep this note private.".utf8), favorite: false))
            core.lock()
        }
    }
#endif
