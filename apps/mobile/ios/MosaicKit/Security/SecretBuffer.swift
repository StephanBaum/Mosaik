import Foundation

extension Data {
    /// Overwrites every byte with zero (`memset_s`, which the optimizer must keep) and empties the data.
    /// The caller owns the only reference, otherwise a copy-on-write copy is zeroed instead.
    public mutating func wipe() {
        withUnsafeMutableBytes { raw in
            if let base = raw.baseAddress, raw.count > 0 {
                _ = memset_s(base, raw.count, 0, raw.count)
            }
        }
        removeAll(keepingCapacity: false)
    }
}

/// A byte buffer for a typed secret (CRYPTO_SPEC 6.9.5). It lives outside Swift's `String` and `Data`
/// storage, so `wipe()` zeroes the one allocation that holds the bytes. The secret field writes into it;
/// the core receives a copy from `copyOut()`, which the caller wipes right after the call.
public final class SecretBuffer: @unchecked Sendable {
    private let lock = NSLock()
    private var pointer: UnsafeMutablePointer<UInt8>?
    private var length = 0

    public init() {}

    /// Takes the bytes of `data` and wipes `data`.
    public init(taking data: inout Data) {
        let count = data.count
        if count > 0 {
            let allocation = UnsafeMutablePointer<UInt8>.allocate(capacity: count)
            data.withUnsafeBytes { raw in
                if let base = raw.baseAddress {
                    allocation.initialize(from: base.assumingMemoryBound(to: UInt8.self), count: count)
                }
            }
            pointer = allocation
            length = count
        }
        data.wipe()
    }

    deinit {
        wipe()
    }

    public var isEmpty: Bool {
        lock.lock()
        defer { lock.unlock() }
        return length == 0
    }

    public var count: Int {
        lock.lock()
        defer { lock.unlock() }
        return length
    }

    /// Replaces the content with the UTF-8 bytes of `text`.
    public func set(from text: String) {
        var text = text
        text.withUTF8 { utf8 in
            lock.lock()
            defer { lock.unlock() }
            clearLocked()
            guard utf8.count > 0, let base = utf8.baseAddress else { return }
            let allocation = UnsafeMutablePointer<UInt8>.allocate(capacity: utf8.count)
            allocation.initialize(from: base, count: utf8.count)
            pointer = allocation
            length = utf8.count
        }
    }

    /// A copy of the bytes. The caller wipes it (`Data.wipe()`) as soon as the call that needs it ends.
    public func copyOut() -> Data {
        lock.lock()
        defer { lock.unlock() }
        guard let pointer, length > 0 else { return Data() }
        return Data(bytes: pointer, count: length)
    }

    /// Zeroes and frees the bytes.
    public func wipe() {
        lock.lock()
        defer { lock.unlock() }
        clearLocked()
    }

    private func clearLocked() {
        if let pointer {
            _ = memset_s(pointer, length, 0, length)
            pointer.deallocate()
        }
        pointer = nil
        length = 0
    }
}
