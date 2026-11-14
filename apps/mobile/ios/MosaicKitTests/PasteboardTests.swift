import UIKit
import UniformTypeIdentifiers
import XCTest

@testable import MosaicKit

final class PasteboardTests: XCTestCase {
    func testLocalOnlyAndExpiryOptions() {
        let now = Date(timeIntervalSince1970: 1_000)
        let options = PasteboardOptions.options(expiringAfter: 30, now: now)
        XCTAssertEqual(options[.localOnly] as? Bool, true)
        XCTAssertEqual(options[.expirationDate] as? Date, now.addingTimeInterval(30))
        XCTAssertEqual(options.count, 2)

        let items = PasteboardOptions.items(for: "value")
        XCTAssertEqual(items.count, 1)
        XCTAssertEqual(Array(items[0].keys), [UTType.utf8PlainText.identifier])
        XCTAssertEqual(items[0][UTType.utf8PlainText.identifier] as? String, "value")
    }
}
