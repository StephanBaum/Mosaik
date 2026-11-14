import Foundation
import LocalAuthentication

/// Mosaic stores no secret on a device without a passcode (ADR-0090 amendment): the secure-store Keychain
/// items need one, so onboarding and unlock ask the user to set one first.
public enum DeviceLock {
    /// The launch argument that stands in for a passcode in simulator tests. Debug builds only.
    public static let testAssumePasscodeArgument = "-mosaic-assume-passcode"

    /// Whether the device has a passcode (or other device-owner authentication).
    public static func isPasscodeSet() -> Bool {
        #if DEBUG
            if ProcessInfo.processInfo.arguments.contains(testAssumePasscodeArgument) {
                return true
            }
        #endif
        var error: NSError?
        return LAContext().canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)
    }

    /// The text of the "unlock with … next time" option: Touch ID on a Touch ID phone, Face ID otherwise.
    public static func quickUnlockOptionText() -> String {
        let context = LAContext()
        var error: NSError?
        _ = context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error)
        return context.biometryType == .touchID ? L10n.unlockQuickOptionTouchId() : L10n.unlockQuickOptionFaceId()
    }
}
