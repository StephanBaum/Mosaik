package io.github.stephanbaum.mosaic.security

/**
 * A secret kept as bytes that goes away after [ttlMs] on a monotonic clock that also counts deep sleep
 * (`SystemClock.elapsedRealtime`: a coroutine `delay` does not, M10-G2 round 2, N2). Plain Kotlin, so the JVM tests run
 * it with a fake clock. The owner wipes it when the app goes to the background.
 */
class ExpiringKey(private val clock: () -> Long, private val ttlMs: Long) {
    private var setAt = 0L

    var bytes: ByteArray? = null
        private set

    /** Keeps [value] (the bytes are owned from here on); the previous key is wiped and the clock restarts. */
    fun set(value: ByteArray) {
        wipe()
        bytes = value
        setAt = clock()
    }

    fun wipe() {
        bytes?.fill(0)
        bytes = null
    }

    /** The time left before the key expires, or null when there is no key. Zero or less: expired. */
    fun remainingMs(): Long? = if (bytes == null) null else setAt + ttlMs - clock()

    /** Wipes the key when its time is up; true when it did. */
    fun wipeIfExpired(): Boolean {
        val left = remainingMs() ?: return false
        if (left > 0) return false
        wipe()
        return true
    }
}
