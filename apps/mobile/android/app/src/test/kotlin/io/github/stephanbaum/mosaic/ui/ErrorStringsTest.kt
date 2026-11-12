package io.github.stephanbaum.mosaic.ui

import io.github.stephanbaum.mosaic.ffi.mobileErrorCodes
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** Every code the Rust core can answer with has an `error.<code>` text in both languages. */
class ErrorStringsTest {
    @Test
    fun everyMobileErrorCodeHasAStringResource() {
        val codes = mobileErrorCodes()
        assertTrue("the core lists no error codes", codes.isNotEmpty())
        for (dir in listOf("values", "values-de")) {
            val xml = File("src/main/res/$dir/strings.xml").readText()
            val names = Regex("<string name=\"([^\"]+)\"").findAll(xml).map { it.groupValues[1] }.toSet()
            for (code in codes) {
                assertTrue("$dir/strings.xml has no error_$code", "error_$code" in names)
            }
        }
        // The generated list of codes is the core's list.
        assertEquals(codes.toSet(), L10n.errorCodes.toSet())
    }
}
