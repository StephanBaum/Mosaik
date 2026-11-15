import Foundation
import MosaicKit
import Observation

/// What the system asked the extension for. Plain values only: the system's request objects are read in the
/// view controller, so nothing non-`Sendable` reaches the model.
enum ProviderRequest: Sendable {
    /// The manual list (the user tapped "Passwords"): matches first, the rest read-only.
    case passwordList(identifiers: [ServiceIdentifier])
    /// A tap on a QuickType suggestion. `record` is the identity's hint; the core still checks the match.
    case passwordProvide(identifiers: [ServiceIdentifier], record: ItemKey?)
    /// The passkey list of a sign-in sheet (and the passwords of the site next to it).
    case passkeyList(identifiers: [ServiceIdentifier], rpId: String, clientDataHash: Data, allowed: [Data])
    /// A tap on a passkey suggestion.
    case passkeyAssert(rpId: String, credentialId: Data, clientDataHash: Data)
    case passkeyRegister(PasskeyRegistrationRequest)
    /// iOS 18: the one-time-code list.
    case oneTimeCodeList(identifiers: [ServiceIdentifier])
    case configuration
}

struct PasskeyRegistrationRequest: Sendable {
    let rpId: String
    let userName: String
    let userHandle: Data
    let clientDataHash: Data
    let algorithms: [Int64]
    /// The credential IDs the relying party already has for this user (iOS 18+; empty before).
    let excluded: [Data]
}

/// How a request ended. The view controller turns it into the system's completion call.
enum ProviderOutcome {
    case password(ProviderPassword)
    case oneTimeCode(String)
    case assertion(IosAssertion, rpId: String, clientDataHash: Data)
    case registration(IosRegistration, rpId: String, clientDataHash: Data)
    case configurationDone
    case cancelled
    /// The passkey named by a stale system identity is gone: the system drops the identity.
    case credentialNotFound
    case failed
}

/// The state of the extension's one screen. It runs the unlock and the request through `ProviderSession`,
/// shows what the core says, and ends the request exactly once.
@MainActor
@Observable
final class ProviderModel {
    enum Screen {
        case loading
        case message(String, retry: Bool)
        case unlock
        /// A QuickType selection: the saved site is shown and nothing fills before the user confirms it.
        case confirmFill(site: String)
        case logins
        case passkeys
        case confirmRegistration
        case configuration
    }

    /// The backstop for a request nobody ends: an idle extension gives up after this long (the store stays open
    /// while it waits). The timer is re-armed on every action and does not run while the process is suspended,
    /// so the dismissal and the host going to the background (`DismissalObserver`) are the real bound.
    static let idleSeconds = 120

    private(set) var screen: Screen = .loading
    private(set) var list: CredentialList?
    private(set) var passkeys: [PasskeyCandidate] = []
    private(set) var vaults: [VaultSummary] = []
    private(set) var errorText: String?
    private(set) var isWorking = false
    var selectedVault: String?
    var showAll = false
    var query = ""

    let request: ProviderRequest
    @ObservationIgnored private let session: ProviderSession?
    @ObservationIgnored private let openFailure: Error?
    @ObservationIgnored private let onFinish: (ProviderOutcome) -> Void
    /// Ends the request once: lock, close the store, release the fill session (M1).
    @ObservationIgnored private let lifecycle: RequestLifecycle
    @ObservationIgnored private var triedAutomaticUnlock = false
    @ObservationIgnored private var fillConfirmed = false
    @ObservationIgnored private var timeout: Task<Void, Never>?

    init(
        request: ProviderRequest, makeSession: () throws -> ProviderSession,
        onFinish: @escaping (ProviderOutcome) -> Void
    ) {
        self.request = request
        self.onFinish = onFinish
        var opened: ProviderSession?
        var failure: Error?
        if case .configuration = request {
            // The configuration screen needs no vault: no session, so the store is not opened.
        } else {
            do {
                opened = try makeSession()
            } catch {
                failure = error
            }
        }
        self.session = opened
        self.openFailure = failure
        self.lifecycle = RequestLifecycle(close: { [opened] in opened?.finish() })
    }

    var isOneTimeCode: Bool {
        if case .oneTimeCodeList = request { return true }
        return false
    }

    /// The matches of the list. A one-time-code request lists only logins that have a generator.
    var visibleMatches: [FillCandidate] {
        let all = list?.matches ?? []
        return isOneTimeCode ? all.filter(\.hasOtp) : all
    }

    /// The other logins, only after "Search all", filtered by the search text.
    var visibleOthers: [ItemSummary] {
        guard showAll, let others = list?.others else { return [] }
        let needle = query.trimmingCharacters(in: .whitespaces).lowercased()
        return others.filter { item in
            (!isOneTimeCode || item.hasOtp)
                && (needle.isEmpty || item.title.lowercased().contains(needle)
                    || (item.subtitle ?? "").lowercased().contains(needle))
        }
    }

    var isRegistrationConfirmable: Bool { selectedVault != nil && !isWorking }

    // MARK: Request

    func start() async {
        guard !lifecycle.isFinished else {
            tellSilentEnd()
            return
        }
        armTimeout()
        if case .configuration = request {
            screen = .configuration
            return
        }
        guard let session else {
            fail(openFailure ?? CodedError("internal"))
            return
        }
        screen = .loading
        do {
            switch try await session.begin() {
            case .noAccount:
                screen = .message(L10n.extNoAccount(), retry: false)
            case .busy:
                screen = .message(L10n.unlockStoreBusy(), retry: true)
            case .needsApp, .unlocked:
                screen = .message(L10n.extOpenApp(), retry: false)
            case .quickUnlock:
                if case let .passwordProvide(identifiers, _) = request {
                    // No automatic fill and no automatic Face ID: the user confirms the saved site first.
                    screen = .confirmFill(site: ProviderFillPolicy.savedSite(of: identifiers) ?? L10n.extThisSite())
                } else {
                    screen = .unlock
                    await unlock(automatic: true)
                }
            }
        } catch {
            fail(error)
        }
    }

    /// Face ID or Touch ID. Runs once when the screen appears and again from its button.
    func unlock(automatic: Bool = false) async {
        tellSilentEnd()
        guard let session, !isWorking, !lifecycle.isFinished else { return }
        if automatic {
            guard !triedAutomaticUnlock else { return }
            triedAutomaticUnlock = true
        }
        armTimeout()
        isWorking = true
        errorText = nil
        do {
            try await session.unlockQuick(reason: L10n.unlockBiometricReason(), cancelTitle: L10n.commonCancel())
        } catch {
            isWorking = false
            if let key = error as? QuickUnlockKeyError, key == .cancelled { return }
            switch ErrorText.code(of: error) {
            case "quickUnlockInvalidated":
                screen = .message(L10n.unlockQuickInvalidated(), retry: false)
            case "passwordRequired", "quickUnlockExpired", "passwordInApp":
                screen = .message(L10n.extOpenApp(), retry: false)
            default:
                errorText = ErrorText.message(for: error)
            }
            return
        }
        isWorking = false
        await proceed()
    }

    /// The user confirmed "Fill the login for <site>?": now the Face ID prompt, then the fill.
    func confirmFill() async {
        tellSilentEnd()
        guard case .confirmFill = screen, !isWorking, !lifecycle.isFinished else { return }
        fillConfirmed = true
        screen = .unlock
        await unlock(automatic: true)
    }

    private func proceed() async {
        guard let session, !lifecycle.isFinished else { return }
        screen = .loading
        do {
            switch request {
            case let .passwordList(identifiers), let .oneTimeCodeList(identifiers):
                list = try await session.list(identifiers: identifiers)
                screen = .logins
            case let .passwordProvide(identifiers, record):
                let found = try await session.list(identifiers: identifiers)
                list = found
                screen = .logins
                if let key = ProviderFillPolicy.loginToFill(
                    record: record, matches: found.matches.map(\.key), confirmed: fillConfirmed)
                {
                    await fill(key)
                }
            case let .passkeyList(identifiers, rpId, _, allowed):
                passkeys = try await session.passkeyCandidates(rpId: rpId, allow: allowed)
                if !identifiers.isEmpty {
                    list = try? await session.list(identifiers: identifiers)
                }
                screen = .passkeys
            case let .passkeyAssert(rpId, credentialId, hash):
                let assertion = try await session.assertPasskey(
                    rpId: rpId, clientDataHash: hash, credentialId: credentialId)
                finish(.assertion(assertion, rpId: rpId, clientDataHash: hash))
            case .passkeyRegister:
                vaults = try await session.availableVaults()
                selectedVault = vaults.first?.vaultId
                if vaults.isEmpty {
                    screen = .message(L10n.errorVaultUnavailable(), retry: false)
                } else {
                    screen = .confirmRegistration
                }
            case .configuration:
                screen = .configuration
            }
        } catch {
            if case .passkeyAssert = request, ["passkeyNotFound", "notFound"].contains(ErrorText.code(of: error)) {
                finish(.credentialNotFound)
                return
            }
            fail(error)
        }
    }

    // MARK: Actions

    /// The user chose a login (a tap on a match).
    func fill(_ key: ItemKey) async {
        tellSilentEnd()
        guard let session, !isWorking, !lifecycle.isFinished else { return }
        armTimeout()
        isWorking = true
        errorText = nil
        defer { isWorking = false }
        do {
            if isOneTimeCode {
                finish(.oneTimeCode(try await session.oneTimeCode(for: key)))
            } else {
                finish(.password(try await session.password(for: key)))
            }
        } catch {
            errorText = ErrorText.message(for: error)
        }
    }

    /// The user chose a passkey from the sign-in sheet.
    func choose(passkey: PasskeyCandidate) async {
        tellSilentEnd()
        guard let session, !isWorking, !lifecycle.isFinished, case let .passkeyList(_, rpId, hash, _) = request else {
            return
        }
        armTimeout()
        isWorking = true
        errorText = nil
        defer { isWorking = false }
        do {
            let assertion = try await session.assertPasskey(rpId: rpId, clientDataHash: hash, candidate: passkey.key)
            finish(.assertion(assertion, rpId: rpId, clientDataHash: hash))
        } catch {
            errorText = ErrorText.message(for: error)
        }
    }

    /// The user confirmed "Save passkey".
    func confirmRegistration() async {
        tellSilentEnd()
        guard let session, !lifecycle.isFinished, case let .passkeyRegister(register) = request,
            let vault = selectedVault, !isWorking
        else {
            return
        }
        isWorking = true
        errorText = nil
        defer { isWorking = false }
        do {
            let made = try await session.registerPasskey(
                rpId: register.rpId, userName: register.userName, userHandle: register.userHandle,
                clientDataHash: register.clientDataHash, algorithms: register.algorithms,
                excludedCredentialIds: register.excluded, vaultId: vault)
            if let identity = made.identity {
                await IdentityStoreSync.add(identity, to: SystemIdentityStore())
            }
            finish(.registration(made.registration, rpId: register.rpId, clientDataHash: register.clientDataHash))
        } catch {
            fail(error)
        }
    }

    func retry() async {
        await start()
    }

    func cancel() {
        tellSilentEnd()
        if case .configuration = screen {
            finish(.configurationDone)
        } else {
            finish(.cancelled)
        }
    }

    // MARK: End

    /// A newer request replaces this one, or the sheet is dismissed, or the host went away: the session locks,
    /// closes the store and releases the fill session, and nobody is told. Safe to call any number of times.
    ///
    /// A silent end leaves the sheet on screen, and the system has not heard about it (G3i-4). Unless a newer
    /// request `replaced` this one (it owns the sheet), the next user action or the host becoming active again
    /// tells the system once (`tellSilentEnd`, `hostDidBecomeActive`).
    func cancelSilently(replaced: Bool = false) {
        timeout?.cancel()
        lifecycle.end()
        if replaced {
            lifecycle.markTold()
        }
    }

    /// The host became active again (`NSExtensionHostDidBecomeActive`). A request that ended silently is told
    /// to the system now.
    func hostDidBecomeActive() {
        tellSilentEnd()
    }

    /// Tells the system once that a silently ended request is over (`userCanceled`).
    private func tellSilentEnd() {
        if lifecycle.claimTelling() {
            onFinish(.cancelled)
        }
    }

    private func fail(_ error: Error) {
        let code = ErrorText.code(of: error)
        screen = .message(ErrorText.message(forCode: code), retry: code == "busy" || code == "storeInUse")
    }

    /// Ends the request once: the session releases its fill session, locks and closes the store first, then
    /// the system hears the outcome.
    private func finish(_ outcome: ProviderOutcome) {
        timeout?.cancel()
        guard lifecycle.end() else { return }
        _ = lifecycle.claimTelling()
        onFinish(outcome)
    }

    private func armTimeout() {
        timeout?.cancel()
        timeout = Task { [weak self] in
            try? await Task.sleep(for: .seconds(ProviderModel.idleSeconds))
            guard !Task.isCancelled else { return }
            self?.finish(.cancelled)
        }
    }
}
