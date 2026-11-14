import Foundation
import os

/// The memory preflight of CRYPTO_SPEC 6.9.4. A failed Argon2 allocation aborts the process, so the app
/// compares `required_unlock_memory_bytes()` with the memory the process may still use before it unlocks,
/// re-authenticates or enrols quick unlock.
public enum MemoryCheck {
    /// `os_proc_available_memory()`, or nil when the system reports none (then the check passes: the
    /// process has no budget to compare with, as on a Mac or a simulator).
    public static func availableBytes() -> UInt64? {
        let available = os_proc_available_memory()
        return available > 0 ? UInt64(available) : nil
    }

    /// Whether `available` covers `required`. Unknown values pass.
    public static func hasEnough(required: UInt64?, available: UInt64?) -> Bool {
        guard let required, let available else { return true }
        return available >= required
    }
}
