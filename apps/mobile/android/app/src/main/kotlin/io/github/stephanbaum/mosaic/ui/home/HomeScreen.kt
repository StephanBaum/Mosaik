package io.github.stephanbaum.mosaic.ui.home

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.SecondaryButton
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.l10n.L10n

/** What follows an unlock until the vault screens (T8) replace it: who is signed in, and a way to lock. */
@Composable
fun HomeScreen(email: String?, onLock: () -> Unit) {
    val resources = LocalContext.current.resources
    ScreenFrame {
        TitleText(L10n.vaultTitle(resources))
        if (email != null) BodyText(L10n.homeUnlockedAs(resources, email))
        SecondaryButton(L10n.lockNow(resources), onClick = onLock)
    }
}
