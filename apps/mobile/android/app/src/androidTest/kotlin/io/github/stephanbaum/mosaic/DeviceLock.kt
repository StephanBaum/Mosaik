package io.github.stephanbaum.mosaic

import android.app.KeyguardManager
import android.content.Context
import android.os.ParcelFileDescriptor
import androidx.test.platform.app.InstrumentationRegistry

/**
 * The emulator has no lock screen, and the secure storage refuses to create its key without one (ADR-0090
 * amendment "device lock required"). Tests that touch the storage set a PIN through the shell first and clear
 * it afterwards. Removing it deletes the Keystore key of the secure storage (the key needs a lock screen), and the core
 * of the process caches that key, so a test class that creates accounts keeps the PIN for the rest of the run.
 */
object DeviceLock {
    private const val PIN = "1234"

    private fun shell(command: String) {
        val descriptor = InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand(command)
        ParcelFileDescriptor.AutoCloseInputStream(descriptor).use { it.readBytes() }
    }

    private fun isSecure(): Boolean {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        return (context.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager).isDeviceSecure
    }

    /**
     * Sets a PIN when the device has no secure lock screen. True when this call set it (the caller clears it). It
     * returns only once the system reports the lock screen as secure: a test that runs right after another one must
     * not see the state of the PIN that is just being removed.
     */
    fun ensure(): Boolean {
        if (isSecure()) return false
        shell("locksettings set-pin $PIN")
        shell("wm dismiss-keyguard")
        waitUntil(true)
        return true
    }

    /** Clears the PIN that [ensure] set, and returns once the system reports no secure lock screen. */
    fun release() {
        shell("locksettings clear --old $PIN")
        waitUntil(false)
    }

    private fun waitUntil(secure: Boolean) {
        val end = System.currentTimeMillis() + 10_000
        while (isSecure() != secure && System.currentTimeMillis() < end) Thread.sleep(100)
    }
}
