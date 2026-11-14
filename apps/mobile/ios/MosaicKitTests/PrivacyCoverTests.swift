import XCTest

@testable import MosaicKit

@MainActor
final class PrivacyCoverTests: XCTestCase {
    func testCoverOnResignActive() {
        let model = PrivacyCoverModel()
        XCTAssertFalse(model.isCovered)
        model.sceneWillResignActive()
        XCTAssertTrue(model.isCovered)
        model.sceneDidBecomeActive()
        XCTAssertFalse(model.isCovered)
    }
}
