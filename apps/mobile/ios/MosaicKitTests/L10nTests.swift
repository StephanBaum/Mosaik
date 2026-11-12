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

    /// `InfoPlist.strings` is written by hand (the export does not make it), so it must say what the catalog
    /// says for the Face ID prompt, in both languages.
    func testFaceIdUsageDescriptionMatchesTheCatalog() throws {
        for language in ["en", "de"] {
            let folder = try XCTUnwrap(Bundle.main.path(forResource: language, ofType: "lproj"))
            let bundle = try XCTUnwrap(Bundle(path: folder))
            // Xcode compiles the table, so ask the bundle instead of opening the file.
            let faceId = bundle.localizedString(forKey: "NSFaceIDUsageDescription", value: "missing", table: "InfoPlist")
            let catalog = bundle.localizedString(forKey: "mobile.unlock.faceIdReason", value: nil, table: nil)
            XCTAssertEqual(faceId, catalog, language)
        }
    }
}
