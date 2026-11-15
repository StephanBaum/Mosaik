import AuthenticationServices
import XCTest

@testable import MosaicKit

final class ServiceIdentifierTests: XCTestCase {
    func testDomainAndUrlTargets() throws {
        let mapped = ProviderIdentifiers.map([
            ASCredentialServiceIdentifier(identifier: "github.com", type: .domain),
            ASCredentialServiceIdentifier(identifier: "https://login.example.com:8443/path", type: .URL),
        ])
        XCTAssertEqual(mapped.count, 2)
        XCTAssertEqual(mapped[0].kind, .domain)
        XCTAssertEqual(mapped[0].value, "github.com")
        XCTAssertEqual(mapped[1].kind, .url)
        XCTAssertEqual(mapped[1].value, "https://login.example.com:8443/path")
        XCTAssertTrue(ProviderIdentifiers.map([]).isEmpty)

        // The core turns them into origins: a domain is https, a URL keeps scheme, host and port.
        let directory = try TestSupport.storeDirectory(for: self)
        let core = try TestSupport.openCore(in: directory, context: .iosAutofillExtension)
        let domain = try core.fillTargetIos(identifier: mapped[0])
        guard case let .web(origin) = domain.kind else { return XCTFail("a domain is a web target") }
        XCTAssertEqual(origin, "https://github.com")
        let url = try core.fillTargetIos(identifier: mapped[1])
        guard case let .web(urlOrigin) = url.kind else { return XCTFail("a URL is a web target") }
        XCTAssertEqual(urlOrigin, "https://login.example.com:8443")
    }
}
