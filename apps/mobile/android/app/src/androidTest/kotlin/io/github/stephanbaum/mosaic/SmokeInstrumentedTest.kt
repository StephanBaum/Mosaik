package io.github.stephanbaum.mosaic

import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.stephanbaum.mosaic.ffi.coreVersion
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** The T5 smoke: the app is installed under its ID and the Rust library loads on the emulator (JNA + jniLibs). */
@RunWith(AndroidJUnit4::class)
class SmokeInstrumentedTest {
    @Test
    fun appContextAndNativeCoreLoad() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        assertEquals("io.github.stephanbaum.mosaic", context.packageName)
        assertTrue(coreVersion().isNotEmpty())
    }
}
