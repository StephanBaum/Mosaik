package io.github.stephanbaum.mosaic.security

import android.content.Intent
import android.content.pm.PackageManager
import android.view.WindowManager
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import io.github.stephanbaum.mosaic.ui.MainActivity
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Every window of the app is secure (ADR-0094): each declared activity extends SecureActivity, and it shows. */
@RunWith(AndroidJUnit4::class)
class ScreenSecurityTest {
    @Test
    @Suppress("DEPRECATION")
    fun everyActivityHasFlagSecure() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext

        val declared = context.packageManager.getPackageInfo(context.packageName, PackageManager.GET_ACTIVITIES).activities
        assertTrue("the manifest declares no activity", declared != null && declared.isNotEmpty())
        // Activities of libraries (the Compose tooling PreviewActivity in debug builds) are not ours to protect.
        for (info in declared.orEmpty().filter { it.name.startsWith(context.packageName) }) {
            val type = Class.forName(info.name)
            assertTrue("${info.name} must extend SecureActivity", SecureActivity::class.java.isAssignableFrom(type))
        }

        val intent = Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        val activity = instrumentation.startActivitySync(intent)
        try {
            val flags = activity.window.attributes.flags
            assertTrue("FLAG_SECURE is not set", flags and WindowManager.LayoutParams.FLAG_SECURE != 0)
        } finally {
            instrumentation.runOnMainSync { activity.finish() }
        }
    }
}
