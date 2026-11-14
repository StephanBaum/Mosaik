import MosaicKit
import Observation
import SwiftUI

/// The state of the unlock screen. The core decides what is possible (`SessionState.locked`); this model
/// runs the typed password and the quick-unlock prompt and shows what went wrong.
@MainActor
@Observable
final class UnlockModel {
    var enrollQuickUnlock = true
    var passwordClearToken = 0
    var secretKeyClearToken = 0
    var passwordFilled = false
    var secretKeyFilled = false
    /// The Secret Key is shown as typed (never while the screen is captured); hidden again in the background.
    var showSecretKey = false
    private(set) var errorMessage: String?
    private(set) var isWorking = false
    private(set) var failedAttempts = 0

    let password = SecretBuffer()
    let secretKey = SecretBuffer()
    @ObservationIgnored private var triedQuickUnlock = false
    private unowned let app: AppModel

    init(app: AppModel) {
        self.app = app
    }

    /// Whether this phone can offer quick unlock at all (a Secure Enclave and enrolled biometry).
    var enrollQuickUnlockOffered: Bool { app.keys.isAvailable }

    func canSubmit(needsSecretKey: Bool) -> Bool {
        passwordFilled && (!needsSecretKey || secretKeyFilled) && !isWorking
    }

    /// Unlocks with the typed password (and Secret Key, when the phone has none).
    func submit(needsSecretKey: Bool) async {
        guard canSubmit(needsSecretKey: needsSecretKey) else { return }
        isWorking = true
        errorMessage = nil
        defer { isWorking = false }
        do {
            try await app.unlockWithPassword(
                password: password, secretKey: needsSecretKey ? secretKey : nil,
                enrollQuickUnlock: enrollQuickUnlock && app.keys.isAvailable)
            failedAttempts = 0
            wipe()
        } catch {
            let code = ErrorText.code(of: error)
            if code == "wrongCredentials" { failedAttempts += 1 }
            errorMessage = ErrorText.message(forCode: code)
            password.wipe()
            passwordFilled = false
            passwordClearToken += 1
        }
    }

    /// Unlocks with Face ID or Touch ID. Runs once when the screen appears, and from its button.
    func quickUnlock(automatic: Bool = false) async {
        if automatic {
            guard !triedQuickUnlock else { return }
            triedQuickUnlock = true
        }
        guard !isWorking else { return }
        isWorking = true
        errorMessage = nil
        defer { isWorking = false }
        do {
            try await app.unlockWithQuickUnlock()
        } catch {
            if let key = error as? QuickUnlockKeyError, key == .cancelled { return }
            let code = ErrorText.code(of: error)
            errorMessage = code == "quickUnlockInvalidated"
                ? L10n.unlockQuickInvalidated() : ErrorText.message(forCode: code)
        }
    }

    /// The app left the foreground: a shown Secret Key goes back to hidden.
    func sceneWentToBackground() {
        showSecretKey = false
    }

    func wipe() {
        showSecretKey = false
        password.wipe()
        secretKey.wipe()
        passwordFilled = false
        secretKeyFilled = false
        passwordClearToken += 1
        secretKeyClearToken += 1
    }
}

struct UnlockView: View {
    @Environment(\.colorScheme) private var colorScheme
    @Bindable var model: UnlockModel
    let capture: CaptureObserver
    let email: String?
    let secretKeyStored: Bool
    let availability: QuickUnlockAvailability

    private var quickUnlockAvailable: Bool {
        if case .available = availability { return true }
        return false
    }

    private var passwordRequiredAfterBoot: Bool {
        if case .passwordRequired = availability { return true }
        return false
    }

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.unlockTitle())
                .accessibilityIdentifier("unlock-title")
            if let email {
                BodyText(text: L10n.unlockAccount(email: email), muted: true)
            }
            if !secretKeyStored {
                BodyText(text: L10n.unlockSecretKeyNeeded())
                SecretField(
                    label: L10n.onboardingSecretKey(), buffer: model.secretKey,
                    identifier: "unlock-secret-key", clearToken: model.secretKeyClearToken,
                    reveal: model.showSecretKey && capture.revealAllowed,
                    onChange: { empty in model.secretKeyFilled = !empty })
                if let note = capture.hiddenNote {
                    BodyText(text: note, muted: true)
                } else {
                    Toggle(isOn: $model.showSecretKey) {
                        Text(L10n.itemSecretRevealAction(field: L10n.onboardingSecretKey()))
                            .font(Font.mosaic(MosaicTokens.FontSize.sm))
                    }
                    .accessibilityIdentifier("unlock-secret-key-reveal")
                }
            }
            SecretField(
                label: L10n.unlockPasswordLabel(), buffer: model.password, identifier: "unlock-password",
                clearToken: model.passwordClearToken,
                onChange: { empty in model.passwordFilled = !empty },
                onSubmit: { Task { await model.submit(needsSecretKey: !secretKeyStored) } })
            if passwordRequiredAfterBoot {
                // After a restart and after five wrong tries alike: the same plain reason.
                BodyText(text: L10n.errorPasswordRequired(), muted: true)
            }
            if !quickUnlockAvailable && !passwordRequiredAfterBoot && model.enrollQuickUnlockOffered {
                OptionRow(
                    text: L10n.unlockQuickOptionFaceId(), isOn: $model.enrollQuickUnlock,
                    identifier: "unlock-enroll-quick")
            }
            if let message = model.errorMessage {
                ErrorLine(text: message)
                if model.failedAttempts > 1 {
                    BodyText(text: L10n.unlockFailed(attempts: model.failedAttempts), muted: true)
                }
            }
            if model.isWorking {
                BodyText(text: L10n.unlockWorking(), muted: true)
            }
            Button(L10n.unlockAction()) { Task { await model.submit(needsSecretKey: !secretKeyStored) } }
                .buttonStyle(PrimaryButtonStyle())
                .disabled(!model.canSubmit(needsSecretKey: !secretKeyStored))
                .accessibilityIdentifier("unlock-submit")
            if quickUnlockAvailable {
                Button(L10n.unlockQuick()) { Task { await model.quickUnlock() } }
                    .buttonStyle(SecondaryButtonStyle())
                    .disabled(model.isWorking)
                    .accessibilityIdentifier("unlock-quick")
            }
        }
        .task {
            if quickUnlockAvailable {
                await model.quickUnlock(automatic: true)
            }
        }
    }
}
