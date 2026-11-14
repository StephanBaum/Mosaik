package io.github.stephanbaum.mosaic.ui.unlock

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.QuickUnlockAvailability
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.security.BiometricGate
import io.github.stephanbaum.mosaic.security.PromptText
import io.github.stephanbaum.mosaic.security.SecretField
import io.github.stephanbaum.mosaic.security.SecretFieldState
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.CheckRow
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.FieldLabel
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.SecondaryButton
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.components.Working
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import kotlinx.coroutines.launch

/** After these errors the state changed (the enrolment is gone or expired), so the screen reads it again. */
private val STATE_CHANGED_ERRORS = setOf("quickUnlockExpired", "quickUnlockInvalidated", "passwordRequired")

/**
 * The unlock screen: the master password (and the typed Secret Key when this phone has none stored), or the
 * biometric quick unlock when the core offers it. Argon2id runs behind a memory preflight in
 * [io.github.stephanbaum.mosaic.session.SessionController]. A checked "next time" box enrols quick unlock with
 * the same typed bytes (docs/MOBILE.md section 3.10).
 */
@Composable
fun UnlockScreen(app: MosaicApp, state: SessionState.Locked) {
    val context = LocalContext.current
    val resources = context.resources
    val scope = rememberCoroutineScope()
    val session = app.session

    val passwordField = remember { SecretFieldState() }
    val keyField = remember { SecretFieldState() }
    val gate = remember { BiometricGate() }
    val biometricAvailable = remember { gate.isAvailable(context) }
    var busy by remember { mutableStateOf(false) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    var failures by remember { mutableIntStateOf(0) }
    var enrollNext by remember { mutableStateOf(true) }

    val quick = state.quickUnlock
    if (quick is QuickUnlockAvailability.Expired) {
        LaunchedEffect(quick.keyAlias) { session.deleteKey(quick.keyAlias) }
    }
    val promptText =
        PromptText(
            title = L10n.unlockBiometricTitle(resources),
            subtitle = L10n.unlockBiometricSubtitle(resources),
            negativeButton = L10n.unlockUsePassword(resources),
        )

    fun report(code: String) {
        errorCode = code
        if (code == "wrongCredentials") failures += 1
        if (code in STATE_CHANGED_ERRORS) session.notifyChanged()
    }

    val unlockWithPassword: () -> Unit = {
        scope.launch {
            busy = true
            errorCode = null
            val password = passwordField.take()
            val typedKey = if (state.secretKeyStored) null else keyField.take()
            if (password == null) {
                typedKey?.wipe()
                errorCode = "emptyPassword"
            } else {
                val outcome =
                    session.unlockPassword(
                        context = context,
                        password = password,
                        secretKey = typedKey,
                        enrollQuick = enrollNext && biometricAvailable,
                        text = promptText,
                    )
                if (outcome is Outcome.Failed) report(outcome.code)
            }
            busy = false
        }
    }

    val unlockWithBiometric: (QuickUnlockAvailability.Available) -> Unit = { available ->
        scope.launch {
            busy = true
            errorCode = null
            val outcome = session.unlockQuick(context, available, promptText)
            if (outcome is Outcome.Failed) report(outcome.code)
            busy = false
        }
    }

    ScreenFrame {
        TitleText(L10n.unlockTitle(resources))
        val email = state.email
        if (email != null) BodyText(L10n.unlockAccount(resources, email))
        if (quick is QuickUnlockAvailability.Available) {
            PrimaryButton(
                L10n.unlockQuick(resources),
                enabled = !busy,
                onClick = { unlockWithBiometric(quick) },
            )
        }
        FieldLabel(L10n.unlockPasswordLabel(resources))
        SecretField(
            state = passwordField,
            label = L10n.unlockPasswordLabel(resources),
            enabled = !busy,
            onSubmit = unlockWithPassword,
        )
        if (!state.secretKeyStored) {
            FieldLabel(L10n.unlockSecretKey(resources))
            SecretField(
                state = keyField,
                label = L10n.unlockSecretKey(resources),
                enabled = !busy,
            )
            HintText(L10n.unlockSecretKeyHelp(resources))
        }
        // After a restart or five wrong tries the fingerprint needs the password first: say so, and do not offer
        // to enrol again while the enrolment is still there.
        val passwordFirst = quick is QuickUnlockAvailability.PasswordRequired
        if (passwordFirst) HintText(errorText(resources, "passwordRequired"))
        if (biometricAvailable && quick !is QuickUnlockAvailability.Available && !passwordFirst) {
            CheckRow(
                checked = enrollNext,
                onCheckedChange = { enrollNext = it },
                label = L10n.unlockEnrollNext(resources),
                enabled = !busy,
            )
        }
        val code = errorCode
        if (code != null) ErrorText(errorText(resources, code))
        if (failures >= 2 && code == "wrongCredentials") HintText(L10n.unlockFailed(resources, failures))
        if (busy) {
            Working(L10n.unlockWorking(resources))
        } else if (quick is QuickUnlockAvailability.Available) {
            SecondaryButton(L10n.unlockAction(resources), onClick = unlockWithPassword)
        } else {
            PrimaryButton(L10n.unlockAction(resources), onClick = unlockWithPassword)
        }
    }
}
