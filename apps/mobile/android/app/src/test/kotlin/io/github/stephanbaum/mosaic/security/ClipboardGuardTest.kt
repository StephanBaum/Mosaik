package io.github.stephanbaum.mosaic.security

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ClipboardGuardTest {
    private class FakePort : ClipboardPort {
        var label: String? = null
        var extras: Map<String, Boolean> = emptyMap()
        var current: ClipRead = ClipRead.Unreadable
        var clears = 0

        override fun set(label: String, value: CharSequence, extras: Map<String, Boolean>) {
            this.label = label
            this.extras = extras
        }

        override fun read(): ClipRead = current

        override fun clear() {
            clears += 1
        }
    }

    private class FakeScheduler : Scheduler {
        var delayMs = -1L
        var task: (() -> Unit)? = null

        override fun after(delayMs: Long, task: () -> Unit): () -> Unit {
            this.delayMs = delayMs
            this.task = task
            return { this.task = null }
        }

        fun fire() {
            val pending = task
            task = null
            pending?.invoke()
        }
    }

    private val port = FakePort()
    private val scheduler = FakeScheduler()
    private val guard = ClipboardGuard(port, scheduler) { "n1" }

    @Test
    fun marksClipSensitive() {
        guard.copy("secret", 30)
        assertEquals(true, port.extras[ClipboardGuard.EXTRA_IS_SENSITIVE])
        assertEquals("android.content.extra.IS_SENSITIVE", ClipboardGuard.EXTRA_IS_SENSITIVE)
        assertEquals(30_000L, scheduler.delayMs)
        assertTrue(port.label.orEmpty().startsWith(ClipboardGuard.LABEL))
    }

    @Test
    fun keepsForeignClipWhenReadable() {
        guard.copy("secret", 30)
        port.current = ClipRead.Readable("something the user copied")
        scheduler.fire()
        assertEquals(0, port.clears)
    }

    @Test
    fun clearsWhenUnreadable() {
        guard.copy("secret", 30)
        port.current = ClipRead.Unreadable
        scheduler.fire()
        assertEquals(1, port.clears)
    }

    @Test
    fun clearsOurOwnClipWhenReadable() {
        guard.copy("secret", 30)
        port.current = ClipRead.Readable(port.label)
        scheduler.fire()
        assertEquals(1, port.clears)
    }

    @Test
    fun aSecondCopyReplacesThePendingClear() {
        guard.copy("one", 30)
        guard.copy("two", 10)
        assertEquals(10_000L, scheduler.delayMs)
        port.current = ClipRead.Readable(port.label)
        scheduler.fire()
        assertEquals(1, port.clears)
        assertNull(scheduler.task)
    }
}
