import MosaicKit
import Observation
import SwiftUI
import UIKit

/// The state of the app: the core, the session, the privacy cover and the lock triggers (spec 3.8).
///
/// The core owns every policy. This class opens it, mirrors its `SessionState`, closes the store when the
/// scene goes to the background (`0xdead10cc`), opens it again when the scene is active, and locks on the
/// idle timeout and on the device lock.
@MainActor
@Observable
final class AppModel {
    enum Phase {
        case starting
        case failed(String)
        case ready
    }

    private(set) var phase: Phase = .starting
    private(set) var session: SessionState = .storeClosed
    /// Set while the onboarding runs; it outlives the unlock that the join causes, up to its last step.
    var onboarding: OnboardingModel?
    private(set) var isSceneActive = true
    /// The unlock screen's state. A new one replaces it when the vault opens, so no typed secret survives.
    private(set) var unlock: UnlockModel?

    let privacy = PrivacyCoverModel()
    let capture = CaptureObserver()
    let identity: AppIdentity

    let keys: any QuickUnlockKeys
    @ObservationIgnored private(set) var holder: CoreHolder?
    @ObservationIgnored private var lifecycle: StoreLifecycle?
    @ObservationIgnored private var idle: IdleLock?
    @ObservationIgnored private var idleStarted = false
    @ObservationIgnored private var protectedDataObserver: NSObjectProtocol?

    init() {
        let identity = AppIdentity.from()
        self.identity = identity
        self.keys = SecureEnclaveQuickUnlock(identity: identity)
        MosaicFonts.register()
        do {
            let holder = try CoreFactory.openApp(identity: identity)
            self.holder = holder
            self.lifecycle = StoreLifecycle(holder: holder)
            self.phase = .ready
        } catch {
            self.phase = .failed(ErrorText.message(for: error))
        }
        self.idle = IdleLock { [weak self] in self?.lock() }
        self.protectedDataObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.protectedDataWillBecomeUnavailableNotification, object: nil, queue: .main
        ) { [weak self] _ in
            MainActor.assumeIsolated {
                self?.lock()
            }
        }
        capture.start()
        self.unlock = UnlockModel(app: self)
        refreshState()
    }

    // MARK: State

    /// Reads the state of the core and runs what it asks for: a quick-unlock key that ran out is deleted,
    /// the idle timer follows the unlocked state, and the onboarding starts when there is no account.
    func refreshState() {
        guard let holder else { return }
        session = holder.state()
        if case let .locked(_, _, availability) = session, case let .expired(alias) = availability {
            try? keys.deleteKey(alias: alias)
        }
        if case .noAccount = session, onboarding == nil {
            onboarding = OnboardingModel(app: self)
        }
        if case .unlocked = session {
            unlock?.wipe()
            unlock = UnlockModel(app: self)
            startIdleLock()
        } else {
            idle?.stop()
            idleStarted = false
        }
    }

    private func startIdleLock() {
        guard !idleStarted, let holder else { return }
        idleStarted = true
        Task {
            let minutes = (try? await holder.call { core in Int(core.settings().idleLockMinutes) }) ?? 5
            idle?.start(minutes: minutes)
        }
    }

    /// A touch anywhere: the idle countdown starts over.
    func userActivity() {
        idle?.touch()
    }

    func lock() {
        holder?.lock()
        refreshState()
    }

    func onboardingFinished() {
        onboarding = nil
        refreshState()
    }

    // MARK: Scene

    func scenePhaseChanged(_ phase: ScenePhase) {
        switch phase {
        case .inactive:
            isSceneActive = false
            privacy.sceneWillResignActive()
        case .background:
            isSceneActive = false
            privacy.sceneWillResignActive()
            idle?.stop()
            idleStarted = false
            lifecycle?.sceneDidEnterBackground()
            refreshState()
        case .active:
            isSceneActive = true
            privacy.sceneDidBecomeActive()
            Task { await reopenStore() }
        @unknown default:
            break
        }
    }

    func reopenStore() async {
        guard let lifecycle else { return }
        _ = try? await lifecycle.sceneDidBecomeActive()
        refreshState()
    }

    // MARK: Unlock

    /// The memory preflight (CRYPTO_SPEC 6.9.4): Argon2id does not start without the memory it needs.
    func preflightMemory() async throws {
        guard let holder else { throw CodedError("storeUnavailable") }
        let required = try await holder.call { core in core.requiredUnlockMemoryBytes() }
        guard MemoryCheck.hasEnough(required: required, available: MemoryCheck.availableBytes()) else {
            throw CodedError("notEnoughMemory")
        }
    }

    /// Unlocks with the master password, and with the Secret Key when the phone does not have it. With
    /// `enrollQuickUnlock` it enrols quick unlock with the same typed bytes, inside the same call.
    func unlockWithPassword(password: SecretBuffer, secretKey: SecretBuffer?, enrollQuickUnlock: Bool) async throws {
        guard let holder else { throw CodedError("storeUnavailable") }
        try await preflightMemory()
        let keys = self.keys
        let enrol = enrollQuickUnlock
        defer { refreshState() }
        try await holder.call { core in
            var pw = password.copyOut()
            defer { pw.wipe() }
            var sk = secretKey.map { $0.copyOut() }
            defer { sk?.wipe() }
            _ = try core.unlockPassword(password: pw, secretKey: sk)
            if enrol {
                _ = QuickUnlockFlow.enroll(core: core, password: pw, keys: keys)
            }
        }
    }

    /// Unlocks with Face ID or Touch ID through the Secure Enclave key.
    func unlockWithQuickUnlock() async throws {
        guard let holder else { throw CodedError("storeUnavailable") }
        defer { refreshState() }
        try await QuickUnlockFlow.unlock(
            holder: holder, keys: keys, reason: L10n.unlockBiometricReason(), cancelTitle: L10n.unlockUsePassword())
    }
}
