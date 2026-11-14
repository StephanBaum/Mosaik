import AVFoundation
import XCTest

@testable import MosaicKit

final class ScanLogicTests: XCTestCase {
    func testCameraAccessFollowsTheSystemStatus() {
        XCTAssertEqual(CameraAccess.from(status: .notDetermined, hasCamera: true), .ask)
        XCTAssertEqual(CameraAccess.from(status: .authorized, hasCamera: true), .granted)
        XCTAssertEqual(CameraAccess.from(status: .denied, hasCamera: true), .denied)
        XCTAssertEqual(CameraAccess.from(status: .restricted, hasCamera: true), .denied)
        XCTAssertEqual(CameraAccess.from(status: .authorized, hasCamera: false), .unavailable)
        XCTAssertEqual(CameraAccess.from(status: .notDetermined, hasCamera: false), .unavailable)
    }

    func testSetupCodeScannerDropsEverythingElse() {
        XCTAssertEqual(ScanPayload.verdict(for: "mosaic-setup:1:eyJ2IjoxfQ", target: .setupCode), .candidate)
        XCTAssertEqual(ScanPayload.verdict(for: "https://example.com", target: .setupCode), .foreign)
        XCTAssertEqual(ScanPayload.verdict(for: "otpauth://totp/a?secret=ABC", target: .setupCode), .foreign)
        XCTAssertEqual(ScanPayload.verdict(for: "", target: .setupCode), .foreign)
        XCTAssertEqual(
            ScanPayload.verdict(for: "mosaic-setup:1:" + String(repeating: "A", count: 5000), target: .setupCode),
            .foreign)
    }

    func testOtpScannerTakesOtpauthLinksOnly() {
        XCTAssertEqual(ScanPayload.verdict(for: "otpauth://totp/a?secret=ABC", target: .otp), .candidate)
        XCTAssertEqual(ScanPayload.verdict(for: "OTPAUTH://totp/a?secret=ABC", target: .otp), .candidate)
        XCTAssertEqual(ScanPayload.verdict(for: "otpauth-migration://offline?data=abc", target: .otp), .candidate)
        XCTAssertEqual(ScanPayload.verdict(for: "mosaic-setup:1:abc", target: .otp), .foreign)
        XCTAssertEqual(ScanPayload.verdict(for: "https://example.com", target: .otp), .foreign)
    }
}
