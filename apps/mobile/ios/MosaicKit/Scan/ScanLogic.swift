import AVFoundation
import Foundation

/// What a scan screen looks for.
public enum ScanTarget: Sendable, Equatable {
    /// The desktop's setup code (`mosaic-setup:1:`), in onboarding.
    case setupCode
    /// An `otpauth://` code, in the login editor.
    case otp
}

/// Where the camera permission stands.
public enum CameraAccess: Sendable, Equatable {
    /// Not asked yet: explain first, then ask.
    case ask
    /// Allowed.
    case granted
    /// Asked and refused (or restricted by the device).
    case denied
    /// This device has no camera, or it could not be started.
    case unavailable

    /// The state from what the system says. A device without a camera is `unavailable` whatever the status.
    public static func from(status: AVAuthorizationStatus, hasCamera: Bool) -> CameraAccess {
        guard hasCamera else { return .unavailable }
        switch status {
        case .notDetermined: return .ask
        case .authorized: return .granted
        case .denied, .restricted: return .denied
        @unknown default: return .denied
        }
    }

    /// The state of this device now.
    public static func current() -> CameraAccess {
        from(
            status: AVCaptureDevice.authorizationStatus(for: .video),
            hasCamera: AVCaptureDevice.default(for: .video) != nil)
    }
}

/// What the scan screen says about the last code it saw.
public enum ScanFeedback: Sendable, Equatable {
    /// Nothing seen yet.
    case looking
    /// A QR code of something else. Shown at once, and the code is dropped unread.
    case notSetupCode
    /// The core refused the code; `code` is the error code of the shared catalog.
    case error(String)
}

/// Decides whether a scanned QR payload is for the core at all. A payload that is not goes nowhere: the core
/// never sees a stranger's QR code (spec 3.14).
public enum ScanPayload {
    public enum Verdict: Sendable, Equatable {
        /// Worth handing to the core.
        case candidate
        /// Not what this screen looks for.
        case foreign
    }

    /// The longest payload that is looked at. The setup code is at most 1,024 bytes.
    public static let maxLength = 4096

    public static func verdict(for payload: String, target: ScanTarget) -> Verdict {
        guard payload.utf8.count <= maxLength else { return .foreign }
        switch target {
        case .setupCode:
            return payload.hasPrefix("mosaic-setup:") ? .candidate : .foreign
        case .otp:
            let lower = payload.lowercased()
            return lower.hasPrefix("otpauth://") || lower.hasPrefix("otpauth-migration://")
                ? .candidate : .foreign
        }
    }
}
