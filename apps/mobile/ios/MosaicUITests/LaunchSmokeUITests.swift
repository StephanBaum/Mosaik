import XCTest

@MainActor
final class LaunchSmokeUITests: XCTestCase {
    func testLaunchShowsCoreVersion() {
        let app = XCUIApplication()
        app.launch()
        let version = app.staticTexts["core-version"]
        XCTAssertTrue(version.waitForExistence(timeout: 30))
        XCTAssertFalse(version.label.isEmpty)
    }
}
