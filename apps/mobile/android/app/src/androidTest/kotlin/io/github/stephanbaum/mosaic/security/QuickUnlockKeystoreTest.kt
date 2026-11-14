package io.github.stephanbaum.mosaic.security

import android.app.KeyguardManager
import android.content.Context
import android.os.ParcelFileDescriptor
import android.security.keystore.KeyInfo
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import javax.crypto.SecretKeyFactory
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeNoException
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The quick-unlock key as the Keystore reports it (CRYPTO_SPEC 6.2): per-use user authentication, invalidated
 * by a biometric enrolment. A Keystore only creates such a key on a device with a secure lock screen (and, on
 * some images, an enrolled biometric), so the test sets a PIN through the shell for its duration and skips,
 * with the reason, when the emulator still refuses to create the key.
 */
@RunWith(AndroidJUnit4::class)
class QuickUnlockKeystoreTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val alias = "mosaic-quick-unlock-test"

    private fun shell(command: String) {
        val descriptor = instrumentation.uiAutomation.executeShellCommand(command)
        ParcelFileDescriptor.AutoCloseInputStream(descriptor).use { it.readBytes() }
    }

    @Test
    @Suppress("DEPRECATION")
    fun keyIsAuthBoundAndInvalidatedByEnrollment() {
        val context = instrumentation.targetContext
        val keyguard = context.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
        val setPin = !keyguard.isDeviceSecure
        if (setPin) shell("locksettings set-pin 1234")
        val keys = QuickUnlockKeys()
        try {
            assumeTrue("no secure lock screen on this image", keyguard.isDeviceSecure)
            try {
                keys.prepareEnroll(alias)
            } catch (e: Exception) {
                assumeNoException("this image cannot create an auth-bound key", e)
            }
            val key = keys.load(alias)
            assertTrue("the key is not in the Keystore", key != null)
            val info = SecretKeyFactory.getInstance(key!!.algorithm, "AndroidKeyStore").getKeySpec(key, KeyInfo::class.java) as KeyInfo
            assertTrue("user authentication is not required", info.isUserAuthenticationRequired)
            assertTrue("not invalidated by a biometric enrolment", info.isInvalidatedByBiometricEnrollment)
            if (info.isInsideSecureHardware) {
                assertTrue("authentication is not enforced by the secure hardware", info.isUserAuthenticationRequirementEnforcedBySecureHardware)
            }
        } finally {
            keys.delete(alias)
            if (setPin) shell("locksettings clear --old 1234")
        }
    }
}
