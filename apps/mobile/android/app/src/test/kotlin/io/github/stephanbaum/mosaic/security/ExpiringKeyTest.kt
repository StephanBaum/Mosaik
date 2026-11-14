package io.github.stephanbaum.mosaic.security

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ExpiringKeyTest {
    private var now = 1_000L
    private val key = ExpiringKey({ now }, 300_000L)

    @Test
    fun aKeyExpiresAfterTheTimeToLiveOnTheInjectedClock() {
        val bytes = byteArrayOf(1, 2, 3)
        key.set(bytes)
        assertEquals(300_000L, key.remainingMs())
        now += 299_999
        assertFalse(key.wipeIfExpired())
        assertEquals(1L, key.remainingMs())
        now += 1
        assertTrue(key.wipeIfExpired())
        assertNull(key.bytes)
        assertNull(key.remainingMs())
        // The bytes themselves were overwritten.
        assertArrayEquals(byteArrayOf(0, 0, 0), bytes)
    }

    @Test
    fun aSleepThatOutlastsTheTimerStillExpiresTheKey() {
        // The clock counts deep sleep; a coroutine delay may not (N2). Checking on return wipes at once.
        key.set(byteArrayOf(9))
        now += 10 * 60_000
        assertTrue(key.wipeIfExpired())
        assertNull(key.bytes)
    }

    @Test
    fun settingAKeyWipesTheOldOneAndRestartsTheClock() {
        val first = byteArrayOf(1, 1)
        key.set(first)
        now += 200_000
        key.set(byteArrayOf(2, 2))
        assertArrayEquals(byteArrayOf(0, 0), first)
        assertEquals(300_000L, key.remainingMs())
    }

    @Test
    fun wipeIsIdempotentAndNeverExpiresWithoutAKey() {
        key.wipe()
        assertFalse(key.wipeIfExpired())
        key.set(byteArrayOf(5))
        key.wipe()
        key.wipe()
        assertNull(key.bytes)
    }
}
