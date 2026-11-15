package io.github.stephanbaum.mosaic.scan

/** What a scan screen looks for. */
enum class ScanTarget {
    /** The desktop's setup code (`mosaic-setup:1:`), in onboarding. */
    SetupCode,

    /** An `otpauth://` code, in the login editor. */
    Otp,
}

/** Lets one frame in per [minIntervalMs]: the core needs a few looks a second, not thirty. */
class FrameThrottle(
    private val minIntervalMs: Long,
    private val clock: () -> Long,
) {
    private var last: Long? = null

    fun admit(): Boolean {
        val now = clock()
        val before = last
        if (before != null && now - before < minIntervalMs) return false
        last = now
        return true
    }
}

/**
 * Counts how long frames came back without any code. After [hintAfterMs] of that the screen says what it is
 * looking for. The core answers "none" both for no QR code at all and for QR codes of something else (it drops
 * those unread), so this is the only signal there is.
 */
class MissTracker(
    private val hintAfterMs: Long,
    private val clock: () -> Long,
) {
    private var since: Long? = null

    /** A frame without a code. Returns true once the hint is due. */
    fun miss(): Boolean {
        val now = clock()
        val start = since ?: now.also { since = it }
        return now - start >= hintAfterMs
    }

    /** A frame with a code, or with an error: start counting again. */
    fun reset() {
        since = null
    }
}

/** Where the camera permission stands. */
enum class CameraAccess {
    /** Not asked yet: explain first, then ask. */
    Ask,

    /** Allowed. */
    Granted,

    /** Asked and refused. */
    Denied,
}

/** The state of the camera permission from what the system says and whether the app has asked. */
fun cameraAccess(granted: Boolean, requested: Boolean): CameraAccess =
    when {
        granted -> CameraAccess.Granted
        !requested -> CameraAccess.Ask
        else -> CameraAccess.Denied
    }
