import Foundation

/// Quick unlock enrolment and unlock, the platform half (spec 3.10). The core decides every policy; this
/// code only runs the Secure Enclave key and carries `W` between the key and the core.
public enum QuickUnlockFlow {
    /// Enrols quick unlock right after a master-password unlock, with the same typed password bytes
    /// (spec 3.10, "Enrolment flow"). Runs on the core's serial queue, inside the call that unlocked.
    ///
    /// Returns whether quick unlock is on afterwards. Every failure leaves the vault unlocked and quick unlock
    /// off, and removes a key that was created but not recorded.
    public static func enroll(core: MosaicCore, password: Data, keys: any QuickUnlockKeys) -> Bool {
        guard keys.isAvailable else { return false }
        guard let setup = try? core.quickUnlockEnrollBegin(password: password) else { return false }
        var w = setup.w.exposeBytes()
        defer {
            w.wipe()
            setup.w.wipe()
        }
        guard let body = try? keys.enroll(alias: setup.keyAlias, secret: w) else { return false }
        guard
            let enrolled = try? core.quickUnlockEnrollFinish(platform: .iosSecureEnclave, body: body)
        else {
            try? keys.deleteKey(alias: setup.keyAlias)
            return false
        }
        if let replaced = enrolled.replacedKeyAlias {
            try? keys.deleteKey(alias: replaced)
        }
        return true
    }

    /// Opens the vault with the key of the current enrolment: one biometric prompt, then `unlock_quick(W)`.
    ///
    /// A key the system invalidated (biometry changed) tells the core, which deletes the wrapping, and the
    /// platform deletes the key. The caller shows the password screen.
    public static func unlock(
        holder: CoreHolder, keys: any QuickUnlockKeys, reason: String, cancelTitle: String
    ) async throws {
        guard
            case let .locked(_, _, availability) = holder.state(),
            case let .available(_, alias, body, _) = availability
        else {
            throw CodedError("passwordRequired")
        }
        var w: Data
        do {
            w = try await keys.decrypt(alias: alias, ciphertext: body, reason: reason, cancelTitle: cancelTitle)
        } catch let failure as QuickUnlockKeyError {
            if failure == .invalidated {
                if let old = try? await holder.call({ core in try core.quickUnlockPlatformFailed() }) {
                    try? keys.deleteKey(alias: old)
                }
                throw CodedError("quickUnlockInvalidated")
            }
            throw failure
        }
        let secret = SecretBuffer(taking: &w)
        defer { secret.wipe() }
        try await holder.call { core in
            var bytes = secret.copyOut()
            defer { bytes.wipe() }
            _ = try core.unlockQuick(w: bytes)
        }
    }
}
