package io.github.stephanbaum.mosaic.ui.settings

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import android.view.autofill.AutofillManager
import androidx.credentials.CredentialManager
import kotlinx.coroutines.delay
import android.content.pm.PackageManager
import android.os.Build
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.MobileSettings
import io.github.stephanbaum.mosaic.ffi.SettingsUpdate
import io.github.stephanbaum.mosaic.ffi.SyncOverview
import io.github.stephanbaum.mosaic.security.BiometricGate
import io.github.stephanbaum.mosaic.security.PromptText
import io.github.stephanbaum.mosaic.security.SecretField
import io.github.stephanbaum.mosaic.security.SecretFieldState
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.ChoiceRow
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.FieldLabel
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.Line
import io.github.stephanbaum.mosaic.ui.components.PageColumn
import io.github.stephanbaum.mosaic.ui.components.Panel
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.QuietButton
import io.github.stephanbaum.mosaic.ui.components.SecondaryButton
import io.github.stephanbaum.mosaic.ui.components.SectionTitle
import io.github.stephanbaum.mosaic.ui.components.TopBar
import io.github.stephanbaum.mosaic.ui.components.Working
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import io.github.stephanbaum.mosaic.ui.vault.syncLine
import kotlinx.coroutines.launch

private val AUTO_LOCK_OPTIONS = listOf(0, 30, 60, 300)
private val IDLE_OPTIONS = listOf(1, 2, 5, 10, 15, 30)
private val CLIPBOARD_OPTIONS = listOf(10, 30, 60, 120, 300)

/** The installed version name. */
@Suppress("DEPRECATION")
fun appVersion(context: Context): String {
    val pm = context.packageManager
    val info =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            pm.getPackageInfo(context.packageName, PackageManager.PackageInfoFlags.of(0L))
        } else {
            pm.getPackageInfo(context.packageName, 0)
        }
    val name: String? = info.versionName
    return name.orEmpty()
}

private fun update(auto: Int? = null, idle: Int? = null, clipboard: Int? = null): SettingsUpdate =
    SettingsUpdate(
        autoLockBackgroundSeconds = auto?.toUInt(),
        idleLockMinutes = idle?.toUInt(),
        clipboardClearSeconds = clipboard?.toUInt(),
        quickUnlockMaxAgeDays = null,
        passkeyIdentities = null,
        passwordIdentities = null,
        reauthPassword = null,
    )

/** The settings tab: lock and clipboard times, quick unlock, sync, about. */
@Composable
fun SettingsScreen(app: MosaicApp, quickEnrolled: Boolean, onLicences: () -> Unit) {
    val context = LocalContext.current
    val resources = context.resources
    val scope = rememberCoroutineScope()
    var settings by remember { mutableStateOf<MobileSettings?>(null) }
    var overview by remember { mutableStateOf<SyncOverview?>(null) }
    var enrolled by remember { mutableStateOf(quickEnrolled) }
    var askPassword by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    val passwordState = remember { SecretFieldState() }
    val biometricAvailable = remember { BiometricGate().isAvailable(context) }

    LaunchedEffect(Unit) {
        val loaded = app.vault.settings()
        if (loaded is Outcome.Ok) settings = loaded.value
        val sync = app.vault.syncOverview()
        if (sync is Outcome.Ok) overview = sync.value
    }

    fun change(next: SettingsUpdate) {
        scope.launch {
            when (val result = app.vault.updateSettings(next)) {
                is Outcome.Ok -> {
                    settings = result.value
                    errorCode = null
                }
                is Outcome.Failed -> errorCode = result.code
                Outcome.Cancelled -> Unit
            }
        }
    }

    SettingsContent(
        settings = settings,
        syncText = syncLine(resources, overview, System.currentTimeMillis()),
        pendingUploads = overview?.pendingUploads?.toInt() ?: 0,
        quickEnrolled = enrolled,
        biometricAvailable = biometricAvailable,
        askPassword = askPassword,
        passwordState = passwordState,
        version = appVersion(context),
        busy = busy,
        error = errorCode?.let { errorText(resources, it) },
        onAutoLock = { change(update(auto = it)) },
        onIdle = { change(update(idle = it)) },
        onClipboard = { change(update(clipboard = it)) },
        onQuickStart = { askPassword = true },
        onQuickCancel = {
            passwordState.clear()
            askPassword = false
        },
        onQuickConfirm = {
            scope.launch {
                val password = passwordState.take()
                if (password == null) {
                    errorCode = "emptyPassword"
                    return@launch
                }
                busy = true
                errorCode = null
                val text =
                    PromptText(
                        title = L10n.unlockBiometricTitle(resources),
                        subtitle = L10n.unlockBiometricSubtitle(resources),
                        negativeButton = L10n.commonCancel(resources),
                    )
                when (val result = app.session.enrollQuickUnlock(context, password, text)) {
                    is Outcome.Ok -> {
                        enrolled = true
                        askPassword = false
                    }
                    is Outcome.Failed -> errorCode = result.code
                    Outcome.Cancelled -> Unit
                }
                busy = false
            }
        },
        onQuickDisable = {
            scope.launch {
                busy = true
                when (val result = app.session.removeQuickUnlock()) {
                    is Outcome.Ok -> {
                        enrolled = false
                        errorCode = null
                    }
                    is Outcome.Failed -> errorCode = result.code
                    Outcome.Cancelled -> Unit
                }
                busy = false
            }
        },
        onSync = {
            scope.launch {
                busy = true
                when (val result = app.vault.syncNow()) {
                    is Outcome.Ok -> {
                        overview = result.value
                        errorCode = null
                    }
                    is Outcome.Failed -> errorCode = result.code
                    Outcome.Cancelled -> Unit
                }
                busy = false
            }
        },
        onLock = { app.lockNow() },
        onLicences = onLicences,
    )
}

private fun withCurrent(options: List<Int>, current: Int): List<Int> = (options + current).distinct().sorted()

/** The stateless settings page (also drawn by the screenshot test). */
@Composable
fun SettingsContent(
    settings: MobileSettings?,
    syncText: String,
    pendingUploads: Int,
    quickEnrolled: Boolean,
    biometricAvailable: Boolean,
    askPassword: Boolean,
    passwordState: SecretFieldState,
    version: String,
    busy: Boolean,
    error: String?,
    onAutoLock: (Int) -> Unit,
    onIdle: (Int) -> Unit,
    onClipboard: (Int) -> Unit,
    onQuickStart: () -> Unit,
    onQuickConfirm: () -> Unit,
    onQuickCancel: () -> Unit,
    onQuickDisable: () -> Unit,
    onSync: () -> Unit,
    onLock: () -> Unit,
    onLicences: () -> Unit,
) {
    val resources = LocalContext.current.resources
    PageColumn {
        TopBar(title = L10n.settingsTitle(resources), onBack = null)
        if (error != null) ErrorText(error)
        Panel {
            SectionTitle(L10n.settingsSecurity(resources))
            if (settings != null) {
                val auto = settings.autoLockBackgroundSeconds.toInt()
                FieldLabel(L10n.settingsAutoLock(resources))
                for (seconds in withCurrent(AUTO_LOCK_OPTIONS, auto)) {
                    val label =
                        when (seconds) {
                            0 -> L10n.settingsAutoLockNow(resources)
                            30 -> L10n.settingsAutoLockS30(resources)
                            60 -> L10n.settingsAutoLockS60(resources)
                            300 -> L10n.settingsAutoLockS300(resources)
                            else -> L10n.settingsClipboardOption(resources, seconds)
                        }
                    ChoiceRow(selected = seconds == auto, label = label, onSelect = { onAutoLock(seconds) })
                }
                val idle = settings.idleLockMinutes.toInt()
                FieldLabel(L10n.settingsIdle(resources))
                for (minutes in withCurrent(IDLE_OPTIONS, idle)) {
                    ChoiceRow(selected = minutes == idle, label = L10n.settingsIdleOption(resources, minutes), onSelect = { onIdle(minutes) })
                }
                val clipboard = settings.clipboardClearSeconds.toInt()
                FieldLabel(L10n.settingsClipboard(resources))
                for (seconds in withCurrent(CLIPBOARD_OPTIONS, clipboard)) {
                    ChoiceRow(
                        selected = seconds == clipboard,
                        label = L10n.settingsClipboardOption(resources, seconds),
                        onSelect = { onClipboard(seconds) },
                    )
                }
            }
        }
        Panel {
            SectionTitle(L10n.settingsQuick(resources))
            if (quickEnrolled) {
                Line(L10n.settingsQuickOn(resources))
                SecondaryButton(L10n.settingsQuickDisable(resources), enabled = !busy, onClick = onQuickDisable)
            } else {
                Line(L10n.settingsQuickOff(resources))
                if (!biometricAvailable) {
                    HintText(L10n.onboardingQuickUnavailable(resources))
                } else if (askPassword) {
                    BodyText(L10n.settingsQuickPassword(resources))
                    SecretField(
                        state = passwordState,
                        label = L10n.unlockPasswordLabel(resources),
                        enabled = !busy,
                        onSubmit = onQuickConfirm,
                    )
                    PrimaryButton(L10n.settingsQuickEnable(resources), enabled = !busy, onClick = onQuickConfirm)
                    QuietButton(L10n.commonCancel(resources), enabled = !busy, onClick = onQuickCancel)
                } else {
                    SecondaryButton(L10n.settingsQuickEnable(resources), enabled = !busy, onClick = onQuickStart)
                }
            }
        }
        FillProvidersPanel()
        Panel {
            SectionTitle(L10n.settingsSync(resources))
            if (syncText.isNotEmpty()) Line(syncText)
            if (pendingUploads > 0) HintText(L10n.syncPending(resources, pendingUploads))
            if (busy) Working(L10n.syncWorking(resources)) else SecondaryButton(L10n.syncNow(resources), onClick = onSync)
        }
        Panel {
            SectionTitle(L10n.settingsAbout(resources))
            Line(L10n.settingsVersion(resources, version))
            BodyText(L10n.aboutPreAudit(resources))
            BodyText(L10n.aboutPrivacy(resources))
            SecondaryButton(L10n.aboutLicences(resources), onClick = onLicences)
        }
        Column(verticalArrangement = Arrangement.spacedBy(MosaicSpace.s3)) {
            PrimaryButton(L10n.lockNow(resources), onClick = onLock)
        }
    }
}

/** The licences of the parts Mosaic for Android is built from. */
@Composable
fun LicencesScreen(onBack: () -> Unit) {
    val resources = LocalContext.current.resources
    PageColumn {
        TopBar(title = L10n.licencesTitle(resources), onBack = onBack)
        BodyText(L10n.licencesIntro(resources))
        for ((name, licence) in LICENCES) {
            Panel {
                Line(name)
                HintText(licence)
            }
        }
        BodyText(L10n.licencesRust(resources))
    }
}

private val LICENCES =
    listOf(
        "DM Sans" to "SIL Open Font License 1.1",
        "IBM Plex Mono" to "SIL Open Font License 1.1",
        "Jetpack Compose, AndroidX" to "Apache License 2.0",
        "Kotlin, kotlinx.coroutines" to "Apache License 2.0",
        "JNA" to "Apache License 2.0 or LGPL 2.1 or later",
    )

/**
 * Enabling Mosaic as the Autofill service and as the Credential Manager provider happens in system screens; these
 * entries open them. The Autofill state is shown (it is exposed); the provider state is not, and its entry exists
 * from API 34 only. The state is read again every second while the page is shown, so it is right after the user
 * comes back from the system screen.
 */
@Composable
private fun FillProvidersPanel() {
    val context = LocalContext.current
    val resources = context.resources
    var autofillOn by remember { mutableStateOf(autofillEnabled(context)) }
    LaunchedEffect(Unit) {
        while (true) {
            autofillOn = autofillEnabled(context)
            delay(1_000)
        }
    }
    Panel {
        SectionTitle(L10n.settingsFill(resources))
        Line(if (autofillOn) L10n.settingsAutofillOn(resources) else L10n.settingsAutofillOff(resources))
        if (!autofillOn) {
            SecondaryButton(L10n.settingsAutofillEnable(resources), onClick = { openAutofillSettings(context) })
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            HintText(L10n.settingsCredentialHint(resources))
            SecondaryButton(L10n.settingsCredentialEnable(resources), onClick = { openCredentialSettings(context) })
        }
    }
}

private fun autofillEnabled(context: Context): Boolean =
    context.getSystemService(AutofillManager::class.java)?.hasEnabledAutofillServices() ?: false

private fun openAutofillSettings(context: Context) {
    val intent =
        Intent(Settings.ACTION_REQUEST_SET_AUTOFILL_SERVICE, Uri.parse("package:" + context.packageName))
    try {
        context.startActivity(intent)
    } catch (e: android.content.ActivityNotFoundException) {
        // A phone without the screen: the user enables it in the system settings by hand.
    }
}

private fun openCredentialSettings(context: Context) {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.UPSIDE_DOWN_CAKE) return
    try {
        CredentialManager.create(context).createSettingsPendingIntent().send()
    } catch (e: android.app.PendingIntent.CanceledException) {
        // The system screen is gone; nothing to do.
    }
}
