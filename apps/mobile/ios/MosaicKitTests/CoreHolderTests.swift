import XCTest

@testable import MosaicKit

final class CoreHolderTests: XCTestCase {
    func testLockPreemptsAndWipesReveals() async throws {
        let directory = try TestSupport.storeDirectory(for: self)
        let holder = CoreHolder(core: try TestSupport.openCore(in: directory))
        let revealed = FakeSecret()
        holder.reveals.register(revealed)
        XCTAssertEqual(holder.reveals.count, 1)

        // A call that blocks the serial queue.
        let started = DispatchSemaphore(value: 0)
        let release = DispatchSemaphore(value: 0)
        let long = Task {
            try await holder.call { _ in
                started.signal()
                release.wait()
            }
        }
        await withCheckedContinuation { continuation in
            DispatchQueue.global().async {
                started.wait()
                continuation.resume()
            }
        }

        // The lock does not wait for it, and it wipes what is revealed.
        holder.lock()
        XCTAssertTrue(revealed.wiped)
        XCTAssertEqual(holder.reveals.count, 0)

        // The queue still runs in order: a second call waits for the first.
        release.signal()
        try await long.value
        let state = try await holder.call { core in core.state() }
        guard case .noAccount = state else {
            return XCTFail("expected no account, got \(state)")
        }
    }

    func testRevealRegistryReleaseWipesOnlyThatHandle() {
        let registry = RevealRegistry()
        let one = FakeSecret()
        let two = FakeSecret()
        registry.register(one)
        registry.register(two)
        registry.release(one)
        XCTAssertTrue(one.wiped)
        XCTAssertFalse(two.wiped)
        XCTAssertEqual(registry.count, 1)
        registry.wipeAll()
        XCTAssertTrue(two.wiped)
    }

    func testSecretBufferWipesItsBytes() {
        let buffer = SecretBuffer()
        buffer.set(from: "pässword")
        var copy = buffer.copyOut()
        XCTAssertEqual(String(decoding: copy, as: UTF8.self), "pässword")
        copy.wipe()
        XCTAssertTrue(copy.isEmpty)
        buffer.wipe()
        XCTAssertTrue(buffer.isEmpty)
        XCTAssertTrue(buffer.copyOut().isEmpty)
    }
}
