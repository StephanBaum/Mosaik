package io.github.stephanbaum.mosaic

import androidx.test.platform.app.InstrumentationRegistry

/**
 * A test that makes an account before any screen is in the front would be locked by the app's own background
 * policy: an earlier test left the last activity stopped (`backgroundedAt` is set, the background delay is 0), and
 * the ticker that is still running from that test locks the new session at its next tick. The tests that create
 * an account in `setUp` mark the app as foreground first, the way starting an activity does (the lock policy is
 * the release one, unchanged). The next activity stop sets the background state again.
 */
object AppForeground {
    fun hold(app: MosaicApp) {
        InstrumentationRegistry.getInstrumentation().runOnMainSync { app.lockPolicy.onForeground() }
    }

    /** Runs [block]; a `Locked` from it gets the state the policy saw, so a failure says why (no secrets). */
    fun <T> explainLock(app: MosaicApp, block: () -> T): T =
        try {
            block()
        } catch (e: io.github.stephanbaum.mosaic.ffi.MobileException.Locked) {
            throw IllegalStateException(
                "locked during setUp: foreground=${app.foreground.value} epoch=${app.sessionEpoch.value}",
                e,
            )
        }
}
