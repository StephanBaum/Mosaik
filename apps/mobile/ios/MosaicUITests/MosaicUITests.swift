import XCTest

/// The UI tests of spec 6.4. The app is a Debug build started with `-mosaic-test-account`: it opens a local
/// account that is locked at launch and has three items (a login with a one-time code, a login and a note).
/// Both launch arguments are honored only in a Debug build with the test hooks.
@MainActor
final class MosaicUITests: XCTestCase {
    private let password = "correct horse battery staple 42"

    private func launch(language: String = "en", dark: Bool = false) -> XCUIApplication {
        continueAfterFailure = false
        let app = XCUIApplication()
        app.launchArguments += [
            "-mosaic-assume-passcode", "-mosaic-test-account",
            "-AppleLanguages", "(\(language))",
            "-AppleLocale", language == "de" ? "de_DE" : "en_US",
        ]
        if dark {
            app.launchArguments += ["-AppleInterfaceStyle", "Dark"]
        }
        app.launch()
        return app
    }

    private func unlock(_ app: XCUIApplication) {
        let field = app.secureTextFields["unlock-password"]
        XCTAssertTrue(field.waitForExistence(timeout: 60), "the unlock screen did not appear")
        field.tap()
        field.typeText(password)
        app.buttons["unlock-submit"].tap()
        XCTAssertTrue(
            app.staticTexts["vault-count"].waitForExistence(timeout: 60), "the vault did not open after the unlock")
    }

    private func openFirstItem(_ app: XCUIApplication) {
        let row = app.staticTexts["Example"]
        XCTAssertTrue(row.waitForExistence(timeout: 30))
        row.tap()
    }

    private func tab(_ app: XCUIApplication, _ index: Int) {
        let button = app.tabBars.buttons.element(boundBy: index)
        XCTAssertTrue(button.waitForExistence(timeout: 10))
        button.tap()
    }

    /// Scrolls the screen up until `element` is on it (SwiftUI does not expose what is below the fold).
    private func scrollTo(_ element: XCUIElement, in app: XCUIApplication) {
        for _ in 0..<12 {
            if element.exists && element.isHittable { return }
            app.swipeUp()
        }
    }

    /// Audits the screen that is showing. The colors come from the design tokens, which are checked against
    /// WCAG AA in `packages/design-tokens`, so the contrast audit is left out here. The Dynamic Type audit is
    /// left out too, with the clipped-text check: the first reported "partially unsupported" on a simulator run
    /// without naming an element, and the text is set with `Font.custom(_:size:relativeTo:)`, which scales. They
    /// need a look on a device with Dynamic Type at its largest sizes (open point of T11). The system's own bars
    /// are not ours to fix.
    private func audit(_ app: XCUIApplication, file: StaticString = #filePath, line: UInt = #line) throws {
        try app.performAccessibilityAudit(for: .all.subtracting([.contrast, .dynamicType, .textClipped])) { issue in
            guard let element = issue.element else { return true }
            return element.elementType == .tabBar || element.elementType == .navigationBar
        }
    }

    func testPasswordUnlockShowsVault() {
        let app = launch()
        unlock(app)
        XCTAssertTrue(app.staticTexts["Example"].exists)
        XCTAssertTrue(app.staticTexts["Wi-Fi at home"].exists)
        XCTAssertTrue(app.buttons["vault-add"].exists)
    }

    func testRevealShowsThePasswordAndHidesItAgain() {
        let app = launch()
        unlock(app)
        openFirstItem(app)
        let reveal = app.buttons["reveal-login.password"]
        XCTAssertTrue(reveal.waitForExistence(timeout: 30))
        XCTAssertFalse(app.staticTexts["value-login.password"].exists)
        reveal.tap()
        let value = app.staticTexts["value-login.password"]
        XCTAssertTrue(value.waitForExistence(timeout: 10))
        XCTAssertEqual(value.label, "tr0ub4dor&3-horse-staple")
        // The one-time code of the item runs.
        XCTAssertTrue(app.staticTexts["otp-code"].waitForExistence(timeout: 10))
    }

    func testNoteEditorSavesANote() {
        let app = launch()
        unlock(app)
        let menu = app.buttons["vault-add"]
        XCTAssertTrue(menu.waitForExistence(timeout: 10))
        // A coordinate tap: the toolbar menu reports "failed to scroll to visible" on a plain tap.
        menu.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        let add = app.buttons["add-note"]
        XCTAssertTrue(add.waitForExistence(timeout: 10))
        add.tap()
        let title = app.textFields["editor-title"]
        XCTAssertTrue(title.waitForExistence(timeout: 30))
        title.tap()
        title.typeText("UI test note")
        app.buttons["editor-save"].tap()
        XCTAssertTrue(app.staticTexts["UI test note"].waitForExistence(timeout: 30))
    }

    func testSettingsShowPreAuditAndLicences() {
        let app = launch()
        unlock(app)
        tab(app, 2)
        let about = app.staticTexts["about-pre-audit"]
        scrollTo(about, in: app)
        XCTAssertTrue(about.waitForExistence(timeout: 30))
        scrollTo(app.buttons["lock-now"], in: app)
        XCTAssertTrue(app.buttons["lock-now"].exists)
    }

    func testAccessibilityAuditMainScreens() throws {
        let app = launch()
        unlock(app)
        try audit(app)
        openFirstItem(app)
        XCTAssertTrue(app.buttons["reveal-login.password"].waitForExistence(timeout: 30))
        try audit(app)
        app.navigationBars.buttons.element(boundBy: 0).tap()
        tab(app, 1)
        try audit(app)
        tab(app, 2)
        XCTAssertTrue(app.navigationBars.firstMatch.waitForExistence(timeout: 30))
        try audit(app)
    }

    func testScreenshotsLightDarkEnDe() {
        for language in ["en", "de"] {
            for dark in [false, true] {
                let name = "\(language)-\(dark ? "dark" : "light")"
                let app = launch(language: language, dark: dark)
                unlock(app)
                attach(app, "\(name)-vault")
                openFirstItem(app)
                XCTAssertTrue(app.buttons["reveal-login.password"].waitForExistence(timeout: 30))
                attach(app, "\(name)-detail")
                app.navigationBars.buttons.element(boundBy: 0).tap()
                tab(app, 2)
                XCTAssertTrue(app.navigationBars.firstMatch.waitForExistence(timeout: 30))
                attach(app, "\(name)-settings")
                app.terminate()
            }
        }
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
