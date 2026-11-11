package io.github.stephanbaum.mosaic.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import io.github.stephanbaum.mosaic.ui.theme.MosaicColors
import io.github.stephanbaum.mosaic.ui.theme.MosaicDarkColors
import io.github.stephanbaum.mosaic.ui.theme.MosaicLightColors

/** The semantic colors of the design tokens (UX_GUIDELINES section 2) for the current theme. */
val LocalMosaicColors = staticCompositionLocalOf { MosaicLightColors }

/** Accessor for composables: `MosaicTheme.colors.text`. */
object MosaicTheme {
    val colors: MosaicColors
        @Composable
        @ReadOnlyComposable
        get() = LocalMosaicColors.current
}

/**
 * Applies the generated design tokens. Material 3 supplies behaviour (ripples, semantics); its color
 * scheme is mapped from the tokens, so no color here is invented.
 */
@Composable
fun MosaicTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val tokens = if (darkTheme) MosaicDarkColors else MosaicLightColors
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
    CompositionLocalProvider(LocalMosaicColors provides tokens) {
        MaterialTheme(colorScheme = scheme, content = content)
    }
}
