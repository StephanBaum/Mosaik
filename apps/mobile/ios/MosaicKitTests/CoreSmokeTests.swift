import XCTest

@testable import MosaicKit

final class CoreSmokeTests: XCTestCase {
    func testCoreVersionIsNotEmpty() {
        XCTAssertFalse(coreVersion().isEmpty)
    }

    func testErrorCodesAreExposed() {
        XCTAssertFalse(mobileErrorCodes().isEmpty)
    }
}
