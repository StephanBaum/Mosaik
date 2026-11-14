import Foundation

/// The store lifecycle of the iOS app (spec 3.8, ADR-0094).
///
/// An app that is suspended while it holds a file lock in an App Group container is killed by the system
/// (`0xdead10cc`). So the app drops the store when its scene goes to the background (`closeStore()` locks
/// first and keeps a pending Safari join, so the app never calls `lock()` there) and opens it again when the
/// scene is active.
public struct StoreLifecycle: Sendable {
    private let holder: CoreHolder

    public init(holder: CoreHolder) {
        self.holder = holder
    }

    /// The scene went to the background: lock, wipe the revealed values and release the store's file lock.
    public func sceneDidEnterBackground() {
        holder.closeStore()
    }

    /// The scene is active: open the store again if it was closed. Returns the state.
    public func sceneDidBecomeActive() async throws -> SessionState {
        try await holder.call { core in try core.reopenStore() }
    }
}

/// The idle lock: calls `onExpire` when the user has not touched the app for the configured time.
@MainActor
public final class IdleLock {
    private var minutes = 0
    private var task: Task<Void, Never>?
    private let onExpire: @MainActor () -> Void

    public init(onExpire: @escaping @MainActor () -> Void) {
        self.onExpire = onExpire
    }

    /// Starts counting (again) with a timeout in minutes. Zero or less stops it.
    public func start(minutes: Int) {
        self.minutes = minutes
        restart()
    }

    /// The user touched the app: the countdown starts over.
    public func touch() {
        guard task != nil else { return }
        restart()
    }

    public func stop() {
        task?.cancel()
        task = nil
    }

    private func restart() {
        task?.cancel()
        task = nil
        guard minutes > 0 else { return }
        let seconds = minutes * 60
        task = Task { [weak self] in
            try? await Task.sleep(for: .seconds(seconds))
            guard !Task.isCancelled else { return }
            self?.expired()
        }
    }

    private func expired() {
        task = nil
        onExpire()
    }
}
