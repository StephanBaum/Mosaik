package io.github.stephanbaum.mosaic.security

import android.app.ActivityManager
import android.content.Context

/**
 * The memory preflight of CRYPTO_SPEC 6.9.4. A failed Argon2 allocation aborts the process, so the phone
 * compares `required_unlock_memory_bytes()` with the memory available before the call.
 */
object MemoryCheck {
    fun availableBytes(context: Context): Long {
        val manager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
        val info = ActivityManager.MemoryInfo()
        manager.getMemoryInfo(info)
        return info.availMem
    }

    /** True when nothing is required (no stored profile yet) or [available] covers [required]. */
    fun isEnough(required: ULong?, available: Long): Boolean {
        if (required == null) return true
        if (available < 0L) return false
        return available.toULong() >= required
    }
}
