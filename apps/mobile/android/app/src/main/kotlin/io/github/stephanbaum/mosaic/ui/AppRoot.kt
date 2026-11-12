package io.github.stephanbaum.mosaic.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.security.ErrorCodes
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.home.HomeScreen
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.onboarding.OnboardingFlow
import io.github.stephanbaum.mosaic.ui.unlock.UnlockScreen
import kotlinx.coroutines.CancellationException

/**
 * Picks the screen from the core's session state: no account gives the onboarding (which stays until the user
 * finishes it, although the state turns unlocked after the join), a locked account the unlock screen, an
 * unlocked one the home screen. The state is read again whenever the session epoch changes (a lock, a join).
 */
@Composable
fun AppRoot(app: MosaicApp) {
    val resources = LocalContext.current.resources
    val epoch by app.sessionEpoch.collectAsState()
    var state by remember { mutableStateOf<SessionState?>(null) }
    var loadError by remember { mutableStateOf<String?>(null) }
    var inOnboarding by rememberSaveable { mutableStateOf(false) }

    LaunchedEffect(epoch) {
        try {
            val read = app.session.currentState()
            loadError = null
            state = read
            if (read is SessionState.NoAccount) inOnboarding = true
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            loadError = ErrorCodes.of(e)
        }
    }

    val failure = loadError
    val current = state
    if (inOnboarding) {
        OnboardingFlow(app = app, onFinished = { inOnboarding = false })
    } else if (failure != null) {
        MessageScreen(
            title = L10n.unlockBusyTitle(resources),
            message = errorText(resources, failure),
            action = L10n.unlockRetry(resources),
            onAction = { app.session.notifyChanged() },
        )
    } else if (current == null) {
        ScreenFrame { }
    } else if (current is SessionState.Locked) {
        UnlockScreen(app = app, state = current)
    } else if (current is SessionState.Unlocked) {
        HomeScreen(email = current.email, onLock = { app.lockNow() })
    } else if (current is SessionState.NoAccount) {
        ScreenFrame { }
    } else {
        // StoreBusy, StoreClosed (reopened by the read) or a long call running: wait and offer a retry.
        MessageScreen(
            title = L10n.unlockBusyTitle(resources),
            message = errorText(resources, if (current is SessionState.StoreBusy) "storeInUse" else "busy"),
            action = L10n.unlockRetry(resources),
            onAction = { app.session.notifyChanged() },
        )
    }
}

@Composable
private fun MessageScreen(title: String, message: String, action: String, onAction: () -> Unit) {
    ScreenFrame {
        TitleText(title)
        BodyText(message)
        PrimaryButton(action, onClick = onAction)
    }
}
