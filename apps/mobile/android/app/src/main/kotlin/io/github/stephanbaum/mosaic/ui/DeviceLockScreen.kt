package io.github.stephanbaum.mosaic.ui

import android.content.ActivityNotFoundException
import android.content.Intent
import android.provider.Settings
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.l10n.L10n

/**
 * Shown instead of the onboarding and the unlock screen on a phone without a secure lock screen: Mosaic stores
 * no secret there (ADR-0090 amendment "device lock required"). It reads the state again when the app returns
 * to the front, so setting a lock in the system settings continues at once.
 */
@Composable
fun DeviceLockScreen() {
    val context = LocalContext.current
    val resources = context.resources
    ScreenFrame {
        TitleText(L10n.deviceLockRequiredTitle(resources))
        BodyText(L10n.deviceLockRequiredBody(resources))
        PrimaryButton(
            L10n.deviceLockRequiredAction(resources),
            onClick = {
                try {
                    context.startActivity(Intent(Settings.ACTION_SECURITY_SETTINGS))
                } catch (e: ActivityNotFoundException) {
                    // The phone has no such settings page; the text above still tells the person what to do.
                }
            },
        )
    }
}
