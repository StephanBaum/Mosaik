package io.github.stephanbaum.mosaic.provider

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.security.ErrorCodes
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.unlock.UnlockScreen
import kotlinx.coroutines.CancellationException

/** The names of the provider flows the lock policy knows (ADR-0094, provider grace). */
object ProviderFlows {
    const val AUTOFILL = "autofill"
    const val CREDENTIAL = "credential"
}

/**
 * The unlock step of a provider activity: the same unlock screen as the app (master password, or quick unlock),
 * then [content]. A session that is already open skips it. Once the session is open the flow [flow] is registered with
 * the lock policy, which lets the session survive the short step between two provider activities for at most 60
 * seconds (ADR-0094). A lock while the activity is open brings the unlock screen back.
 */
@Composable
fun ProviderGate(app: MosaicApp, flow: String, onClose: () -> Unit = {}, content: @Composable () -> Unit) {
    val resources = LocalContext.current.resources
    val epoch by app.sessionEpoch.collectAsState()
    var state by remember { mutableStateOf<SessionState?>(null) }
    var failure by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(epoch) {
        try {
            state = app.session.currentState()
            failure = null
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            failure = ErrorCodes.of(e)
        }
    }

    val current = state
    val error = failure
    if (error != null) {
        Message(L10n.providerErrorTitle(resources), errorText(resources, error), L10n.unlockRetry(resources)) {
            app.session.notifyChanged()
        }
    } else if (current is SessionState.Locked) {
        UnlockScreen(app = app, state = current)
    } else if (current is SessionState.Unlocked) {
        LaunchedEffect(Unit) { app.lockPolicy.openProviderFlow(flow) }
        content()
    } else if (current == null) {
        ScreenFrame { }
    } else if (current is SessionState.NoAccount) {
        // Nothing to retry: the app has to be set up first.
        Message(L10n.appName(resources), errorText(resources, "noAccount"), L10n.commonClose(resources), onClose)
    } else {
        val code = if (current is SessionState.StoreBusy) "storeInUse" else "busy"
        Message(L10n.unlockBusyTitle(resources), errorText(resources, code), L10n.unlockRetry(resources)) {
            app.session.notifyChanged()
        }
    }
}

@Composable
private fun Message(title: String, message: String, action: String, onAction: () -> Unit) {
    ScreenFrame {
        TitleText(title)
        BodyText(message)
        PrimaryButton(action, onClick = onAction)
    }
}
