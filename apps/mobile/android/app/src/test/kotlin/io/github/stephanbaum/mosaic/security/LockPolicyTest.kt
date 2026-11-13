package io.github.stephanbaum.mosaic.security

import io.github.stephanbaum.mosaic.ffi.SessionState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LockPolicyTest {
    private var now = 1_000_000L
    private var locks = 0
    private var unlocked = true
    private var joinPending = false
    private var settings = LockSettings()

    private fun policy() =
        LockPolicy(
            clock = { now },
            settings = { settings },
            hasSession = { unlocked },
            joinPending = { joinPending },
            lock = {
                locks += 1
                unlocked = false
            },
        )

    @Test
    fun locksImmediatelyOnBackgroundByDefault() {
        val policy = policy()
        policy.onBackground()
        assertEquals(1, locks)
    }

    @Test
    fun backgroundDelayKeepsTheSessionUntilItHasPassed() {
        settings = LockSettings(backgroundSeconds = 30)
        val policy = policy()
        policy.onBackground()
        now += 29_000
        policy.tick()
        assertEquals(0, locks)
        now += 1_000
        policy.tick()
        assertEquals(1, locks)
    }

    @Test
    fun returningAfterTheDelayLocksEvenWithoutATick() {
        settings = LockSettings(backgroundSeconds = 30)
        val policy = policy()
        policy.onBackground()
        now += 45_000
        policy.onForeground()
        assertEquals(1, locks)
    }

    @Test
    fun idleTimeoutLocks() {
        val policy = policy()
        now += 4 * 60_000L + 59_000L
        policy.tick()
        assertEquals(0, locks)
        // A touch restarts the idle time.
        policy.onUserInteraction()
        now += 4 * 60_000L + 59_000L
        policy.tick()
        assertEquals(0, locks)
        now += 1_000
        policy.tick()
        assertEquals(1, locks)
    }

    @Test
    fun aLockDuringBusyReachesTheCore() {
        // The core is unlocking (Busy) when the lock fires: it must be passed on, not skipped (M10-G2 M1).
        assertTrue(sessionNeedsLock(SessionState.Busy))
        // A store that is closed or in use by another process has no session: no ticker runs for it (N4).
        assertFalse(sessionNeedsLock(SessionState.StoreBusy))
        assertFalse(sessionNeedsLock(SessionState.StoreClosed))
        assertFalse(sessionNeedsLock(SessionState.NoAccount))
        assertFalse(sessionNeedsLock(null))
        val policy = policy()
        policy.onScreenOff()
        assertEquals(1, locks)
    }

    @Test
    fun screenOffLocks() {
        val policy = policy()
        policy.onScreenOff()
        assertEquals(1, locks)
    }

    @Test
    fun providerGraceEndsAfter60s() {
        val policy = policy()
        policy.openProviderFlow("fill")
        policy.onBackground()
        now += 59_999
        policy.tick()
        assertEquals(0, locks)
        now += 1
        policy.tick()
        assertEquals(1, locks)
    }

    @Test
    fun providerGraceEndsOnScreenOff() {
        val policy = policy()
        policy.openProviderFlow("fill")
        policy.onBackground()
        now += 10_000
        policy.onScreenOff()
        assertEquals(1, locks)
    }

    @Test
    fun closingTheProviderFlowAppliesTheBackgroundRule() {
        val policy = policy()
        policy.openProviderFlow("fill")
        policy.onBackground()
        now += 5_000
        policy.closeProviderFlow("fill")
        assertEquals(1, locks)
    }

    @Test
    fun aLockEndsTheProviderFlowsSoAStaleFlowGrantsNoLaterGrace() {
        val policy = policy()
        policy.openProviderFlow("fill")
        policy.onBackground()
        now += 60_000
        policy.tick()
        assertEquals(1, locks)
        // The user unlocks again later and leaves again: no flow is open, so the background rule applies at once.
        unlocked = true
        policy.onForeground()
        policy.onBackground()
        assertEquals(2, locks)
    }

    @Test
    fun returningToTheForegroundEndsAnAbandonedProviderFlow() {
        val policy = policy()
        policy.openProviderFlow("fill")
        policy.onBackground()
        now += 5_000
        policy.onForeground()
        assertEquals(0, locks)
        policy.onBackground()
        assertEquals(1, locks)
    }

    @Test
    fun aPendingJoinIsNeverLockedAway() {
        joinPending = true
        val policy = policy()
        policy.onBackground()
        policy.onScreenOff()
        assertEquals(0, locks)
        joinPending = false
        policy.onScreenOff()
        assertEquals(1, locks)
    }

    @Test
    fun aLockedCoreIsNotLockedAgain() {
        unlocked = false
        val policy = policy()
        policy.onBackground()
        assertEquals(0, locks)
    }
}
