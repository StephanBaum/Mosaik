import Foundation

/// Where an AutoFill request stands once the store is open (spec 3.11, iOS).
public enum ProviderReadiness: Sendable, Equatable {
    /// No account on this phone.
    case noAccount
    /// The app (or another extension run) holds the store, or a call is running.
    case busy
    /// Locked and quick unlock does not apply (not enrolled, expired, or a reboot since the last password
    /// unlock). The extension never runs Argon2id, so the user opens the app.
    case needsApp
    /// Locked, and the Secure Enclave key can open it.
    case quickUnlock
    case unlocked
}

/// The logins for one request: the ones that may fill the target, and every other login, read-only.
public struct CredentialList: Sendable {
    /// What the request is for. Nil when the system gave no identifier the core accepts.
    public let target: FillTarget?
    /// The site to show (`target.display`).
    public let site: String?
    public let warning: OriginWarning?
    /// Logins the core says may fill the target (exact matches first).
    public let matches: [FillCandidate]
    /// Every other login. No select action: the core would refuse it.
    public let others: [ItemSummary]
    /// A reason the core gave for showing no matches (`insecureOrigin`).
    public let refusalCode: String?

    public init(
        target: FillTarget?, warning: OriginWarning?, matches: [FillCandidate], others: [ItemSummary],
        refusalCode: String?
    ) {
        self.target = target
        self.site = target?.display
        self.warning = warning
        self.matches = matches
        self.others = others
        self.refusalCode = refusalCode
    }
}

/// A user name and password for `ASPasswordCredential`. The system API takes `String`s, so these two values
/// cannot be wiped; they live only until the request completes (honest limit, THREAT_MODEL R131).
public struct ProviderPassword: Sendable {
    public let user: String
    public let password: String
}

/// A passkey the extension just created. `identity` is set when the user turned the passkey identities on, so
/// the extension adds it to the system store (spec 3.12).
public struct ProviderRegistration: Sendable {
    public let registration: IosRegistration
    public let identity: PasskeyIdentity?
}

/// What one request has done so far, behind a lock (the session is used from the main actor and from the
/// serial core queue).
private final class RequestFlags: @unchecked Sendable {
    private let lock = NSLock()
    private var verified = false
    private var fillSession: String?

    var isVerified: Bool {
        lock.lock()
        defer { lock.unlock() }
        return verified
    }

    func setVerified(_ value: Bool) {
        lock.lock()
        defer { lock.unlock() }
        verified = value
    }

    /// Stores the new fill session and returns the one it replaces.
    func swapFillSession(_ id: String?) -> String? {
        lock.lock()
        defer { lock.unlock() }
        let old = fillSession
        fillSession = id
        return old
    }

    var currentFillSession: String? {
        lock.lock()
        defer { lock.unlock() }
        return fillSession
    }
}

/// One AutoFill request of the extension (spec 3.8, 3.11, 3.12): it opens the store, unlocks with the Secure
/// Enclave key, asks the core for the logins or the passkey, and `finish()` releases the fill session, locks
/// and closes the store. The core decides every match and every refusal; this class only carries values
/// between the system API and the core.
///
/// A session serves one request. The extension makes a new one per request, so each request needs its own
/// biometric prompt, and a passkey ceremony always has user verification (ADR-0093).
public final class ProviderSession: Sendable {
    private let holder: CoreHolder
    private let keys: any QuickUnlockKeys
    private let flags = RequestFlags()

    public init(holder: CoreHolder, keys: any QuickUnlockKeys) {
        self.holder = holder
        self.keys = keys
    }

    /// Opens the extension's core on the shared store with the Secure Enclave key of this install.
    public convenience init(identity: AppIdentity = .from()) throws {
        try self.init(
            holder: CoreFactory.openExtension(identity: identity), keys: SecureEnclaveQuickUnlock(identity: identity))
    }

    // MARK: Readiness and unlock

    /// Reopens the store if it was closed and says what the request needs next. A session that is already
    /// unlocked (it was not this request that unlocked it) is locked first, so every request verifies the
    /// user itself.
    public func begin() async throws -> ProviderReadiness {
        flags.setVerified(false)
        var state = holder.state()
        switch state {
        case .storeClosed, .storeBusy:
            // Closed by an earlier request, or held by the app when this process opened it: try to open now.
            do {
                state = try await holder.call { core in try core.reopenStore() }
            } catch {
                if ErrorText.code(of: error) == "storeInUse" { return .busy }
                throw error
            }
        default:
            break
        }
        if case .unlocked = state {
            holder.lock()
            state = holder.state()
        }
        return Self.readiness(of: state)
    }

    static func readiness(of state: SessionState) -> ProviderReadiness {
        switch state {
        case .noAccount:
            return .noAccount
        case .storeBusy, .busy, .storeClosed:
            return .busy
        case let .locked(_, _, availability):
            if case .available = availability {
                return .quickUnlock
            }
            return .needsApp
        case .unlocked:
            return .unlocked
        }
    }

    /// Unlocks with Face ID or Touch ID (one prompt). This is the user verification of a passkey ceremony.
    public func unlockQuick(reason: String, cancelTitle: String) async throws {
        try await QuickUnlockFlow.unlock(holder: holder, keys: keys, reason: reason, cancelTitle: cancelTitle)
        flags.setVerified(true)
    }

    // MARK: Passwords and one-time codes

    /// The logins for `identifiers`. The first identifier the core turns into a target is the request's target;
    /// without one every login is listed read-only. Opens a fill session, which the next `list` or `finish`
    /// releases.
    public func list(identifiers: [ServiceIdentifier]) async throws -> CredentialList {
        var target: FillTarget?
        var refusal: String?
        for identifier in identifiers {
            do {
                target = try await holder.call { core in try core.fillTargetIos(identifier: identifier) }
                refusal = nil
                break
            } catch {
                refusal = ErrorText.code(of: error)
            }
        }
        var matches: [FillCandidate] = []
        var warning: OriginWarning?
        if let target {
            warning = try await holder.call { core in try core.fillWarning(target: target) }
            do {
                let found = try await holder.call { core in try core.fillCandidates(target: target) }
                if let old = flags.swapFillSession(found.sessionId) {
                    holder.releaseFillSession(old)
                }
                matches = found.candidates
            } catch {
                let code = ErrorText.code(of: error)
                guard code == "insecureOrigin" else { throw error }
                refusal = code
            }
        }
        let filter = ItemFilter(vaultId: nil, trash: .exclude, favoritesOnly: false, kind: .login)
        let all = try await holder.call { core in try core.items(filter: filter) }
        let matched = Set(matches.map(\.key))
        let others = all.filter { !matched.contains(ItemKey(vaultId: $0.vaultId, itemId: $0.itemId)) }
        return CredentialList(target: target, warning: warning, matches: matches, others: others, refusalCode: refusal)
    }

    /// The user name and password of `key`. The core checks the match against the target of the fill session
    /// again; a key that does not match is refused (`noMatch`).
    public func password(for key: ItemKey) async throws -> ProviderPassword {
        guard let session = flags.currentFillSession else { throw CodedError("notFound") }
        let values = try await holder.call { core in try core.fillValues(sessionId: session, candidate: key) }
        defer { values.password?.wipe() }
        var bytes = values.password?.exposeBytes() ?? Data()
        defer { bytes.wipe() }
        return ProviderPassword(user: values.username ?? "", password: String(decoding: bytes, as: UTF8.self))
    }

    /// The current one-time code of `key` (iOS 18 provider).
    public func oneTimeCode(for key: ItemKey) async throws -> String {
        guard let session = flags.currentFillSession else { throw CodedError("notFound") }
        let secret = try await holder.call { core in try core.fillOtp(sessionId: session, candidate: key) }
        defer { secret.wipe() }
        var bytes = secret.exposeBytes()
        defer { bytes.wipe() }
        return String(decoding: bytes, as: UTF8.self)
    }

    // MARK: Passkeys

    /// The passkeys of `rpId` that can answer the request (`allow` empty: any discoverable one).
    public func passkeyCandidates(rpId: String, allow: [Data]) async throws -> [PasskeyCandidate] {
        try await holder.call { core in try core.passkeyIosCandidates(rpId: rpId, allow: allow) }
    }

    /// Signs an assertion in hash mode. Only after a quick unlock of this request (user verification); the
    /// core refuses a session it did not open that way as well.
    public func assertPasskey(rpId: String, clientDataHash: Data, credentialId: Data) async throws -> IosAssertion {
        guard flags.isVerified else { throw CodedError("reauthRequired") }
        let request = IosAssertRequest(
            rpId: rpId, clientDataHash: clientDataHash, credentialId: credentialId, userVerified: true)
        return try await holder.call { core in try core.passkeyIosAssert(req: request) }
    }

    /// Signs for a passkey picked from the list. The candidate has no credential ID, so it is read from the
    /// identity of the same item (the identities carry no key material).
    public func assertPasskey(rpId: String, clientDataHash: Data, candidate: ItemKey) async throws -> IosAssertion {
        guard flags.isVerified else { throw CodedError("reauthRequired") }
        let credentialId = try await holder.call { core in
            guard let found = try core.passkeyIdentities().first(where: { $0.record == candidate }) else {
                throw CodedError("passkeyNotFound")
            }
            return found.credentialId
        }
        return try await assertPasskey(rpId: rpId, clientDataHash: clientDataHash, credentialId: credentialId)
    }

    /// The vaults a new passkey can go to.
    public func availableVaults() async throws -> [VaultSummary] {
        try await holder.call { core in try core.vaults().filter { $0.available } }
    }

    /// Creates a passkey in hash mode and stores it before the attestation is returned (ADR-0093). The result
    /// carries the identity for the system store when the user turned passkey identities on.
    public func registerPasskey(
        rpId: String, userName: String, userHandle: Data, clientDataHash: Data, algorithms: [Int64],
        excludedCredentialIds: [Data], vaultId: String
    ) async throws -> ProviderRegistration {
        guard flags.isVerified else { throw CodedError("reauthRequired") }
        let request = IosRegisterRequest(
            rpId: rpId, userName: userName, userHandle: userHandle, clientDataHash: clientDataHash,
            algorithms: algorithms, excludeCredentialIds: excludedCredentialIds, vaultId: vaultId,
            userVerified: true)
        return try await holder.call { core in
            let registration = try core.passkeyIosRegister(req: request)
            var identity: PasskeyIdentity?
            if core.settings().passkeyIdentities {
                identity = try? core.passkeyIdentities().first { $0.credentialId == registration.credentialId }
            }
            return ProviderRegistration(registration: registration, identity: identity)
        }
    }

    // MARK: End of the request

    /// Releases the fill session, locks (which wipes every handed-out secret) and closes the store with its
    /// file lock. Safe to call more than once and on every exit path; it does not wait for a running call.
    public func finish() {
        flags.setVerified(false)
        if let session = flags.swapFillSession(nil) {
            holder.releaseFillSession(session)
        }
        holder.closeStore()
    }
}
