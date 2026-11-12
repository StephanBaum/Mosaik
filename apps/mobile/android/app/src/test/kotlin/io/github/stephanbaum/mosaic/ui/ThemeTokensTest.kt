package io.github.stephanbaum.mosaic.ui

import androidx.compose.ui.graphics.toArgb
import io.github.stephanbaum.mosaic.ui.theme.MosaicDarkColors
import io.github.stephanbaum.mosaic.ui.theme.MosaicLightColors
import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Test

/** The window background before Compose draws (res/values*/colors.xml) is the generated bg token. */
class ThemeTokensTest {
    private fun windowBackground(dir: String): Int {
        val xml = File("src/main/res/$dir/colors.xml").readText()
        val hex = Regex("name=\"mosaic_window_background\">#([0-9A-Fa-f]{8})<").find(xml)!!.groupValues[1]
        return hex.toLong(16).toInt()
    }

    @Test
    fun windowBackgroundMatchesTokens() {
        assertEquals(MosaicLightColors.bg.toArgb(), windowBackground("values"))
        assertEquals(MosaicDarkColors.bg.toArgb(), windowBackground("values-night"))
    }
}
