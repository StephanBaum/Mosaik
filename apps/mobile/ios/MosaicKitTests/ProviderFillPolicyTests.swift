import XCTest

@testable import MosaicKit

/// G2 M3: a QuickType selection never fills without the user's confirmation of the saved site.
final class ProviderFillPolicyTests: XCTestCase {
    private let record = ItemKey(vaultId: "v", itemId: "i")
    private let other = ItemKey(vaultId: "v", itemId: "j")

    func testNothingFillsWithoutTheConfirmStep() {
        XCTAssertNil(ProviderFillPolicy.loginToFill(record: record, matches: [record], confirmed: false))
        XCTAssertNil(ProviderFillPolicy.loginToFill(record: record, matches: [record, other], confirmed: false))
    }

    func testAConfirmedRequestFillsOnlyTheIdentitysLoginWhenTheCoreStillMatchesIt() {
        XCTAssertEqual(ProviderFillPolicy.loginToFill(record: record, matches: [other, record], confirmed: true), record)
        XCTAssertNil(ProviderFillPolicy.loginToFill(record: record, matches: [other], confirmed: true))
        XCTAssertNil(ProviderFillPolicy.loginToFill(record: nil, matches: [record], confirmed: true))
    }

    func testTheSavedSiteIsNamedByItsHost() {
        XCTAssertEqual(
            ProviderFillPolicy.savedSite(of: [ServiceIdentifier(kind: .domain, value: "github.com")]), "github.com")
        XCTAssertEqual(
            ProviderFillPolicy.savedSite(of: [ServiceIdentifier(kind: .url, value: "https://example.com/login?x=1")]),
            "example.com")
        XCTAssertNil(ProviderFillPolicy.savedSite(of: []))
    }
}
