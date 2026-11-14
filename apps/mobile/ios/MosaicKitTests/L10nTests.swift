import XCTest

@testable import MosaicKit

final class L10nTests: XCTestCase {
    func testEveryErrorCodeLocalized() {
        let codes = mobileErrorCodes()
        XCTAssertFalse(codes.isEmpty)
        for code in codes {
            XCTAssertNotNil(L10n.errorMessage(code: code), "no string for error code \(code)")
        }
        XCTAssertTrue(Set(codes).isSubset(of: Set(L10n.errorCodes)))
    }

    func testErrorTextUsesTheCodeOfTheCore() {
        XCTAssertEqual(ErrorText.code(of: MobileError.Locked), "locked")
        XCTAssertEqual(ErrorText.code(of: MobileError.PasswordInApp), "passwordInApp")
        XCTAssertEqual(ErrorText.code(of: CodedError("notEnoughMemory")), "notEnoughMemory")
        XCTAssertEqual(ErrorText.code(of: StorageError.Failed), "storageUnavailable")
        XCTAssertEqual(ErrorText.message(forCode: "notEnoughMemory"), L10n.errorNotEnoughMemory())
    }
}
