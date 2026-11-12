package io.github.stephanbaum.mosaic.ui.onboarding

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.HandoffInfo
import io.github.stephanbaum.mosaic.ffi.HandoffStatus
import io.github.stephanbaum.mosaic.ffi.JoinOutcome
import io.github.stephanbaum.mosaic.ffi.SecondFactorMethods
import io.github.stephanbaum.mosaic.security.BiometricGate
import io.github.stephanbaum.mosaic.security.PromptText
import io.github.stephanbaum.mosaic.security.SecretField
import io.github.stephanbaum.mosaic.security.SecretFieldState
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.FieldLabel
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.QuietButton
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.SecondaryButton
import io.github.stephanbaum.mosaic.ui.components.TextInput
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.components.Working
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

private enum class Step { Welcome, Details, Password, Second, Quick, Autofill, Done }

/** The Secret Key typed in step 2, kept as bytes until the join no longer needs it (never a `String`). */
private class KeyHolder {
    var bytes: ByteArray? = null

    fun wipe() {
        bytes?.fill(0)
        bytes = null
    }
}

private val KEY_ERRORS = setOf("secretKeyMalformed", "secretKeyMistyped", "secretKeyRequired")
private val GIVE_UP_ERRORS = setOf("syncHandoffExpired", "noAccount", "accountExists")
private const val POLL_MS = 2000L

/**
 * The mobile onboarding of ADR-0096, typed setup (the scanner arrives with T13): welcome, account details,
 * master password, an optional second factor, quick unlock, autofill, done. Every step is one screen with one
 * primary action; the core owns the join (docs/MOBILE.md section 3.14).
 */
@Composable
fun OnboardingFlow(app: MosaicApp, onFinished: () -> Unit) {
    val context = LocalContext.current
    val resources = context.resources
    val scope = rememberCoroutineScope()
    val session = app.session

    var step by rememberSaveable { mutableStateOf(Step.Welcome) }
    var server by rememberSaveable { mutableStateOf("") }
    var origin by rememberSaveable { mutableStateOf("") }
    var email by rememberSaveable { mutableStateOf("") }
    val defaultDevice = L10n.onboardingDetailsDeviceNameDefault(resources)
    var deviceName by rememberSaveable { mutableStateOf(defaultDevice) }
    var busy by remember { mutableStateOf(false) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    var methods by remember { mutableStateOf<SecondFactorMethods?>(null) }
    var handoff by remember { mutableStateOf<HandoffInfo?>(null) }

    val keyHolder = remember { KeyHolder() }
    val keyField = remember { SecretFieldState() }
    val passwordField = remember { SecretFieldState() }
    val totpField = remember { SecretFieldState() }
    val enrollField = remember { SecretFieldState() }
    val gate = remember { BiometricGate() }
    val promptText =
        PromptText(
            title = L10n.unlockBiometricTitle(resources),
            subtitle = L10n.unlockBiometricSubtitle(resources),
            negativeButton = L10n.unlockUsePassword(resources),
        )

    DisposableEffect(Unit) {
        onDispose { keyHolder.wipe() }
    }

    // The security-key handoff: poll every 2 s while the app is in the foreground.
    LaunchedEffect(handoff) {
        if (handoff == null) return@LaunchedEffect
        while (isActive) {
            delay(POLL_MS)
            if (!app.foreground.value) continue
            when (val outcome = session.webauthnStatus()) {
                is Outcome.Ok ->
                    when (outcome.value) {
                        is HandoffStatus.Waiting -> Unit
                        is HandoffStatus.Done -> {
                            handoff = null
                            step = Step.Quick
                        }
                        is HandoffStatus.Failed -> {
                            handoff = null
                            errorCode = "syncTwoFactorInvalid"
                        }
                        is HandoffStatus.Expired -> {
                            handoff = null
                            errorCode = "syncHandoffExpired"
                        }
                    }
                is Outcome.Failed -> {
                    handoff = null
                    errorCode = outcome.code
                }
                Outcome.Cancelled -> Unit
            }
        }
    }

    val error = errorCode

    when (step) {
        Step.Welcome ->
            ScreenFrame {
                TitleText(L10n.onboardingWelcomeTitle(resources))
                BodyText(L10n.onboardingWelcomeBody(resources))
                PrimaryButton(L10n.onboardingWelcomeStart(resources), onClick = { step = Step.Details })
            }

        Step.Details ->
            ScreenFrame {
                TitleText(L10n.onboardingDetailsTitle(resources))
                TextInput(
                    value = server,
                    onValueChange = { server = it },
                    label = L10n.onboardingDetailsServer(resources),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                    enabled = !busy,
                )
                TextInput(
                    value = email,
                    onValueChange = { email = it },
                    label = L10n.onboardingDetailsEmail(resources),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
                    enabled = !busy,
                )
                FieldLabel(L10n.onboardingDetailsSecretKey(resources))
                SecretField(
                    state = keyField,
                    label = L10n.onboardingDetailsSecretKey(resources),
                    enabled = !busy,
                )
                HintText(L10n.onboardingDetailsSecretKeyHelp(resources))
                TextInput(
                    value = deviceName,
                    onValueChange = { deviceName = it },
                    label = L10n.onboardingDetailsDeviceName(resources),
                    enabled = !busy,
                )
                if (error != null) ErrorText(errorText(resources, error))
                if (busy) {
                    Working(L10n.onboardingDetailsChecking(resources))
                } else {
                    PrimaryButton(
                        L10n.commonContinue(resources),
                        onClick = {
                            scope.launch {
                                busy = true
                                errorCode = null
                                val typed = keyField.take()
                                if (typed != null) {
                                    keyHolder.wipe()
                                    keyHolder.bytes = typed.consume { it.copyOf() }
                                }
                                if (keyHolder.bytes == null) {
                                    errorCode = "secretKeyRequired"
                                } else {
                                    when (val outcome = session.checkServer(server.trim())) {
                                        is Outcome.Ok -> {
                                            origin = outcome.value.origin
                                            step = Step.Password
                                        }
                                        is Outcome.Failed -> errorCode = outcome.code
                                        Outcome.Cancelled -> Unit
                                    }
                                }
                                busy = false
                            }
                        },
                    )
                }
                QuietButton(L10n.commonBack(resources), onClick = { step = Step.Welcome }, enabled = !busy)
            }

        Step.Password ->
            ScreenFrame {
                TitleText(L10n.onboardingPasswordTitle(resources))
                BodyText(L10n.onboardingPasswordBody(resources))
                SecretField(
                    state = passwordField,
                    label = L10n.unlockPasswordLabel(resources),
                    enabled = !busy,
                )
                if (error != null) ErrorText(errorText(resources, error))
                if (busy) {
                    Working(L10n.onboardingPasswordWorking(resources))
                } else {
                    PrimaryButton(
                        L10n.onboardingPasswordAction(resources),
                        onClick = {
                            scope.launch {
                                busy = true
                                errorCode = null
                                val password = passwordField.take()
                                val key = keyHolder.bytes
                                if (password == null) {
                                    errorCode = "emptyPassword"
                                } else if (key == null) {
                                    password.wipe()
                                    errorCode = "secretKeyRequired"
                                    step = Step.Details
                                } else {
                                    when (val outcome = session.join(origin, email.trim(), deviceName, password, key)) {
                                        is Outcome.Ok -> {
                                            keyHolder.wipe()
                                            when (val joined = outcome.value) {
                                                is JoinOutcome.Unlocked -> step = Step.Quick
                                                is JoinOutcome.SecondFactor -> {
                                                    methods = joined.methods
                                                    step = Step.Second
                                                }
                                            }
                                        }
                                        is Outcome.Failed -> {
                                            errorCode = outcome.code
                                            if (outcome.code in KEY_ERRORS) step = Step.Details
                                        }
                                        Outcome.Cancelled -> Unit
                                    }
                                }
                                busy = false
                            }
                        },
                    )
                }
                QuietButton(L10n.commonBack(resources), onClick = { step = Step.Details }, enabled = !busy)
            }

        Step.Second -> {
            val offered = methods
            ScreenFrame {
                TitleText(L10n.onboardingSecondTitle(resources))
                if (offered?.totp == true) {
                    FieldLabel(L10n.onboardingSecondTotpLabel(resources))
                    SecretField(
                        state = totpField,
                        label = L10n.onboardingSecondTotpLabel(resources),
                        numeric = true,
                        enabled = !busy,
                    )
                    PrimaryButton(
                        L10n.onboardingSecondTotpAction(resources),
                        enabled = !busy,
                        onClick = {
                            scope.launch {
                                busy = true
                                errorCode = null
                                val typedCode = totpField.take()
                                if (typedCode == null) {
                                    errorCode = "syncTwoFactorInvalid"
                                } else {
                                    when (val outcome = session.joinTotp(typedCode)) {
                                        is Outcome.Ok -> step = Step.Quick
                                        is Outcome.Failed -> {
                                            errorCode = outcome.code
                                            if (outcome.code in GIVE_UP_ERRORS) step = Step.Details
                                        }
                                        Outcome.Cancelled -> Unit
                                    }
                                }
                                busy = false
                            }
                        },
                    )
                }
                if (offered?.webauthn == true) {
                    SecondaryButton(
                        L10n.onboardingSecondKeyAction(resources),
                        enabled = !busy && handoff == null,
                        onClick = {
                            scope.launch {
                                busy = true
                                errorCode = null
                                when (val outcome = session.webauthnBegin()) {
                                    is Outcome.Ok -> {
                                        handoff = outcome.value
                                        openInBrowser(context, outcome.value.url)
                                    }
                                    is Outcome.Failed -> errorCode = outcome.code
                                    Outcome.Cancelled -> Unit
                                }
                                busy = false
                            }
                        },
                    )
                }
                val waiting = handoff
                if (waiting != null) BodyText(L10n.onboardingSecondKeyWaiting(resources, waiting.matchCode))
                if (error != null) ErrorText(errorText(resources, error))
                QuietButton(
                    L10n.commonBack(resources),
                    onClick = {
                        scope.launch {
                            handoff = null
                            session.cancelJoin()
                            step = Step.Details
                        }
                    },
                )
            }
        }

        Step.Quick -> {
            val available = remember { gate.isAvailable(context) }
            ScreenFrame {
                TitleText(L10n.onboardingQuickTitle(resources))
                BodyText(L10n.onboardingQuickBody(resources))
                if (!available) {
                    HintText(L10n.onboardingQuickUnavailable(resources))
                } else {
                    SecretField(
                        state = enrollField,
                        label = L10n.unlockPasswordLabel(resources),
                        enabled = !busy,
                    )
                    if (error != null) ErrorText(errorText(resources, error))
                    if (busy) {
                        Working(L10n.unlockWorking(resources))
                    } else {
                        PrimaryButton(
                            L10n.onboardingQuickEnable(resources),
                            onClick = {
                                scope.launch {
                                    busy = true
                                    errorCode = null
                                    val password = enrollField.take()
                                    if (password == null) {
                                        errorCode = "emptyPassword"
                                    } else {
                                        when (val outcome = session.enrollQuickUnlock(context, password, promptText)) {
                                            is Outcome.Ok -> step = Step.Autofill
                                            is Outcome.Failed -> errorCode = outcome.code
                                            Outcome.Cancelled -> Unit
                                        }
                                    }
                                    busy = false
                                }
                            },
                        )
                    }
                }
                QuietButton(L10n.commonNotNow(resources), onClick = { step = Step.Autofill }, enabled = !busy)
            }
        }

        Step.Autofill ->
            ScreenFrame {
                TitleText(L10n.onboardingAutofillTitle(resources))
                BodyText(L10n.onboardingAutofillBody(resources))
                PrimaryButton(
                    L10n.onboardingAutofillOpen(resources),
                    onClick = {
                        openAutofillSettings(context)
                        step = Step.Done
                    },
                )
                QuietButton(L10n.commonNotNow(resources), onClick = { step = Step.Done })
            }

        Step.Done ->
            ScreenFrame {
                TitleText(L10n.onboardingDoneTitle(resources))
                BodyText(L10n.onboardingDoneBody(resources))
                PrimaryButton(L10n.onboardingDoneAction(resources), onClick = onFinished)
            }
    }
}

/** Opens the handoff page in the system browser; only an https address is opened. */
private fun openInBrowser(context: Context, url: String) {
    if (!url.startsWith("https://")) return
    try {
        context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
    } catch (e: ActivityNotFoundException) {
        // No browser: the waiting text and the Back button stay on screen.
    }
}

/** The system screen that makes Mosaic the autofill service (the service itself arrives with T9). */
private fun openAutofillSettings(context: Context) {
    val intent =
        Intent(Settings.ACTION_REQUEST_SET_AUTOFILL_SERVICE)
            .setData(Uri.parse("package:" + context.packageName))
    try {
        context.startActivity(intent)
    } catch (e: ActivityNotFoundException) {
        // The settings screen is missing on this device; the user can skip the step.
    }
}
