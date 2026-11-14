import Darwin
import XCTest

@testable import MosaicKit

/// The extension's memory limit (about 120 MiB, spec 3.8 and 9) against a vault of 5 000 items.
final class MemoryTests: XCTestCase {
    private static let itemCount = 5000
    private static let limit: Int = 60 * 1024 * 1024

    /// Bytes the allocator has handed out in this process (Rust uses the system allocator, so the core's
    /// decrypted items count).
    private func heapInUse() -> Int {
        var statistics = malloc_statistics_t()
        malloc_zone_statistics(nil, &statistics)
        return Int(statistics.size_in_use)
    }

    /// One run of the extension: a new core in the extension context, quick unlock, and the list of one site.
    /// Returns the heap this request held while the vault was open and listed.
    private func oneRequest(_ phone: TestPhone) throws -> Int {
        let before = heapInUse()
        let core = try phone.openCore(.iosAutofillExtension)
        guard case let .locked(_, _, availability) = core.state(), case let .available(_, alias, _, _) = availability
        else {
            XCTFail("quick unlock should be available, got \(core.state())")
            return 0
        }
        var w = try XCTUnwrap(phone.keys.secret(for: alias))
        _ = try core.unlockQuick(w: w)
        w.wipe()
        let target = try core.fillTargetIos(
            identifier: ServiceIdentifier(kind: .domain, value: "site2500.example.com"))
        let found = try core.fillCandidates(target: target)
        XCTAssertEqual(found.candidates.count, 1)
        let filter = ItemFilter(vaultId: nil, trash: .exclude, favoritesOnly: false, kind: .login)
        let items = try core.items(filter: filter)
        XCTAssertEqual(items.count, Self.itemCount)
        let used = heapInUse() - before
        core.fillRelease(sessionId: found.sessionId)
        core.closeStore()
        return used
    }

    func testUnlock5000ItemsUnder60MiB() throws {
        let phone = try TestPhone(for: self)
        try phone.seed { core, _ in
            for index in 0..<Self.itemCount {
                _ = try phone.login(
                    core, title: "Site \(index)", username: "user\(index)@example.com",
                    password: "pw-\(index)-abcdefghijklmnop", url: "https://site\(index).example.com")
            }
        }

        let used = try oneRequest(phone)
        XCTAssertLessThan(used, Self.limit, "the extension held \(used / 1024 / 1024) MiB for \(Self.itemCount) items")

        // The same run under XCTMemoryMetric, so the result shows in the test report.
        let options = XCTMeasureOptions()
        options.iterationCount = 2
        measure(metrics: [XCTMemoryMetric()], options: options) {
            _ = try? oneRequest(phone)
        }
    }
}
