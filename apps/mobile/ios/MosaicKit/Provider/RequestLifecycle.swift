import Foundation
import UIKit

/// The end of one AutoFill request, run exactly once (G2 M1). Every way a request can stop (the user's
/// choice, a failure, a dismissal, the host going to the background, the idle timer, a newer request) goes
/// through `end()`, which locks the vault, closes the store and releases the fill session. Thread-safe:
/// the notifications can arrive off the main actor.
public final class RequestLifecycle: @unchecked Sendable {
    private let lock = NSLock()
    private var done = false
    private var told = false
    private let close: @Sendable () -> Void

    /// `close` is `ProviderSession.finish` (it is idempotent too; this guard makes the count one).
    public init(close: @escaping @Sendable () -> Void) {
        self.close = close
    }

    public var isFinished: Bool {
        lock.lock()
        defer { lock.unlock() }
        return done
    }

    /// Runs `close` the first time and returns true; later calls do nothing and return false.
    @discardableResult
    public func end() -> Bool {
        lock.lock()
        let first = !done
        done = true
        lock.unlock()
        if first {
            close()
        }
        return first
    }

    /// True once, when the request has ended and the system has not been told yet (G3i-4). A request that
    /// ended silently (a dismissal, the host going away) leaves a sheet the system still shows; the next user
    /// action or the host becoming active again tells it, exactly once.
    public func claimTelling() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard done, !told else { return false }
        told = true
        return true
    }

    /// The system needs no telling (a newer request replaced this one and owns the sheet now).
    public func markTold() {
        lock.lock()
        told = true
        lock.unlock()
    }
}

/// Calls `onDismiss` when the extension's host goes away: the host resigns active or enters the background
/// (the user left the sheet, or the app behind it went), or protected data becomes unavailable (the phone
/// locked). The 120 second idle timer of the model is only the backstop for a request that nobody ends.
public final class DismissalObserver: @unchecked Sendable {
    /// The notifications that end a request.
    public static var hostNotifications: [Notification.Name] {
        [
            .NSExtensionHostWillResignActive, .NSExtensionHostDidEnterBackground,
            UIApplication.protectedDataWillBecomeUnavailableNotification,
        ]
    }

    private let center: NotificationCenter
    private let lock = NSLock()
    private var tokens: [any NSObjectProtocol] = []

    /// `queue` nil delivers on the posting thread.
    public init(
        names: [Notification.Name] = DismissalObserver.hostNotifications, center: NotificationCenter = .default,
        queue: OperationQueue? = nil, onDismiss: @escaping @Sendable () -> Void
    ) {
        self.center = center
        let added = names.map { name in
            center.addObserver(forName: name, object: nil, queue: queue) { _ in onDismiss() }
        }
        lock.lock()
        tokens = added
        lock.unlock()
    }

    deinit {
        for token in tokens {
            center.removeObserver(token)
        }
    }
}
