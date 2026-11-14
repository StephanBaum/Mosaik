import Foundation

/// Turns any error of the apps into the string of the shared catalog (`error.<code>`).
public enum ErrorText {
    /// The camelCase code of `error`: the core's `MobileError` case name, a `CodedError`, a storage failure,
    /// or `internal` for anything else. The core's errors carry no message, ID or value.
    public static func code(of error: Error) -> String {
        if let coded = error as? CodedError {
            return coded.code
        }
        if let mobile = error as? MobileError {
            // UniFFI names error cases in PascalCase (`PasswordInApp`); the catalog codes are camelCase.
            let name = String(describing: mobile)
            return name.prefix(1).lowercased() + name.dropFirst()
        }
        if error is StorageError {
            return "storageUnavailable"
        }
        if let key = error as? QuickUnlockKeyError {
            return key == .invalidated ? "quickUnlockInvalidated" : "passwordRequired"
        }
        return "internal"
    }

    public static func message(for error: Error) -> String {
        message(forCode: code(of: error))
    }

    public static func message(forCode code: String) -> String {
        L10n.errorMessage(code: code) ?? L10n.errorInternal()
    }
}
