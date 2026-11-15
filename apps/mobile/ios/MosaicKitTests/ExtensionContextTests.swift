import XCTest

@testable import MosaicKit

final class ExtensionContextTests: XCTestCase {
    func testPasswordUnlockRefusedInExtension() throws {
        let directory = try TestSupport.storeDirectory(for: self)
        let core = try TestSupport.openCore(in: directory, context: .iosAutofillExtension)

        XCTAssertThrowsError(try core.unlockPassword(password: Data("password".utf8), secretKey: nil)) {
            XCTAssertEqual(TestSupport.code($0), "passwordInApp")
        }
        // Joining, syncing and the other App-only calls are refused before any work.
        XCTAssertThrowsError(try core.syncNow()) {
            XCTAssertEqual(TestSupport.code($0), "invalidInput")
        }
        XCTAssertThrowsError(try core.preloginCheck(serverUrl: "https://example.com")) {
            XCTAssertEqual(TestSupport.code($0), "invalidInput")
        }
        XCTAssertThrowsError(try core.recoverPassword(newPassword: Data("x".utf8))) {
            XCTAssertEqual(TestSupport.code($0), "invalidInput")
        }
    }
}
