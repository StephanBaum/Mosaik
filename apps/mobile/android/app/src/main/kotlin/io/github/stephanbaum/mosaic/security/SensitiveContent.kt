package io.github.stephanbaum.mosaic.security

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.platform.LocalView

/**
 * Marks the window content as sensitive (API 35+) while [active] is true: a revealed value, a generator preview or
 * a one-time code is on screen, so screen sharing and recording hide the window (M10-G2 L7). The mark is taken off
 * again when the value is hidden or the screen leaves the composition. The secret input fields mark themselves
 * ([SecretField]); this is for secrets drawn as text.
 */
@Composable
fun SensitiveWhile(active: Boolean) {
    val view = LocalView.current
    DisposableEffect(view, active) {
        if (active) ScreenSecurity.markSensitive(view)
        onDispose { if (active) ScreenSecurity.clearSensitive(view) }
    }
}
