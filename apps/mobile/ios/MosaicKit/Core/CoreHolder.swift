import Foundation

/// A secret handle that can be emptied: the core's `SecretValue`.
public protocol Wipeable: AnyObject, Sendable {
    func wipe()
}

extension SecretValue: Wipeable {}

/// The revealed values the UI holds. The view models register a handle when they show a value; a lock or a
/// closed store wipes them all, so no revealed value outlives the session (spec 3.8).
public final class RevealRegistry: @unchecked Sendable {
    private let lock = NSLock()
    private var handles: [ObjectIdentifier: any Wipeable] = [:]

    public init() {}

    public var count: Int {
        lock.lock()
        defer { lock.unlock() }
        return handles.count
    }

    public func register(_ handle: any Wipeable) {
        lock.lock()
        defer { lock.unlock() }
        handles[ObjectIdentifier(handle)] = handle
    }

    /// Wipes `handle` and forgets it (the user hid the value).
    public func release(_ handle: any Wipeable) {
        lock.lock()
        let removed = handles.removeValue(forKey: ObjectIdentifier(handle))
        lock.unlock()
        removed?.wipe()
    }

    public func wipeAll() {
        lock.lock()
        let all = Array(handles.values)
        handles.removeAll()
        lock.unlock()
        for handle in all {
            handle.wipe()
        }
    }
}

/// Owns the one `MosaicCore` of the process and runs every call on a serial queue (spec 3.6).
///
/// The core is blocking (Argon2id, OPAQUE and HTTP), so no call runs on the actor or a cooperative
/// thread: `call` hops to the serial queue and resumes the caller with the result. `lock()`, `closeStore()`
/// and `state()` bypass the queue: they are cheap and must preempt a long call. Both wipe the revealed
/// values first.
public actor CoreHolder {
    private let core: MosaicCore
    private let queue = DispatchQueue(label: "io.github.stephanbaum.mosaic.core", qos: .userInitiated)
    /// The revealed values of the UI, wiped by every lock.
    public nonisolated let reveals = RevealRegistry()

    public init(core: MosaicCore) {
        self.core = core
    }

    /// Runs `body` with the core on the serial queue.
    public func call<T: Sendable>(_ body: @escaping @Sendable (MosaicCore) throws -> T) async throws -> T {
        let core = self.core
        let queue = self.queue
        return try await withCheckedThrowingContinuation { continuation in
            queue.async {
                continuation.resume(with: Result { try body(core) })
            }
        }
    }

    /// Locks the core now, without waiting for a running call.
    public nonisolated func lock() {
        reveals.wipeAll()
        core.lock()
    }

    /// Locks and drops the store with its file lock (the app in the background, the extension at its end),
    /// without waiting for a running call. A pending join survives it (the Safari handoff, ADR-0091).
    public nonisolated func closeStore() {
        reveals.wipeAll()
        core.closeStore()
    }

    /// The session state, without waiting for a running call.
    public nonisolated func state() -> SessionState {
        core.state()
    }

    /// Forgets a fill session the request will not use any more, without waiting for a running call. An
    /// unknown session or a locked core is ignored (`fill_release`).
    public nonisolated func releaseFillSession(_ id: String) {
        core.fillRelease(sessionId: id)
    }
}
