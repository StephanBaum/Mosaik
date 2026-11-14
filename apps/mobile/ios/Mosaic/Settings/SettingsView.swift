import AuthenticationServices
import MosaicKit
import Observation
import SwiftUI

/// The state of the settings tab: the typed master password for the two changes that need it (quick unlock and
/// the password identity store), the busy flag and the last error.
@MainActor
@Observable
final class SettingsModel {
    private(set) var busy = false
    private(set) var errorCode: String?
    var askQuickPassword = false
    var askIdentityPassword = false
    var passwordClearToken = 0
    var passwordFilled = false
    private(set) var autofillEnabled = false
    /// The values the pickers and switches show. They follow the core's settings (adopt) and send a change
    /// back when the user makes one.
    var autoLock = 0
    var idle = 5
    var clipboard = 30
    var passkeyIdentities = false
    var passwordIdentities = false

    let password = SecretBuffer()
    @ObservationIgnored private let app: AppModel
    @ObservationIgnored private let vault: VaultModel

    init(app: AppModel, vault: VaultModel) {
        self.app = app
        self.vault = vault
    }

    var quickEnrolled: Bool {
        if case let .unlocked(_, _, enrolled) = app.session { return enrolled }
        return false
    }

    var biometricAvailable: Bool { app.keys.isAvailable }

    func refreshAutofill() async {
        autofillEnabled = await SystemIdentityStore().isEnabled()
    }

    /// Shows the settings of the core.
    func adopt(_ settings: MobileSettings?) {
        guard let settings else { return }
        autoLock = Int(settings.autoLockBackgroundSeconds)
        idle = Int(settings.idleLockMinutes)
        clipboard = Int(settings.clipboardClearSeconds)
        passkeyIdentities = settings.passkeyIdentities
        // While the master password is asked for, the switch stays where the user put it.
        if !askIdentityPassword {
            passwordIdentities = settings.passwordIdentities
        }
    }

    private var stored: MobileSettings? { vault.settings }

    func autoLockChanged(_ value: Int) async {
        guard let stored, Int(stored.autoLockBackgroundSeconds) != value else { return }
        await change(SettingsOptions.update(autoLock: value))
    }

    func idleChanged(_ value: Int) async {
        guard let stored, Int(stored.idleLockMinutes) != value else { return }
        await change(SettingsOptions.update(idle: value))
    }

    func clipboardChanged(_ value: Int) async {
        guard let stored, Int(stored.clipboardClearSeconds) != value else { return }
        await change(SettingsOptions.update(clipboard: value))
    }

    func passkeyIdentitiesChanged(_ on: Bool) async {
        guard let stored, stored.passkeyIdentities != on else { return }
        await change(SettingsOptions.update(passkeyIdentities: on))
    }

    func change(_ update: SettingsUpdate) async {
        do {
            try await vault.updateSettings(update)
            errorCode = nil
        } catch {
            errorCode = ErrorText.code(of: error)
        }
    }

    // MARK: Quick unlock

    func confirmQuickUnlock() async {
        guard !password.isEmpty else {
            errorCode = "emptyPassword"
            return
        }
        busy = true
        errorCode = nil
        defer { busy = false }
        do {
            try await app.enrollQuickUnlock(password: password)
            cancelPassword()
        } catch {
            errorCode = ErrorText.code(of: error)
            clearPassword()
        }
    }

    func disableQuickUnlock() async {
        busy = true
        defer { busy = false }
        do {
            try await app.disableQuickUnlock()
            errorCode = nil
        } catch {
            errorCode = ErrorText.code(of: error)
        }
    }

    // MARK: Password identity store (OD-M10-7)

    /// Turning it on needs the master password; turning it off does not.
    func passwordIdentitiesChanged(_ on: Bool) async {
        guard let stored else { return }
        if on {
            if !stored.passwordIdentities {
                askIdentityPassword = true
            }
        } else if askIdentityPassword {
            cancelPassword()
        } else if stored.passwordIdentities {
            await change(SettingsOptions.update(passwordIdentities: false))
        }
    }

    func confirmIdentityPassword() async {
        guard !password.isEmpty else {
            errorCode = "emptyPassword"
            return
        }
        busy = true
        errorCode = nil
        defer { busy = false }
        var typed = password.copyOut()
        // The update holds the same bytes as `typed`; it lives only in this inner scope, so `typed` is the
        // last reference when it is wiped (a best effort: the core call has its own copy until it returns).
        defer { typed.wipe() }
        do {
            do {
                let update = SettingsOptions.update(passwordIdentities: true, reauthPassword: typed)
                try await vault.updateSettings(update)
            }
            cancelPassword()
        } catch {
            errorCode = ErrorText.code(of: error)
            clearPassword()
        }
    }

    func cancelPassword() {
        askQuickPassword = false
        askIdentityPassword = false
        clearPassword()
        passwordIdentities = vault.settings?.passwordIdentities ?? false
    }

    private func clearPassword() {
        password.wipe()
        passwordFilled = false
        passwordClearToken += 1
    }

    func sceneLeft() {
        if askQuickPassword || askIdentityPassword {
            cancelPassword()
        }
    }
}

@MainActor
struct SettingsView: View {
    @Environment(\.colorScheme) private var colorScheme
    let app: AppModel
    let vault: VaultModel
    @State private var model: SettingsModel

    init(app: AppModel, vault: VaultModel) {
        self.app = app
        self.vault = vault
        _model = State(initialValue: SettingsModel(app: app, vault: vault))
    }

    var body: some View {
        let colors = colorScheme.mosaic
        ZStack {
            colors.bg.ignoresSafeArea()
            ScrollView {
                VStack(alignment: .leading, spacing: MosaicTokens.Space.s5) {
                    if let code = model.errorCode {
                        ErrorLine(text: ErrorText.message(forCode: code))
                    }
                    securityPanel
                    quickUnlockPanel
                    fillPanel
                    syncPanel
                    aboutPanel
                    Button(L10n.lockNow()) { app.lock() }
                        .buttonStyle(PrimaryButtonStyle())
                        .accessibilityIdentifier("lock-now")
                }
                .padding(MosaicTokens.Space.s6)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .scrollDismissesKeyboard(.interactively)
        }
        .navigationTitle(L10n.settingsTitle())
        .navigationBarTitleDisplayMode(.large)
        .task { await vault.load() }
        .onAppear { model.adopt(vault.settings) }
        .onChange(of: vault.settings) { _, settings in model.adopt(settings) }
        .task(id: app.isSceneActive) {
            if app.isSceneActive {
                await model.refreshAutofill()
            } else {
                model.sceneLeft()
            }
        }
    }

    // MARK: Panels

    private var securityPanel: some View {
        Panel {
            SectionTitle(text: L10n.settingsSecurity())
            if let settings = vault.settings {
                picker(
                    title: L10n.settingsAutoLock(), identifier: "setting-auto-lock", selection: $model.autoLock,
                    options: SettingsOptions.withCurrent(
                        SettingsOptions.autoLockSeconds, current: Int(settings.autoLockBackgroundSeconds)),
                    label: { SettingsOptions.autoLockLabel(seconds: $0) })
                    .onChange(of: model.autoLock) { _, value in Task { await model.autoLockChanged(value) } }
                picker(
                    title: L10n.settingsIdle(), identifier: "setting-idle", selection: $model.idle,
                    options: SettingsOptions.withCurrent(SettingsOptions.idleMinutes, current: Int(settings.idleLockMinutes)),
                    label: { L10n.settingsIdleOption(minutes: $0) })
                    .onChange(of: model.idle) { _, value in Task { await model.idleChanged(value) } }
                picker(
                    title: L10n.settingsClipboard(), identifier: "setting-clipboard", selection: $model.clipboard,
                    options: SettingsOptions.withCurrent(
                        SettingsOptions.clipboardSeconds, current: Int(settings.clipboardClearSeconds)),
                    label: { L10n.settingsClipboardOption(seconds: $0) })
                    .onChange(of: model.clipboard) { _, value in Task { await model.clipboardChanged(value) } }
            }
        }
    }

    private func picker(
        title: String, identifier: String, selection: Binding<Int>, options: [Int], label: @escaping (Int) -> String
    ) -> some View {
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s3) {
            FieldLabel(text: title)
            Picker(title, selection: selection) {
                ForEach(options, id: \.self) { value in
                    Text(label(value)).tag(value)
                }
            }
            .pickerStyle(.menu)
            .frame(minHeight: 44, alignment: .leading)
            .accessibilityIdentifier(identifier)
        }
    }

    private func switchRow(_ text: String, identifier: String, isOn: Binding<Bool>) -> some View {
        Toggle(isOn: isOn) {
            Text(text)
                .font(Font.mosaic(MosaicTokens.FontSize.md))
                .foregroundStyle(colorScheme.mosaic.text)
                .fixedSize(horizontal: false, vertical: true)
        }
        .tint(colorScheme.mosaic.accent)
        .frame(minHeight: 48)
        .accessibilityIdentifier(identifier)
    }

    private var quickUnlockPanel: some View {
        Panel {
            SectionTitle(text: L10n.settingsQuick())
            if model.quickEnrolled {
                BodyText(text: L10n.settingsQuickOn())
                Button(L10n.settingsQuickDisable()) { Task { await model.disableQuickUnlock() } }
                    .buttonStyle(SecondaryButtonStyle())
                    .disabled(model.busy)
                    .accessibilityIdentifier("quick-disable")
            } else {
                BodyText(text: L10n.settingsQuickOff())
                if !model.biometricAvailable {
                    HintText(text: L10n.onboardingQuickUnavailable())
                } else if model.askQuickPassword {
                    passwordPrompt(
                        text: L10n.settingsQuickPassword(), action: L10n.settingsQuickEnable(),
                        onConfirm: { Task { await model.confirmQuickUnlock() } })
                } else {
                    Button(L10n.settingsQuickEnable()) { model.askQuickPassword = true }
                        .buttonStyle(SecondaryButtonStyle())
                        .disabled(model.busy)
                        .accessibilityIdentifier("quick-enable")
                }
            }
        }
    }

    private func passwordPrompt(text: String, action: String, onConfirm: @escaping @MainActor () -> Void) -> some View {
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s3) {
            BodyText(text: text)
            SecretField(
                label: L10n.unlockPasswordLabel(), buffer: model.password, identifier: "settings-password",
                clearToken: model.passwordClearToken, onChange: { empty in model.passwordFilled = !empty },
                onSubmit: onConfirm)
            Button(action, action: onConfirm)
                .buttonStyle(PrimaryButtonStyle())
                .disabled(!model.passwordFilled || model.busy)
                .accessibilityIdentifier("settings-password-confirm")
            Button(L10n.commonCancel()) { model.cancelPassword() }
                .buttonStyle(QuietButtonStyle())
        }
    }

    private var fillPanel: some View {
        Panel {
            SectionTitle(text: L10n.settingsFill())
            BodyText(text: model.autofillEnabled ? L10n.settingsAutofillOn() : L10n.settingsAutofillOff())
                .accessibilityIdentifier("autofill-state")
            if !model.autofillEnabled {
                if #available(iOS 18.0, *) {
                    Button(L10n.settingsAutofillEnable()) {
                        ASSettingsHelper.openCredentialProviderAppSettings { _ in }
                    }
                    .buttonStyle(SecondaryButtonStyle())
                    .accessibilityIdentifier("autofill-enable")
                } else {
                    HintText(text: L10n.onboardingAutofillSteps())
                }
            }
            if vault.settings != nil {
                switchRow(
                    L10n.settingsPasskeyIdentities(), identifier: "setting-passkey-identities",
                    isOn: $model.passkeyIdentities)
                    .onChange(of: model.passkeyIdentities) { _, on in Task { await model.passkeyIdentitiesChanged(on) } }
                HintText(text: L10n.settingsPasskeyIdentitiesHint())
                switchRow(
                    L10n.settingsPasswordIdentities(), identifier: "setting-password-identities",
                    isOn: $model.passwordIdentities)
                    .onChange(of: model.passwordIdentities) { _, on in Task { await model.passwordIdentitiesChanged(on) } }
                HintText(text: L10n.settingsPasswordIdentitiesHint())
                HintText(text: L10n.settingsPasswordIdentitiesLimit())
                if model.askIdentityPassword {
                    passwordPrompt(
                        text: L10n.settingsIdentitiesPassword(), action: L10n.settingsQuickEnable(),
                        onConfirm: { Task { await model.confirmIdentityPassword() } })
                }
            }
        }
    }

    private var syncPanel: some View {
        Panel {
            SectionTitle(text: L10n.settingsSync())
            let text = SyncLine.text(for: vault.overview)
            if !text.isEmpty {
                BodyText(text: text)
            }
            if let pending = vault.overview?.pendingUploads, pending > 0 {
                HintText(text: L10n.syncPending(count: Int(pending)))
            }
            if vault.isRefreshing {
                HintText(text: L10n.syncWorking())
            } else {
                Button(L10n.syncNow()) { Task { await vault.refresh() } }
                    .buttonStyle(SecondaryButtonStyle())
                    .accessibilityIdentifier("sync-now")
            }
        }
    }

    private var aboutPanel: some View {
        Panel {
            SectionTitle(text: L10n.settingsAbout())
            BodyText(text: L10n.settingsVersion(version: Self.appVersion))
            BodyText(text: L10n.aboutPreAudit())
                .accessibilityIdentifier("about-pre-audit")
            BodyText(text: L10n.aboutPrivacy())
            NavigationLink {
                LicencesView()
            } label: {
                Text(L10n.aboutLicences())
                    .font(Font.mosaic(MosaicTokens.FontSize.md, weight: .medium))
                    .foregroundStyle(colorScheme.mosaic.accentText)
                    .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
            }
            .accessibilityIdentifier("about-licences")
        }
    }

    private static var appVersion: String {
        let info = Bundle.main.infoDictionary
        let short = info?["CFBundleShortVersionString"] as? String ?? ""
        return short.isEmpty ? coreVersion() : short
    }
}

/// The licences of the parts Mosaic for iPhone is built from.
struct LicencesView: View {
    @Environment(\.colorScheme) private var colorScheme

    private static let licences: [(String, String)] = [
        ("DM Sans", "SIL Open Font License 1.1"),
        ("IBM Plex Mono", "SIL Open Font License 1.1"),
        ("UniFFI", "Mozilla Public License 2.0"),
    ]

    var body: some View {
        let colors = colorScheme.mosaic
        ZStack {
            colors.bg.ignoresSafeArea()
            ScrollView {
                VStack(alignment: .leading, spacing: MosaicTokens.Space.s5) {
                    BodyText(text: L10n.licencesIntroIos())
                    ForEach(Self.licences, id: \.0) { name, licence in
                        Panel {
                            Text(name)
                                .font(Font.mosaic(MosaicTokens.FontSize.md, weight: .medium))
                                .foregroundStyle(colors.text)
                            HintText(text: licence)
                        }
                    }
                    BodyText(text: L10n.licencesRust())
                }
                .padding(MosaicTokens.Space.s6)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .navigationTitle(L10n.licencesTitle())
        .navigationBarTitleDisplayMode(.inline)
    }
}
