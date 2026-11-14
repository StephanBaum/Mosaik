import XCTest

@testable import MosaicKit

@MainActor
final class CaptureObserverTests: XCTestCase {
    func testRevealDisabledWhileCaptured() {
        let observer = CaptureObserver()
        XCTAssertTrue(observer.revealAllowed)
        XCTAssertNil(observer.hiddenNote)

        observer.update(isCaptured: true)
        XCTAssertFalse(observer.revealAllowed)
        XCTAssertEqual(observer.hiddenNote, L10n.captureHidden())

        observer.update(isCaptured: false)
        XCTAssertTrue(observer.revealAllowed)

        // A screenshot only tells the screens to hide what they reveal.
        observer.screenshotTaken()
        XCTAssertEqual(observer.screenshotCount, 1)
    }
}
