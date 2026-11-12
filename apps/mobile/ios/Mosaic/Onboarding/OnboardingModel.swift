import AuthenticationServices
import MosaicKit
import Observation
import SwiftUI
import UIKit

/// The onboarding of a phone without an account (spec 3.14): welcome, the account details, the master
/// password, a second factor when the account has one, quick unlock, AutoFill setup, done. The scanner
/// for the desktop setup code arrives with T13, so the details are typed.
@MainActor
@Observable
final class OnboardingModel {
    enum Step: Equatable {
        case welcome
        case details
        case password
        case secondFactor
        case securityKey
        case autofill
        case done
    }

    var step: Step = .welcome
    var server = ""
    var email = ""
    var deviceName = "iPhone"
    var totp = ""
    var enrollQuickUnlock = true
    /// Bumped to empty the secret fields.
    var passwordClearToken = 0
    var secretKeyClearToken = 0
    var showSecretKey = false
    private(set) var secondFactor: SecondFactorMethods?
    private(set) var handoffCode: String?
    private(set) var errorMessage: String?
    private(set) var isWorking = false

    let password = SecretBuffer()
    let secretKey = SecretBuffer()
    /// The state of the two secret fields, for the buttons.
    var passwordFilled = false
    var secretKeyFilled = false

    private unowned let app: AppModel
    @ObservationIgnored private var canonicalServer: String?
    @ObservationIgnored private var pollTask: Task<Void, Never>?

    init(app: AppModel) {
        self.app = app
    }

    var canSubmitDetails: Bool {
        !server.trimmingCharacters(in: .whitespaces).isEmpty
            && !email.trimmingCharacters(in: .whitespaces).isEmpty && secretKeyFilled && !isWorking
    }

    var canSignIn: Bool { passwordFilled && !isWorking }

    var canUseQuickUnlock: Bool { app.keys.isAvailable }

    // MARK: Steps

    func start() {
        errorMessage = nil
        step = .details
    }

    func back() {
        errorMessage = nil
        switch step {
        case .details:
            step = .welcome
        case .password:
            step = .details
        case .secondFactor, .securityKey:
            cancelJoin()
        default:
            break
        }
    }

    /// Checks the server (the canonical origin, `/v1/info`) before the password is asked for.
    func submitDetails() async {
        guard canSubmitDetails, let holder = app.holder else { return }
        isWorking = true
        errorMessage = nil
        defer { isWorking = false }
        let address = server.trimmingCharacters(in: .whitespaces)
        do {
            let info = try await holder.call { core in try core.preloginCheck(serverUrl: address) }
            canonicalServer = info.origin
            step = .password
        } catch {
            errorMessage = ErrorText.message(for: error)
        }
    }

    func signIn() async {
        guard canSignIn, let holder = app.holder else { return }
        isWorking = true
        errorMessage = nil
        defer { isWorking = false }
        let origin = canonicalServer ?? server.trimmingCharacters(in: .whitespaces)
        let mail = email.trimmingCharacters(in: .whitespaces)
        let device = deviceName.trimmingCharacters(in: .whitespaces)
        let passwordBuffer = password
        let keyBuffer = secretKey
        do {
            let outcome = try await holder.call { core in
                var pw = passwordBuffer.copyOut()
                defer { pw.wipe() }
                var sk = keyBuffer.copyOut()
                defer { sk.wipe() }
                return try core.joinHosted(
                    req: JoinHostedRequest(
                        serverUrl: origin, email: mail, password: pw, secretKey: sk,
                        deviceName: device.isEmpty ? nil : device))
            }
            switch outcome {
            case .unlocked:
                await finishJoin()
            case let .secondFactor(methods):
                secondFactor = methods
                step = .secondFactor
            }
        } catch {
            errorMessage = ErrorText.message(for: error)
            wipePassword()
        }
    }

    func submitTotp() async {
        guard let holder = app.holder, !isWorking else { return }
        let code = Data(totp.trimmingCharacters(in: .whitespaces).utf8)
        guard !code.isEmpty else { return }
        isWorking = true
        errorMessage = nil
        defer { isWorking = false }
        do {
            _ = try await holder.call { core in try core.joinTotp(code: code) }
            totp = ""
            await finishJoin()
        } catch {
            errorMessage = ErrorText.message(for: error)
        }
    }

    /// Opens the handoff page in the browser and polls until the security key is done.
    func startSecurityKey() async {
        guard let holder = app.holder, !isWorking else { return }
        isWorking = true
        errorMessage = nil
        defer { isWorking = false }
        do {
            let info = try await holder.call { core in try core.joinWebauthnBegin() }
            guard let url = URL(string: info.url) else {
                errorMessage = ErrorText.message(forCode: "syncFailed")
                return
            }
            handoffCode = info.matchCode
            step = .securityKey
            startPolling(holder: holder)
            _ = await UIApplication.shared.open(url)
        } catch {
            errorMessage = ErrorText.message(for: error)
        }
    }

    private func startPolling(holder: CoreHolder) {
        pollTask?.cancel()
        pollTask = Task { [weak self] in
            while !Task.isCancelled {
                guard let self else { return }
                if self.app.isSceneActive {
                    do {
                        let status = try await holder.call { core in try core.joinWebauthnStatus() }
                        switch status {
                        case .waiting:
                            break
                        case .done:
                            await self.finishJoin()
                            return
                        case .failed:
                            self.errorMessage = ErrorText.message(forCode: "syncTwoFactorInvalid")
                            self.step = .secondFactor
                            return
                        case .expired:
                            self.errorMessage = ErrorText.message(forCode: "syncHandoffExpired")
                            self.step = .secondFactor
                            return
                        }
                    } catch {
                        // The store is opened again when the scene becomes active; until then the poll waits.
                        let code = ErrorText.code(of: error)
                        if code != "storeUnavailable" && code != "busy" && code != "storeInUse" {
                            self.errorMessage = ErrorText.message(for: error)
                            self.step = .secondFactor
                            return
                        }
                    }
                }
                try? await Task.sleep(for: .seconds(2))
            }
        }
    }

    /// Drops the pending join (the handoff, the second factor) and asks for the password again.
    func cancelJoin() {
        pollTask?.cancel()
        pollTask = nil
        if let holder = app.holder {
            Task { _ = try? await holder.call { core in core.joinCancel() } }
        }
        secondFactor = nil
        handoffCode = nil
        step = .password
    }

    /// The phone joined and is unlocked: enrol quick unlock if asked, wipe the typed secrets, go on.
    private func finishJoin() async {
        pollTask?.cancel()
        pollTask = nil
        if enrollQuickUnlock, canUseQuickUnlock, let holder = app.holder {
            let keys = app.keys
            let passwordBuffer = password
            do {
                try await app.preflightMemory()
                _ = try await holder.call { core in
                    var pw = passwordBuffer.copyOut()
                    defer { pw.wipe() }
                    return QuickUnlockFlow.enroll(core: core, password: pw, keys: keys)
                }
            } catch {
                // Quick unlock stays off; the vault is unlocked and the user can turn it on in Settings.
            }
        }
        wipeSecrets()
        app.refreshState()
        step = .autofill
    }

    func goToDone() {
        step = .done
    }

    func finish() {
        pollTask?.cancel()
        wipeSecrets()
        app.onboardingFinished()
    }

    // MARK: AutoFill setup

    var autofillSettingsAvailable: Bool {
        if #available(iOS 18.0, *) { return true }
        return false
    }

    func openAutofillSettings() {
        if #available(iOS 18.0, *) {
            ASSettingsHelper.openCredentialProviderAppSettings { _ in }
        }
    }

    // MARK: Secrets

    private func wipePassword() {
        password.wipe()
        passwordFilled = false
        passwordClearToken += 1
    }

    private func wipeSecrets() {
        password.wipe()
        secretKey.wipe()
        passwordFilled = false
        secretKeyFilled = false
        passwordClearToken += 1
        secretKeyClearToken += 1
    }
}
