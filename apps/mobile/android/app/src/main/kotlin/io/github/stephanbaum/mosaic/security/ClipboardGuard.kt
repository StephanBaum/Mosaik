package io.github.stephanbaum.mosaic.security

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.PersistableBundle

/** What the guard can learn about the current clip's label. */
sealed interface ClipRead {
    /** The label of the current clip (null when the clip has none). */
    class Readable(val label: String?) : ClipRead

    /** Android 10+ hides the clipboard from a background app. */
    data object Unreadable : ClipRead
}

/** The clipboard as the guard needs it; the Android adapter is [AndroidClipboardPort]. */
interface ClipboardPort {
    fun set(label: String, value: CharSequence, extras: Map<String, Boolean>)

    fun read(): ClipRead

    fun clear()
}

/** Runs [task] after [delayMs]; the returned function cancels it. */
fun interface Scheduler {
    fun after(delayMs: Long, task: () -> Unit): () -> Unit
}

/**
 * Copies a secret to the clipboard marked as sensitive, and clears it after a timeout (docs/MOBILE.md,
 * ADR-0094, THREAT_MODEL R130):
 * - the clip carries `android.content.extra.IS_SENSITIVE`, so keyboards and the overlay do not show it;
 * - a label nonce kept in memory tells our clip from one the user copied later: a readable clipboard with
 *   another label is left alone, an unreadable one is cleared unconditionally;
 * - a process death before the timeout leaves the clip, which Android 13+ clears on its own later.
 *
 * The value is a `CharSequence`: a JVM string cannot be wiped (CRYPTO_SPEC 6.9.5).
 */
class ClipboardGuard(
    private val port: ClipboardPort,
    private val scheduler: Scheduler,
    private val nonce: () -> String,
) {
    private var currentLabel: String? = null
    private var cancelPending: (() -> Unit)? = null

    /** Copies [value] and schedules the clearing after [clearAfterSeconds] seconds. */
    fun copy(value: CharSequence, clearAfterSeconds: Int) {
        cancelPending?.invoke()
        val label = LABEL + "-" + nonce()
        currentLabel = label
        port.set(label, value, mapOf(EXTRA_IS_SENSITIVE to true))
        cancelPending = scheduler.after(clearAfterSeconds * 1000L) { clearIfOurs(label) }
    }

    /** Clears our clip now (on lock), unless the user copied something else. */
    fun clearNow() {
        val label = currentLabel ?: return
        cancelPending?.invoke()
        clearIfOurs(label)
    }

    private fun clearIfOurs(label: String) {
        if (currentLabel != label) return
        currentLabel = null
        cancelPending = null
        when (val read = port.read()) {
            is ClipRead.Readable -> if (read.label == label) port.clear()
            ClipRead.Unreadable -> port.clear()
        }
    }

    companion object {
        const val LABEL = "Mosaic"

        /** The literal key of `ClipDescription.EXTRA_IS_SENSITIVE` (API 33), which older versions ignore. */
        const val EXTRA_IS_SENSITIVE = "android.content.extra.IS_SENSITIVE"
    }
}

/** The platform adapter of [ClipboardPort]. */
class AndroidClipboardPort(context: Context) : ClipboardPort {
    private val manager = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager

    override fun set(label: String, value: CharSequence, extras: Map<String, Boolean>) {
        val clip = ClipData.newPlainText(label, value)
        val bundle = PersistableBundle()
        for ((key, flag) in extras) bundle.putBoolean(key, flag)
        clip.description.extras = bundle
        manager.setPrimaryClip(clip)
    }

    override fun read(): ClipRead {
        val description = manager.primaryClipDescription ?: return ClipRead.Unreadable
        return ClipRead.Readable(description.label?.toString())
    }

    override fun clear() {
        manager.clearPrimaryClip()
    }
}

/** [Scheduler] on the main looper. */
class MainLooperScheduler : Scheduler {
    private val handler = Handler(Looper.getMainLooper())

    override fun after(delayMs: Long, task: () -> Unit): () -> Unit {
        val runnable = Runnable { task() }
        handler.postDelayed(runnable, delayMs)
        return { handler.removeCallbacks(runnable) }
    }
}
