import XCTest

@testable import MosaicKit

final class StoreLifecycleTests: XCTestCase {
    func testCloseOnBackgroundReleasesLock() async throws {
        let directory = try TestSupport.storeDirectory(for: self)
        let app = CoreHolder(core: try TestSupport.openCore(in: directory))
        let lifecycle = StoreLifecycle(holder: app)

        // While the app holds the store, another process (the extension) finds it busy.
        let extensionCore = try TestSupport.openCore(in: directory, context: .iosAutofillExtension)
        guard case .storeBusy = extensionCore.state() else {
            return XCTFail("expected a busy store, got \(extensionCore.state())")
        }

        // The scene goes to the background: the store and its file lock are released.
        lifecycle.sceneDidEnterBackground()
        guard case .storeClosed = app.state() else {
            return XCTFail("expected a closed store, got \(app.state())")
        }
        let opened = try extensionCore.reopenStore()
        guard case .noAccount = opened else {
            return XCTFail("the extension should open the store now, got \(opened)")
        }

        // The extension lets go, and the app opens it again when the scene is active.
        extensionCore.closeStore()
        let reopened = try await lifecycle.sceneDidBecomeActive()
        guard case .noAccount = reopened else {
            return XCTFail("the app should open the store again, got \(reopened)")
        }
    }
}
