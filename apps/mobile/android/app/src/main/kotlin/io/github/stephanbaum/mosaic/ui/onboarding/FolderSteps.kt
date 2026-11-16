package io.github.stephanbaum.mosaic.ui.onboarding

import android.content.res.Resources
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.KeyboardType
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.FolderCandidate
import io.github.stephanbaum.mosaic.ffi.FolderDirEntry
import io.github.stephanbaum.mosaic.ffi.FolderProvider
import io.github.stephanbaum.mosaic.ffi.LinkKind
import io.github.stephanbaum.mosaic.ffi.LinkStatus
import io.github.stephanbaum.mosaic.security.SecretField
import io.github.stephanbaum.mosaic.security.SecretFieldState
import io.github.stephanbaum.mosaic.session.FolderLinkEvent
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.ChoiceRow
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
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/** How often a Nextcloud sign-in is asked whether the person finished it in the browser. */
private const val FOLDER_POLL_MS = 2000L

/** How many characters of an account ID the person sees. */
private const val ACCOUNT_SHORT = 8

/** The name of a provider as the person knows it. */
internal fun providerName(resources: Resources, provider: FolderProvider): String =
    when (provider) {
        FolderProvider.DROPBOX -> L10n.folderProviderDropbox(resources)
        FolderProvider.ONE_DRIVE -> L10n.folderProviderOneDrive(resources)
        FolderProvider.WEB_DAV -> L10n.folderProviderWebDav(resources)
    }

/** The provider a saved name stands for, or null. */
internal fun providerOf(name: String?): FolderProvider? = FolderProvider.entries.firstOrNull { it.name == name }

/**
 * The provider sign-in, shared by the onboarding and the settings ("Sign in again"): a button that opens the
 * provider's page in the system browser (https only), the wait for its end (the redirect for Dropbox and OneDrive,
 * a poll every two seconds for Nextcloud), and for WebDAV an app-password form. [onLinked] runs once the core
 * holds the link. Tokens, passwords and addresses with codes never reach a log or a string resource.
 */
@Composable
fun FolderSignInPanel(
    app: MosaicApp,
    provider: FolderProvider,
    server: String,
    serverEditable: Boolean,
    onLinked: () -> Unit,
    onCancel: (() -> Unit)? = null,
    dropOnLeave: Boolean = false,
) {
    val context = LocalContext.current
    val resources = context.resources
    val scope = rememberCoroutineScope()
    val session = app.session
    val name = providerName(resources, provider)
    val webDav = provider == FolderProvider.WEB_DAV
    val linked by rememberUpdatedState(onLinked)

    var serverText by rememberSaveable { mutableStateOf(server) }
    var davText by rememberSaveable { mutableStateOf(server) }
    var login by rememberSaveable { mutableStateOf("") }
    var usePassword by rememberSaveable { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var waiting by remember { mutableStateOf(false) }
    var polling by remember { mutableStateOf(false) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    val passwordField = remember { SecretFieldState() }
    var finished by remember { mutableStateOf(false) }

    // A sign in again that is left unfinished must not keep the app from locking: drop the pending link.
    DisposableEffect(Unit) {
        onDispose { if (dropOnLeave && !finished) session.abandonFolderLink() }
    }

    // The end of a redirect sign-in, delivered by the redirect activity.
    LaunchedEffect(Unit) {
        session.folderLink.collect { event ->
            when (event) {
                FolderLinkEvent.Linked -> {
                    session.clearFolderLinkEvent()
                    waiting = false
                    finished = true
                    linked()
                }
                is FolderLinkEvent.Failed -> {
                    session.clearFolderLinkEvent()
                    waiting = false
                    errorCode = event.code
                }
                null -> Unit
            }
        }
    }

    // A Nextcloud sign-in ends in the browser without a redirect to the app: ask every 2 s while in the foreground.
    LaunchedEffect(polling) {
        if (!polling) return@LaunchedEffect
        while (isActive) {
            delay(FOLDER_POLL_MS)
            if (!app.foreground.value) continue
            when (val outcome = session.folderLinkStatus()) {
                is Outcome.Ok ->
                    when (outcome.value) {
                        LinkStatus.WAITING -> Unit
                        LinkStatus.LINKED -> {
                            polling = false
                            waiting = false
                            finished = true
                    linked()
                        }
                        LinkStatus.EXPIRED -> {
                            polling = false
                            waiting = false
                            errorCode = "folderLinkExpired"
                        }
                    }
                is Outcome.Failed -> {
                    polling = false
                    waiting = false
                    errorCode = outcome.code
                }
                Outcome.Cancelled -> Unit
            }
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(MosaicSpace.s5)) {
        if (webDav && serverEditable) {
            TextInput(
                value = serverText,
                onValueChange = { serverText = it },
                label = L10n.folderServerLabel(resources),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                enabled = !busy,
                placeholder = L10n.folderServerHint(resources),
            )
        }
        if (!webDav) HintText(L10n.folderScope(resources, name))
        if (waiting) BodyText(L10n.folderWaiting(resources))
        val error = errorCode
        if (error != null) ErrorText(errorText(resources, error))
        if (busy) {
            Working(L10n.commonWorking(resources))
        } else {
            PrimaryButton(
                L10n.folderSignIn(resources, name),
                enabled = !webDav || serverText.isNotBlank(),
                onClick = {
                    scope.launch {
                        busy = true
                        errorCode = null
                        val origin = if (webDav) serverText.trim() else null
                        when (val outcome = session.folderLinkBegin(provider, origin)) {
                            is Outcome.Ok -> {
                                waiting = true
                                polling = outcome.value.kind == LinkKind.POLL
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
        if (webDav) {
            SecondaryButton(
                L10n.folderAppPasswordToggle(resources),
                enabled = !busy,
                onClick = { usePassword = !usePassword },
            )
            if (usePassword) {
                TextInput(
                    value = davText,
                    onValueChange = { davText = it },
                    label = L10n.folderDavLabel(resources),
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                    enabled = !busy,
                    placeholder = L10n.folderServerHint(resources),
                )
                TextInput(
                    value = login,
                    onValueChange = { login = it },
                    label = L10n.folderLoginLabel(resources),
                    enabled = !busy,
                )
                FieldLabel(L10n.folderAppPasswordLabel(resources))
                SecretField(
                    state = passwordField,
                    label = L10n.folderAppPasswordLabel(resources),
                    enabled = !busy,
                )
                HintText(L10n.folderAppPasswordHint(resources))
                PrimaryButton(
                    L10n.folderAppPasswordAction(resources),
                    enabled = !busy && davText.isNotBlank() && login.isNotBlank(),
                    onClick = {
                        scope.launch {
                            busy = true
                            errorCode = null
                            val password = passwordField.take()
                            if (password == null) {
                                errorCode = "emptyPassword"
                            } else {
                                when (val outcome = session.folderLinkPassword(davText.trim(), login.trim(), password)) {
                                    is Outcome.Ok -> {
                                        waiting = false
                                        finished = true
                    linked()
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
        }
        if (onCancel != null) QuietButton(L10n.commonCancel(resources), onClick = onCancel)
    }
}

/** The provider choice of "use an existing account" (and of a setup code that names no provider). */
@Composable
fun FolderChooseContent(
    provider: FolderProvider?,
    onProvider: (FolderProvider) -> Unit,
    server: String,
    onServer: (String) -> Unit,
    keyField: SecretFieldState,
    keyKept: Boolean,
    error: String?,
    busy: Boolean,
    onContinue: () -> Unit,
    onBack: () -> Unit,
) {
    val resources = LocalContext.current.resources
    ScreenFrame {
        TitleText(L10n.folderChooseProvider(resources))
        for (option in FolderProvider.entries) {
            ChoiceRow(
                selected = provider == option,
                label = providerName(resources, option),
                onSelect = { onProvider(option) },
                enabled = !busy,
            )
        }
        HintText(L10n.folderNoDrive(resources))
        if (provider == FolderProvider.WEB_DAV) {
            TextInput(
                value = server,
                onValueChange = onServer,
                label = L10n.folderServerLabel(resources),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                enabled = !busy,
                placeholder = L10n.folderServerHint(resources),
            )
        }
        FieldLabel(L10n.onboardingDetailsSecretKey(resources))
        SecretField(state = keyField, label = L10n.onboardingDetailsSecretKey(resources), enabled = !busy)
        if (keyKept) HintText(L10n.onboardingDetailsKeyFromCode(resources))
        HintText(L10n.onboardingDetailsSecretKeyHelp(resources))
        if (error != null) ErrorText(errorText(resources, error))
        PrimaryButton(
            L10n.commonContinue(resources),
            onClick = onContinue,
            enabled = provider != null && !busy,
        )
        QuietButton(L10n.commonBack(resources), onClick = onBack, enabled = !busy)
    }
}

private enum class FolderStep { Intro, SignIn, Locate, Browse, PickAccount }

/**
 * What comes after the provider is known: the intro (and, for WebDAV, the confirmation of the server), the
 * sign-in, the look for the Mosaic folder (the hint of a setup code, else a browse of the drive), and the choice
 * of an account when the folder holds several. The phone never creates a folder: it only reads. [onChosen] gets
 * the folder and the account; the master password step joins. The sign-in stays pending in the core until then.
 */
@Composable
fun FolderJoinFlow(
    app: MosaicApp,
    provider: FolderProvider,
    server: String,
    pathHint: String?,
    accountHint: String?,
    onChosen: (path: String, account: String) -> Unit,
    onBack: () -> Unit,
) {
    val resources = LocalContext.current.resources
    val scope = rememberCoroutineScope()
    val session = app.session
    val name = providerName(resources, provider)
    val chosen by rememberUpdatedState(onChosen)

    var step by remember { mutableStateOf(FolderStep.Intro) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var dir by remember { mutableStateOf("") }
    var entries by remember { mutableStateOf<List<FolderDirEntry>?>(null) }
    var pickPath by remember { mutableStateOf("") }
    var pickAccounts by remember { mutableStateOf<List<String>>(emptyList()) }
    var picked by remember { mutableStateOf<String?>(null) }

    /** A found folder: the hinted account, the only account, or a choice; a folder without it is a wrong folder. */
    fun choose(found: FolderCandidate) {
        val accounts = found.accounts
        val wanted = accountHint
        when {
            wanted != null && wanted in accounts -> chosen(found.path, wanted)
            wanted != null || accounts.isEmpty() -> {
                errorCode = "syncFolderNoAccount"
                dir = ""
                step = FolderStep.Browse
            }
            accounts.size == 1 -> chosen(found.path, accounts[0])
            else -> {
                pickPath = found.path
                pickAccounts = accounts
                picked = null
                step = FolderStep.PickAccount
            }
        }
    }

    /** Back out of the sign-in for good: the pending link is dropped, so the next try starts clean. */
    fun dropLinkThen(next: () -> Unit) {
        scope.launch {
            session.folderLinkCancel()
            next()
        }
    }

    // The look at the hinted folder (or the drive root), right after the sign-in.
    LaunchedEffect(step) {
        if (step != FolderStep.Locate) return@LaunchedEffect
        when (val outcome = session.folderLocate(pathHint)) {
            is Outcome.Ok -> {
                val found = outcome.value.firstOrNull()
                if (found != null) {
                    choose(found)
                } else {
                    dir = ""
                    step = FolderStep.Browse
                }
            }
            is Outcome.Failed -> {
                errorCode = outcome.code
                step = FolderStep.SignIn
            }
            Outcome.Cancelled -> Unit
        }
    }

    // The folders below the current one.
    LaunchedEffect(step, dir) {
        if (step != FolderStep.Browse) return@LaunchedEffect
        entries = null
        when (val outcome = session.folderBrowse(dir)) {
            is Outcome.Ok -> entries = outcome.value
            is Outcome.Failed -> {
                errorCode = outcome.code
                entries = emptyList()
            }
            Outcome.Cancelled -> Unit
        }
    }

    when (step) {
        FolderStep.Intro ->
            ScreenFrame {
                TitleText(L10n.folderIntro(resources, name))
                if (provider == FolderProvider.WEB_DAV) {
                    BodyText(L10n.folderServerConfirm(resources, server))
                } else {
                    BodyText(L10n.folderScope(resources, name))
                }
                PrimaryButton(L10n.commonContinue(resources), onClick = { step = FolderStep.SignIn })
                QuietButton(L10n.commonBack(resources), onClick = { dropLinkThen(onBack) })
            }

        FolderStep.SignIn ->
            ScreenFrame {
                TitleText(L10n.folderSignIn(resources, name))
                val error = errorCode
                if (error != null) ErrorText(errorText(resources, error))
                FolderSignInPanel(
                    app = app,
                    provider = provider,
                    server = server,
                    serverEditable = false,
                    onLinked = {
                        errorCode = null
                        step = FolderStep.Locate
                    },
                )
                QuietButton(
                    L10n.commonBack(resources),
                    onClick = { dropLinkThen { step = FolderStep.Intro } },
                )
            }

        FolderStep.Locate ->
            ScreenFrame {
                Working(L10n.folderLocating(resources))
            }

        FolderStep.Browse ->
            ScreenFrame {
                TitleText(L10n.folderBrowseTitle(resources))
                BodyText(L10n.folderBrowseHint(resources))
                HintText(L10n.folderCurrentFolder(resources, "/$dir"))
                val error = errorCode
                if (error != null) ErrorText(errorText(resources, error))
                val list = entries
                if (list == null) {
                    Working(L10n.folderLocating(resources))
                } else {
                    if (list.isEmpty()) HintText(L10n.folderNoSubfolders(resources))
                    for (entry in list) {
                        SecondaryButton(
                            entry.name,
                            enabled = !busy,
                            onClick = {
                                errorCode = null
                                dir = entry.path
                            },
                        )
                    }
                }
                if (busy) {
                    Working(L10n.folderLocating(resources))
                } else {
                    PrimaryButton(
                        L10n.folderUseThis(resources),
                        enabled = list != null,
                        onClick = {
                            scope.launch {
                                busy = true
                                errorCode = null
                                when (val outcome = session.folderLocate(dir)) {
                                    is Outcome.Ok -> {
                                        val found = outcome.value.firstOrNull()
                                        if (found == null) errorCode = "syncFolderNoAccount" else choose(found)
                                    }
                                    is Outcome.Failed -> errorCode = outcome.code
                                    Outcome.Cancelled -> Unit
                                }
                                busy = false
                            }
                        },
                    )
                }
                if (dir.isNotEmpty()) {
                    QuietButton(
                        L10n.folderUp(resources),
                        enabled = !busy,
                        onClick = {
                            errorCode = null
                            dir = dir.substringBeforeLast('/', "")
                        },
                    )
                }
                QuietButton(
                    L10n.commonBack(resources),
                    enabled = !busy,
                    onClick = { dropLinkThen { step = FolderStep.SignIn } },
                )
            }

        FolderStep.PickAccount ->
            ScreenFrame {
                TitleText(L10n.folderPickAccount(resources))
                for (account in pickAccounts) {
                    ChoiceRow(
                        selected = picked == account,
                        label = L10n.folderAccountOption(resources, account.take(ACCOUNT_SHORT)),
                        onSelect = { picked = account },
                    )
                }
                PrimaryButton(
                    L10n.commonContinue(resources),
                    enabled = picked != null,
                    onClick = { picked?.let { chosen(pickPath, it) } },
                )
                QuietButton(
                    L10n.commonBack(resources),
                    onClick = { dropLinkThen { step = FolderStep.SignIn } },
                )
            }
    }
}
