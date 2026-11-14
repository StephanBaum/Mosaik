import Foundation
import XCTest

@testable import MosaicKit

/// An in-memory `SecureStorage` for the tests of the core.
final class MemorySecureStorage: SecureStorage, @unchecked Sendable {
    private let lock = NSLock()
    private var values: [String: Data] = [:]

    func get(name: String) throws -> Data? {
        lock.lock()
        defer { lock.unlock() }
        return values[name]
    }

    func contains(name: String) throws -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return values[name] != nil
    }

    func put(name: String, value: Data) throws {
        lock.lock()
        defer { lock.unlock() }
        values[name] = value
    }

    func delete(name: String) throws {
        lock.lock()
        defer { lock.unlock() }
        values[name] = nil
    }
}

/// A secret handle that records that it was wiped.
final class FakeSecret: Wipeable, @unchecked Sendable {
    private let lock = NSLock()
    private var flag = false

    var wiped: Bool {
        lock.lock()
        defer { lock.unlock() }
        return flag
    }

    func wipe() {
        lock.lock()
        defer { lock.unlock() }
        flag = true
    }
}

enum TestSupport {
    /// A fresh store directory that is removed when the test ends.
    static func storeDirectory(for test: XCTestCase) throws -> URL {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("mosaic-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        test.addTeardownBlock { try? FileManager.default.removeItem(at: directory) }
        return directory
    }

    /// A core on `directory` with the given context, an in-memory secure store and this boot's ID.
    static func openCore(in directory: URL, context: CoreContext = .app) throws -> MosaicCore {
        try MosaicCore.open(
            config: CoreFactory.config(storeDirectory: directory, context: context),
            storage: MemorySecureStorage())
    }

    static func code(_ error: Error) -> String {
        ErrorText.code(of: error)
    }
}
