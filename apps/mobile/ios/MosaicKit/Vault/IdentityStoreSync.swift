import AuthenticationServices
import Foundation

/// What the system credential identity store should hold (spec 3.11, ADR-0092, R132).
public struct IdentityPlan: Sendable {
    public let passkeys: [PasskeyIdentity]
    public let passwords: [PasswordIdentity]

    public init(passkeys: [PasskeyIdentity], passwords: [PasswordIdentity]) {
        self.passkeys = passkeys
        self.passwords = passwords
    }

    public var isEmpty: Bool { passkeys.isEmpty && passwords.isEmpty }
}

/// The system store, behind a protocol so the policy is testable without the system.
public protocol CredentialIdentityStoring: Sendable {
    /// Whether the user turned Mosaic on as a credential provider.
    func isEnabled() async -> Bool
    func replaceAll(_ plan: IdentityPlan) async throws
    func removeAll() async throws
}

/// Keeps the identity store in step with the vault: run in the app after an unlock, after each sync and after
/// each edit or settings change.
public enum IdentityStoreSync {
    /// Passkey identities only with the passkey setting; password identities only with the opt-in
    /// (OD-M10-7, default off). With both off the plan is empty and the store is cleared.
    public static func plan(
        settings: MobileSettings, passkeys: [PasskeyIdentity], passwords: [PasswordIdentity]
    ) -> IdentityPlan {
        IdentityPlan(
            passkeys: settings.passkeyIdentities ? passkeys : [],
            passwords: settings.passwordIdentities ? passwords : [])
    }

    /// The record ID of an identity: the item it comes from (`<vault id>/<item id>`).
    public static func recordIdentifier(_ key: ItemKey) -> String {
        key.vaultId + "/" + key.itemId
    }

    /// Reads the plan from the core. A locked core or a core error gives nil and leaves the store as it is.
    public static func currentPlan(holder: CoreHolder) async -> IdentityPlan? {
        try? await holder.call { core in
            // A locked core reports the strict default settings, which would clear the store: only an unlocked
            // core says what the store should hold.
            guard case .unlocked = core.state() else { throw CodedError("locked") }
            let settings = core.settings()
            var passkeys: [PasskeyIdentity] = []
            var passwords: [PasswordIdentity] = []
            if settings.passkeyIdentities {
                passkeys = try core.passkeyIdentities()
            }
            if settings.passwordIdentities {
                passwords = try core.passwordIdentities()
            }
            return IdentityStoreSync.plan(settings: settings, passkeys: passkeys, passwords: passwords)
        }
    }

    /// Writes the plan to the store. Best effort: a failure is retried by the next sync or edit.
    public static func apply(_ plan: IdentityPlan, to store: any CredentialIdentityStoring) async {
        guard await store.isEnabled() else { return }
        do {
            if plan.isEmpty {
                try await store.removeAll()
            } else {
                try await store.replaceAll(plan)
            }
        } catch {
            // The system list stays as it was.
        }
    }

    public static func run(holder: CoreHolder, store: any CredentialIdentityStoring) async {
        guard let plan = await currentPlan(holder: holder) else { return }
        await apply(plan, to: store)
    }
}

/// `ASCredentialIdentityStore`. Passkeys carry no key material, only the site, user name, credential ID and
/// user handle.
public struct SystemIdentityStore: CredentialIdentityStoring {
    public init() {}

    public func isEnabled() async -> Bool {
        await withCheckedContinuation { continuation in
            ASCredentialIdentityStore.shared.getState { state in
                continuation.resume(returning: state.isEnabled)
            }
        }
    }

    public func replaceAll(_ plan: IdentityPlan) async throws {
        var identities: [any ASCredentialIdentity] = []
        for passkey in plan.passkeys {
            identities.append(
                ASPasskeyCredentialIdentity(
                    relyingPartyIdentifier: passkey.rpId, userName: passkey.userName,
                    credentialID: passkey.credentialId, userHandle: passkey.userHandle,
                    recordIdentifier: IdentityStoreSync.recordIdentifier(passkey.record)))
        }
        for password in plan.passwords {
            identities.append(
                ASPasswordCredentialIdentity(
                    serviceIdentifier: ASCredentialServiceIdentifier(identifier: password.domain, type: .domain),
                    user: password.userName,
                    recordIdentifier: IdentityStoreSync.recordIdentifier(password.record)))
        }
        let entries = identities
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            ASCredentialIdentityStore.shared.replaceCredentialIdentities(entries) { _, error in
                if let error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            }
        }
    }

    public func removeAll() async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            ASCredentialIdentityStore.shared.removeAllCredentialIdentities { _, error in
                if let error {
                    continuation.resume(throwing: error)
                } else {
                    continuation.resume()
                }
            }
        }
    }
}
