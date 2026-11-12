package io.github.stephanbaum.mosaic.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import io.github.stephanbaum.mosaic.ui.theme.MosaicColors
import io.github.stephanbaum.mosaic.ui.theme.MosaicDarkColors
import io.github.stephanbaum.mosaic.ui.theme.MosaicLightColors
import io.github.stephanbaum.mosaic.ui.theme.MosaicType

/** The semantic colors of the design tokens (UX_GUIDELINES section 2) for the current theme. */
val LocalMosaicColors = staticCompositionLocalOf { MosaicLightColors }

/** The UI and mono font families (apps/mobile/shared/fonts, SIL OFL 1.1), loaded from the app assets. */
class MosaicFontFamilies(val ui: FontFamily, val mono: FontFamily)

val LocalMosaicFonts = staticCompositionLocalOf { MosaicFontFamilies(FontFamily.Default, FontFamily.Monospace) }

/** Accessor for composables: `MosaicTheme.colors.text`, `MosaicTheme.fonts.ui`. */
object MosaicTheme {
    val colors: MosaicColors
        @Composable
        @ReadOnlyComposable
        get() = LocalMosaicColors.current

    val fonts: MosaicFontFamilies
        @Composable
        @ReadOnlyComposable
        get() = LocalMosaicFonts.current
}

/**
 * Applies the generated design tokens. Material 3 supplies behaviour (ripples, semantics); its color
 * scheme is mapped from the tokens, so no color here is invented. The fonts are DM Sans (UI) and IBM Plex
 * Mono (secrets, codes).
 */
@Composable
fun MosaicTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val tokens = if (darkTheme) MosaicDarkColors else MosaicLightColors
    val assets = LocalContext.current.assets
    val fonts =
        remember(assets) {
            MosaicFontFamilies(
                ui =
                    FontFamily(
                        Font(path = "DMSans-Variable.ttf", assetManager = assets, weight = FontWeight.Normal),
                        Font(path = "DMSans-Variable.ttf", assetManager = assets, weight = FontWeight.Medium),
                        Font(path = "DMSans-Variable.ttf", assetManager = assets, weight = FontWeight.SemiBold),
                    ),
                mono =
                    FontFamily(
                        Font(path = "IBMPlexMono-Regular.ttf", assetManager = assets, weight = FontWeight.Normal),
                        Font(path = "IBMPlexMono-Medium.ttf", assetManager = assets, weight = FontWeight.Medium),
                        Font(path = "IBMPlexMono-SemiBold.ttf", assetManager = assets, weight = FontWeight.SemiBold),
                    ),
            )
        }
    val typography =
        Typography(
            bodyLarge = TextStyle(fontFamily = fonts.ui, fontSize = MosaicType.md.size, lineHeight = MosaicType.md.lineHeight),
            bodyMedium = TextStyle(fontFamily = fonts.ui, fontSize = MosaicType.sm.size, lineHeight = MosaicType.sm.lineHeight),
            bodySmall = TextStyle(fontFamily = fonts.ui, fontSize = MosaicType.xs.size, lineHeight = MosaicType.xs.lineHeight),
            labelLarge =
                TextStyle(
                    fontFamily = fonts.ui,
                    fontWeight = FontWeight.Medium,
                    fontSize = MosaicType.sm.size,
                    lineHeight = MosaicType.sm.lineHeight,
                ),
            titleMedium =
                TextStyle(
                    fontFamily = fonts.ui,
                    fontWeight = FontWeight.Medium,
                    fontSize = MosaicType.md.size,
                    lineHeight = MosaicType.md.lineHeight,
                ),
        )
    val scheme =
        if (darkTheme) {
            darkColorScheme(
                primary = tokens.accent,
                onPrimary = tokens.accentText,
                background = tokens.bg,
                onBackground = tokens.text,
                surface = tokens.surface,
                onSurface = tokens.text,
                surfaceVariant = tokens.bgSubtle,
                onSurfaceVariant = tokens.textMuted,
                outline = tokens.border,
                error = tokens.danger,
            )
        } else {
            lightColorScheme(
                primary = tokens.accent,
                onPrimary = tokens.accentText,
                background = tokens.bg,
                onBackground = tokens.text,
                surface = tokens.surface,
                onSurface = tokens.text,
                surfaceVariant = tokens.bgSubtle,
                onSurfaceVariant = tokens.textMuted,
                outline = tokens.border,
                error = tokens.danger,
            )
        }
    CompositionLocalProvider(LocalMosaicColors provides tokens, LocalMosaicFonts provides fonts) {
        MaterialTheme(colorScheme = scheme, typography = typography, content = content)
    }
}
