import Foundation

/// Owns the one `MosaicCore` of the process and runs every call on a serial queue (spec 3.6).
///
/// The core is blocking (Argon2id, OPAQUE and HTTP), so no call runs on the actor or a cooperative
/// thread: `call` hops to the serial queue and resumes the caller with the result. `lock()` and
/// `state()` bypass the queue: they are cheap and must preempt a long call.
public actor CoreHolder {
    private let core: MosaicCore
    private let queue = DispatchQueue(label: "io.github.stephanbaum.mosaic.core", qos: .userInitiated)

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
        core.lock()
    }

    /// The session state, without waiting for a running call.
    public nonisolated func state() -> SessionState {
        core.state()
    }
}
