import XCTest

@testable import MosaicKit

/// G2 M1: every way a request can stop closes the store exactly once, and a host going away is one of them.
final class RequestLifecycleTests: XCTestCase {
    private final class Counter: @unchecked Sendable {
        private let lock = NSLock()
        private var value = 0

        func bump() {
            lock.lock()
            defer { lock.unlock() }
            value += 1
        }

        var count: Int {
            lock.lock()
            defer { lock.unlock() }
            return value
        }
    }

    func testEndRunsTheCloseExactlyOnce() {
        let closed = Counter()
        let lifecycle = RequestLifecycle { closed.bump() }
        XCTAssertFalse(lifecycle.isFinished)
        XCTAssertTrue(lifecycle.end())
        XCTAssertFalse(lifecycle.end())
        XCTAssertFalse(lifecycle.end())
        XCTAssertTrue(lifecycle.isFinished)
        XCTAssertEqual(closed.count, 1)
    }

    func testEveryHostNotificationEndsTheRequestOnce() {
        let names = DismissalObserver.hostNotifications
        XCTAssertEqual(names.count, 3)
        for name in names {
            let center = NotificationCenter()
            let closed = Counter()
            let lifecycle = RequestLifecycle { closed.bump() }
            let observer = DismissalObserver(names: names, center: center) { _ = lifecycle.end() }
            center.post(name: name, object: nil)
            XCTAssertTrue(lifecycle.isFinished, "\(name.rawValue) must end the request")
            // The same notification again, and the others, do not close twice.
            center.post(name: name, object: nil)
            for other in names {
                center.post(name: other, object: nil)
            }
            XCTAssertEqual(closed.count, 1, name.rawValue)
            withExtendedLifetime(observer) {}
        }
    }

    func testObserverStopsWhenItIsReleased() {
        let center = NotificationCenter()
        let name = Notification.Name("test.dismiss")
        let closed = Counter()
        var observer: DismissalObserver? = DismissalObserver(names: [name], center: center) { closed.bump() }
        center.post(name: name, object: nil)
        XCTAssertEqual(closed.count, 1)
        observer = nil
        center.post(name: name, object: nil)
        XCTAssertEqual(closed.count, 1)
        XCTAssertNil(observer)
    }

    /// The request that ended by a dismissal closes the real session: the store is closed and nothing fills.
    func testDismissalClosesTheSessionsStore() async throws {
        let phone = try TestPhone(for: self)
        try phone.seed()
        let holder = try phone.extensionHolder()
        let session = ProviderSession(holder: holder, keys: phone.keys)
        _ = try await session.begin()
        try await session.unlockQuick(reason: "test", cancelTitle: "Cancel")

        let center = NotificationCenter()
        let lifecycle = RequestLifecycle { session.finish() }
        let observer = DismissalObserver(names: [.NSExtensionHostDidEnterBackground], center: center) {
            _ = lifecycle.end()
        }
        center.post(name: .NSExtensionHostDidEnterBackground, object: nil)
        withExtendedLifetime(observer) {}
        guard case .storeClosed = holder.state() else {
            return XCTFail("the store must be closed after a dismissal, got \(holder.state())")
        }
    }
}
