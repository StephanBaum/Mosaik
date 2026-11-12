package io.github.stephanbaum.mosaic.security

import io.github.stephanbaum.mosaic.ffi.SessionState

/**
 * Whether a lock must reach the core in [state]: every state except a locked one and "no account". A lock that
 * arrives while the core is unlocking (`Busy`) is passed on too: the core turns it into "ends locked" (M10-G2 M1).
 * A core that is not open yet (null) has no session.
 */
fun sessionNeedsLock(state: SessionState?): Boolean =
    state != null && state !is SessionState.Locked && state !is SessionState.NoAccount

/** The settings the lock policy reads (from `MobileSettings`; the defaults apply until they are loaded). */
class LockSettings(
    val backgroundSeconds: Int = DEFAULT_BACKGROUND_SECONDS,
    val idleMinutes: Int = DEFAULT_IDLE_MINUTES,
    val clipboardSeconds: Int = DEFAULT_CLIPBOARD_SECONDS,
) {
    companion object {
        const val DEFAULT_BACKGROUND_SECONDS = 0
        const val DEFAULT_IDLE_MINUTES = 5
        const val DEFAULT_CLIPBOARD_SECONDS = 30
    }
}

/**
 * When the app locks (docs/MOBILE.md section 3, ADR-0094). A plain state machine: the platform code feeds it
 * events and a clock, so it runs without Android in JVM tests.
 *
 * - Background (the last activity stopped) locks after `backgroundSeconds` (default 0, at once).
 * - Foreground idle time locks after `idleMinutes` (default 5), reset by [onUserInteraction].
 * - Screen off locks.
 * - A new process starts locked (that is the core's state, not this class).
 * - None of them can be turned off.
 *
 * Provider grace (Android providers only): while a provider flow is open ([openProviderFlow]), a background
 * keeps the session for at most [PROVIDER_GRACE_MS] and not longer than the flow lasts. Screen off ends the
 * flow and locks. Outside a flow the background rule applies unchanged (THREAT_MODEL R131).
 *
 * The lock action is never called while [joinPending]: `lock()` in the core drops a pending join, which a
 * TOTP app switch must survive (docs/MOBILE.md section 2).
 */
class LockPolicy(
    private val clock: () -> Long,
    private val settings: () -> LockSettings,
    private val hasSession: () -> Boolean,
    private val joinPending: () -> Boolean,
    private val lock: () -> Unit,
) {
    private var backgroundedAt: Long? = null
    private var graceDeadline: Long? = null
    private var lastInteractionAt: Long = clock()
    private val flows = HashSet<String>()

    /** The last activity stopped. */
    fun onBackground() {
        val now = clock()
        backgroundedAt = now
        if (flows.isNotEmpty()) {
            graceDeadline = now + PROVIDER_GRACE_MS
            return
        }
        evaluate(now)
    }

    /** An activity started. The ticker may not have run while the app was away, so the policy checks first. */
    fun onForeground() {
        val now = clock()
        evaluate(now)
        backgroundedAt = null
        graceDeadline = null
        // A flow only matters while the app is away: a provider activity opens its flow again once it is unlocked.
        flows.clear()
        lastInteractionAt = now
    }

    /** A touch in an activity. */
    fun onUserInteraction() {
        lastInteractionAt = clock()
    }

    /** `ACTION_SCREEN_OFF`. */
    fun onScreenOff() {
        flows.clear()
        graceDeadline = null
        lockNow()
    }

    /** A provider flow (autofill or credential request) starts; [token] names it. */
    fun openProviderFlow(token: String) {
        flows.add(token)
    }

    /** The flow finished or was cancelled. Closing the last one ends the grace. */
    fun closeProviderFlow(token: String) {
        flows.remove(token)
        if (flows.isEmpty() && graceDeadline != null) {
            graceDeadline = null
            evaluate(clock())
        }
    }

    /** Called about once a second while the app is in the foreground, and on each event. */
    fun tick() {
        evaluate(clock())
    }

    private fun evaluate(now: Long) {
        val current = settings()
        val since = backgroundedAt
        if (since != null) {
            val deadline = graceDeadline ?: (since + current.backgroundSeconds * 1000L)
            if (now >= deadline) lockNow()
        } else if (now - lastInteractionAt >= current.idleMinutes * 60_000L) {
            lockNow()
        }
    }

    private fun lockNow() {
        if (!hasSession()) return
        if (joinPending()) return
        // A lock ends every provider flow, so a flow that was abandoned cannot grant a later grace.
        flows.clear()
        graceDeadline = null
        lock()
    }

    companion object {
        /** The provider grace of ADR-0094, in milliseconds. */
        const val PROVIDER_GRACE_MS = 60_000L
    }
}
