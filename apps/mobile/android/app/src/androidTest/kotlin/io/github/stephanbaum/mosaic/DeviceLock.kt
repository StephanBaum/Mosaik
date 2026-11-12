package io.github.stephanbaum.mosaic

import android.app.KeyguardManager
import android.content.Context
import android.os.ParcelFileDescriptor
import androidx.test.platform.app.InstrumentationRegistry

/**
 * The emulator has no lock screen, and the secure storage refuses to create its key without one (ADR-0090
 * amendment "device lock required"). Tests that touch the storage set a PIN through the shell first and clear
 * it afterwards.
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

    /** Sets a PIN when the device has no secure lock screen. True when this call set it (the caller clears it). */
    fun ensure(): Boolean {
        if (isSecure()) return false
        shell("locksettings set-pin $PIN")
        shell("wm dismiss-keyguard")
        return true
    }

    /** Clears the PIN that [ensure] set. */
    fun release() {
        shell("locksettings clear --old $PIN")
    }
}
