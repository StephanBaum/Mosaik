import XCTest

@testable import MosaicKit

final class BootIdTests: XCTestCase {
    func testStableWithinBootAnd16Bytes() {
        let first = BootId.current()
        let second = BootId.current()
        XCTAssertEqual(first.count, 16)
        XCTAssertEqual(first, second)
        // big-endian u64 seconds, then u64 microseconds
        XCTAssertEqual(
            BootId.encode(seconds: 0x0102_0304_0506_0708, microseconds: 0x1112_1314_1516_1718),
            Data([1, 2, 3, 4, 5, 6, 7, 8, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]))
    }
}
