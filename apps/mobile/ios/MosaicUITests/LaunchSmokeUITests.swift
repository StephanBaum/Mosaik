import XCTest

@MainActor
final class LaunchSmokeUITests: XCTestCase {
    func testLaunchShowsCoreVersion() {
        let app = XCUIApplication()
        // The simulator has no passcode; Debug builds honor this argument (DeviceLock).
        app.launchArguments += ["-mosaic-assume-passcode"]
        app.launch()
        let version = app.staticTexts["core-version"]
        XCTAssertTrue(version.waitForExistence(timeout: 30))
        XCTAssertFalse(version.label.isEmpty)
    }
}
