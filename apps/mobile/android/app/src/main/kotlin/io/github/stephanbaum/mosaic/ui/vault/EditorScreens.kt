package io.github.stephanbaum.mosaic.ui.vault

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Slider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.core.SecretBytes
import io.github.stephanbaum.mosaic.ffi.GeneratorMode
import io.github.stephanbaum.mosaic.ffi.GeneratorOptions
import io.github.stephanbaum.mosaic.ffi.ItemDetail
import io.github.stephanbaum.mosaic.ffi.ItemKey
import io.github.stephanbaum.mosaic.ffi.LoginDraft
import io.github.stephanbaum.mosaic.ffi.NoteDraft
import io.github.stephanbaum.mosaic.ffi.OtpEdit
import io.github.stephanbaum.mosaic.ffi.SecretValue
import io.github.stephanbaum.mosaic.ffi.UrlDraft
import io.github.stephanbaum.mosaic.scan.ScanLayout
import io.github.stephanbaum.mosaic.scan.ScanScreen
import io.github.stephanbaum.mosaic.scan.ScanTarget
import io.github.stephanbaum.mosaic.security.SecretField
import io.github.stephanbaum.mosaic.security.SecretFieldState
import io.github.stephanbaum.mosaic.security.SecretText
import io.github.stephanbaum.mosaic.security.SensitiveWhile
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.components.ActionButton
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.CheckRow
import io.github.stephanbaum.mosaic.ui.components.ChoiceRow
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.FieldLabel
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.Line
import io.github.stephanbaum.mosaic.ui.components.PageColumn
import io.github.stephanbaum.mosaic.ui.components.Panel
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.SecondaryButton
import io.github.stephanbaum.mosaic.ui.components.SectionTitle
import io.github.stephanbaum.mosaic.ui.components.TextInput
import io.github.stephanbaum.mosaic.ui.components.TopBar
import io.github.stephanbaum.mosaic.ui.components.Working
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

private class UrlEntry(val url: String, val matchEtld1: Boolean, val allowHttp: Boolean, val isApp: Boolean)

private fun fieldValue(detail: ItemDetail, key: String): String? = detail.fields.firstOrNull { it.key == key }?.value

/** A typed address gets `https://` when it has no scheme. */
private fun normalizeUrl(typed: String): String {
    val trimmed = typed.trim()
    return if (trimmed.contains("://")) trimmed else "https://$trimmed"
}

/** The vault a new item goes to: the first available one. */
private suspend fun firstVault(app: MosaicApp): String? {
    val result = app.vault.vaults()
    return if (result is Outcome.Ok) result.value.firstOrNull { it.available }?.vaultId else null
}

/** The login editor: title, username, password with the generator, websites, notes, a one-time code. */
@Composable
fun LoginEditorScreen(
    app: MosaicApp,
    vaultId: String?,
    itemId: String?,
    onSaved: (ItemKey) -> Unit,
    onCancel: () -> Unit,
) {
    val resources = LocalContext.current.resources
    val scope = rememberCoroutineScope()
    var loaded by remember { mutableStateOf(itemId == null) }
    var target by remember { mutableStateOf(vaultId) }
    // Vault metadata stays out of the saved instance state (the bundle is not wiped): `remember`, not `rememberSaveable`.
    var title by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("") }
    var favorite by rememberSaveable { mutableStateOf(false) }
    var hadNotes by remember { mutableStateOf(false) }
    var hasOtp by remember { mutableStateOf(false) }
    var otpRemoved by remember { mutableStateOf(false) }
    var pendingId by remember { mutableStateOf<String?>(null) }
    var pendingName by remember { mutableStateOf("") }
    var showGenerator by remember { mutableStateOf(false) }
    var scanning by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    val urls = remember { mutableStateListOf<UrlEntry>() }
    val passwordState = remember { SecretFieldState() }
    val notesState = remember { SecretFieldState() }
    val otpState = remember { SecretFieldState() }

    LaunchedEffect(itemId) {
        if (itemId != null && vaultId != null) {
            when (val result = app.vault.item(vaultId, itemId)) {
                is Outcome.Ok -> {
                    val detail = result.value
                    title = detail.summary.title
                    username = fieldValue(detail, "login.username").orEmpty()
                    favorite = detail.summary.favorite
                    val notes = fieldValue(detail, "notes").orEmpty()
                    hadNotes = notes.isNotEmpty()
                    if (hadNotes) notesState.set(notes)
                    hasOtp = detail.otp != null
                    urls.clear()
                    for (url in detail.urls) urls.add(UrlEntry(url.url, url.matchEtld1, url.allowHttp, url.isApp))
                    loaded = true
                }
                is Outcome.Failed -> errorCode = result.code
                Outcome.Cancelled -> Unit
            }
        } else if (target == null) {
            target = firstVault(app)
            if (target == null) errorCode = "vaultUnavailable"
        }
    }

    fun addOtp() {
        scope.launch {
            val typed = otpState.take() ?: return@launch
            busy = true
            when (val result = app.vault.otpFromText(typed)) {
                is Outcome.Ok -> {
                    pendingId = result.value.pendingId
                    pendingName = result.value.issuer ?: result.value.accountName ?: ""
                    otpRemoved = false
                    errorCode = null
                }
                is Outcome.Failed -> errorCode = result.code
                Outcome.Cancelled -> Unit
            }
            busy = false
        }
    }

    fun save() {
        scope.launch {
            val vault = target
            if (vault == null) {
                errorCode = "vaultUnavailable"
                return@launch
            }
            busy = true
            errorCode = null
            val password = passwordState.take()
            val notes = notesState.take() ?: if (hadNotes) SecretBytes.of(ByteArray(0)) else null
            val pending = pendingId
            val otpEdit: OtpEdit =
                if (pending != null) {
                    OtpEdit.SetPending(pending)
                } else if (otpRemoved) {
                    OtpEdit.Remove
                } else {
                    OtpEdit.Keep
                }
            val drafts =
                urls.filter { it.isApp || it.url.isNotBlank() }
                    .map { UrlDraft(if (it.isApp) it.url else normalizeUrl(it.url), it.matchEtld1, it.allowHttp) }
            val draft =
                LoginDraft(
                    vaultId = vault,
                    itemId = itemId,
                    title = title.trim(),
                    username = username.trim(),
                    password = null,
                    urls = drafts,
                    notes = null,
                    otp = otpEdit,
                    favorite = favorite,
                )
            when (val result = app.vault.saveLogin(draft, password, notes)) {
                is Outcome.Ok -> onSaved(result.value)
                is Outcome.Failed -> errorCode = result.code
                Outcome.Cancelled -> Unit
            }
            busy = false
        }
    }

    PageColumn {
        TopBar(
            title = if (itemId == null) L10n.editorNewLogin(resources) else L10n.editorEditLogin(resources),
            onBack = onCancel,
        )
        if (!loaded) {
            val failure = errorCode
            if (failure != null) ErrorText(errorText(resources, failure)) else Working(L10n.commonWorking(resources))
        } else {
            FieldLabel(L10n.editorTitle(resources))
            TextInput(value = title, onValueChange = { title = it }, label = L10n.editorTitle(resources), enabled = !busy)
            FieldLabel(L10n.editorUsername(resources))
            TextInput(
                value = username,
                onValueChange = { username = it },
                label = L10n.editorUsername(resources),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Email),
                enabled = !busy,
            )
            FieldLabel(L10n.editorPassword(resources))
            SecretField(state = passwordState, label = L10n.editorPassword(resources), enabled = !busy)
            if (itemId != null) HintText(L10n.editorPasswordKeep(resources))
            SecondaryButton(L10n.editorGenerate(resources), onClick = { showGenerator = !showGenerator })
            if (showGenerator) {
                GeneratorPanel(
                    app = app,
                    onUse = { bytes ->
                        passwordState.put(bytes)
                        showGenerator = false
                    },
                )
            }
            SectionTitle(L10n.editorWebsites(resources))
            for (index in urls.indices) {
                val entry = urls[index]
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s3)) {
                    if (entry.isApp) {
                        Line(L10n.detailLinkedApp(resources, appUrlName(entry.url)), modifier = Modifier.weight(1f))
                    } else {
                        Box(modifier = Modifier.weight(1f)) {
                            TextInput(
                                value = entry.url,
                                onValueChange = { urls[index] = UrlEntry(it, entry.matchEtld1, entry.allowHttp, false) },
                                label = L10n.editorWebsiteHint(resources),
                                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri),
                                enabled = !busy,
                            )
                        }
                    }
                    ActionButton(
                        text = L10n.commonRemove(resources),
                        description = L10n.editorWebsiteRemove(resources, (index + 1).toString()),
                        onClick = { urls.removeAt(index) },
                    )
                }
            }
            SecondaryButton(L10n.editorWebsiteAdd(resources), onClick = { urls.add(UrlEntry("", false, false, false)) })
            FieldLabel(L10n.editorNotes(resources))
            SecretField(state = notesState, label = L10n.editorNotes(resources), multiline = true, enabled = !busy)
            SectionTitle(L10n.editorOtpTitle(resources))
            val ready = pendingId
            if (ready != null) {
                BodyText(L10n.editorOtpReady(resources, pendingName))
            } else if (hasOtp && !otpRemoved) {
                BodyText(L10n.editorOtpHave(resources))
                SecondaryButton(L10n.editorOtpRemove(resources), onClick = { otpRemoved = true })
            } else if (otpRemoved) {
                BodyText(L10n.editorOtpRemoved(resources))
            }
            if (ready == null && scanning) {
                ScanScreen(
                    app = app,
                    target = ScanTarget.Otp,
                    layout = ScanLayout.Embedded,
                    onSetupCode = {},
                    onOtp = { preview ->
                        pendingId = preview.pendingId
                        pendingName = preview.issuer ?: preview.accountName ?: ""
                        otpRemoved = false
                        errorCode = null
                        scanning = false
                    },
                    onCancel = { scanning = false },
                )
            } else if (ready == null) {
                SecretField(state = otpState, label = L10n.editorOtpInput(resources), enabled = !busy, onSubmit = { addOtp() })
                SecondaryButton(L10n.editorOtpAdd(resources), enabled = !busy, onClick = { addOtp() })
                SecondaryButton(L10n.scanOtpAction(resources), enabled = !busy, onClick = { scanning = true })
            }
            CheckRow(checked = favorite, onCheckedChange = { favorite = it }, label = L10n.editorFavorite(resources), enabled = !busy)
            val failure = errorCode
            if (failure != null) ErrorText(errorText(resources, failure))
            if (busy) Working(L10n.editorSaving(resources)) else PrimaryButton(L10n.commonSave(resources), onClick = { save() })
        }
    }
}

/** The note editor: title, the note text, a favorite mark. */
@Composable
fun NoteEditorScreen(
    app: MosaicApp,
    vaultId: String?,
    itemId: String?,
    onSaved: (ItemKey) -> Unit,
    onCancel: () -> Unit,
) {
    val resources = LocalContext.current.resources
    val scope = rememberCoroutineScope()
    var loaded by remember { mutableStateOf(itemId == null) }
    var target by remember { mutableStateOf(vaultId) }
    var title by remember { mutableStateOf("") }
    var favorite by rememberSaveable { mutableStateOf(false) }
    var hadNotes by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    val notesState = remember { SecretFieldState() }

    LaunchedEffect(itemId) {
        if (itemId != null && vaultId != null) {
            when (val result = app.vault.item(vaultId, itemId)) {
                is Outcome.Ok -> {
                    val detail = result.value
                    title = detail.summary.title
                    favorite = detail.summary.favorite
                    val notes = fieldValue(detail, "notes").orEmpty()
                    hadNotes = notes.isNotEmpty()
                    if (hadNotes) notesState.set(notes)
                    loaded = true
                }
                is Outcome.Failed -> errorCode = result.code
                Outcome.Cancelled -> Unit
            }
        } else if (target == null) {
            target = firstVault(app)
            if (target == null) errorCode = "vaultUnavailable"
        }
    }

    fun save() {
        scope.launch {
            val vault = target
            if (vault == null) {
                errorCode = "vaultUnavailable"
                return@launch
            }
            busy = true
            errorCode = null
            val notes = notesState.take() ?: if (hadNotes) SecretBytes.of(ByteArray(0)) else null
            val draft = NoteDraft(vaultId = vault, itemId = itemId, title = title.trim(), notes = null, favorite = favorite)
            when (val result = app.vault.saveNote(draft, notes)) {
                is Outcome.Ok -> onSaved(result.value)
                is Outcome.Failed -> errorCode = result.code
                Outcome.Cancelled -> Unit
            }
            busy = false
        }
    }

    PageColumn {
        TopBar(
            title = if (itemId == null) L10n.editorNewNote(resources) else L10n.editorEditNote(resources),
            onBack = onCancel,
        )
        if (!loaded) {
            val failure = errorCode
            if (failure != null) ErrorText(errorText(resources, failure)) else Working(L10n.commonWorking(resources))
        } else {
            FieldLabel(L10n.editorTitle(resources))
            TextInput(value = title, onValueChange = { title = it }, label = L10n.editorTitle(resources), enabled = !busy)
            FieldLabel(L10n.editorNotes(resources))
            SecretField(state = notesState, label = L10n.editorNotes(resources), multiline = true, enabled = !busy)
            CheckRow(checked = favorite, onCheckedChange = { favorite = it }, label = L10n.editorFavorite(resources), enabled = !busy)
            val failure = errorCode
            if (failure != null) ErrorText(errorText(resources, failure))
            if (busy) Working(L10n.editorSaving(resources)) else PrimaryButton(L10n.commonSave(resources), onClick = { save() })
        }
    }
}

private fun defaultOptions(): GeneratorOptions =
    GeneratorOptions(
        mode = GeneratorMode.PASSWORD,
        length = 20u,
        lowercase = true,
        uppercase = true,
        digits = true,
        symbols = true,
        excludeAmbiguous = false,
        words = 5u,
        separator = "-",
        capitalize = true,
        appendDigit = false,
    )

private class GeneratedHolder {
    var secret: SecretValue? = null

    fun replace(next: SecretValue?) {
        secret?.wipe()
        secret = next
    }
}

/** The generator: options, a result with its strength, and "use". The value never leaves the core as a `String`
 *  except for this preview, which is dropped when the panel closes. */
@Composable
private fun GeneratorPanel(app: MosaicApp, onUse: (ByteArray) -> Unit) {
    val resources = LocalContext.current.resources
    val scope = rememberCoroutineScope()
    var options by remember { mutableStateOf(defaultOptions()) }
    var preview by remember { mutableStateOf<String?>(null) }
    var bits by remember { mutableStateOf(0) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    val holder = remember { GeneratedHolder() }

    DisposableEffect(Unit) {
        onDispose { holder.replace(null) }
    }
    // The preview shows a password: the window is sensitive while it is up (M10-G2 L7).
    SensitiveWhile(preview != null)

    Panel {
        SectionTitle(L10n.generatorTitle(resources))
        ChoiceRow(
            selected = options.mode == GeneratorMode.PASSWORD,
            label = L10n.generatorPassword(resources),
            onSelect = { options = options.copy(mode = GeneratorMode.PASSWORD) },
        )
        ChoiceRow(
            selected = options.mode == GeneratorMode.PASSPHRASE,
            label = L10n.generatorPassphrase(resources),
            onSelect = { options = options.copy(mode = GeneratorMode.PASSPHRASE) },
        )
        if (options.mode == GeneratorMode.PASSWORD) {
            val lengthLabel = L10n.generatorLength(resources, options.length.toString())
            BodyText(lengthLabel)
            Slider(
                value = options.length.toFloat(),
                onValueChange = { options = options.copy(length = it.roundToInt().toUInt()) },
                valueRange = 8f..64f,
                steps = 55,
                modifier = Modifier.semantics { contentDescription = lengthLabel },
            )
            CheckRow(options.lowercase, { options = options.copy(lowercase = it) }, L10n.generatorLowercase(resources))
            CheckRow(options.uppercase, { options = options.copy(uppercase = it) }, L10n.generatorUppercase(resources))
            CheckRow(options.digits, { options = options.copy(digits = it) }, L10n.generatorDigits(resources))
            CheckRow(options.symbols, { options = options.copy(symbols = it) }, L10n.generatorSymbols(resources))
            CheckRow(options.excludeAmbiguous, { options = options.copy(excludeAmbiguous = it) }, L10n.generatorAmbiguous(resources))
        } else {
            val wordsLabel = L10n.generatorWordCount(resources, options.words.toString())
            BodyText(wordsLabel)
            Slider(
                value = options.words.toFloat(),
                onValueChange = { options = options.copy(words = it.roundToInt().toUInt()) },
                valueRange = 3f..10f,
                steps = 6,
                modifier = Modifier.semantics { contentDescription = wordsLabel },
            )
            TextInput(
                value = options.separator,
                onValueChange = { options = options.copy(separator = it.take(1)) },
                label = L10n.generatorSeparator(resources),
            )
            CheckRow(options.capitalize, { options = options.copy(capitalize = it) }, L10n.generatorCapitalize(resources))
            CheckRow(options.appendDigit, { options = options.copy(appendDigit = it) }, L10n.generatorAppendDigit(resources))
        }
        SecondaryButton(
            L10n.generatorRun(resources),
            onClick = {
                scope.launch {
                    when (val result = app.vault.generate(options)) {
                        is Outcome.Ok -> {
                            holder.replace(result.value.value)
                            preview = SecretText.display(result.value.value)
                            bits = result.value.entropyBits.roundToInt()
                            errorCode = null
                        }
                        is Outcome.Failed -> errorCode = result.code
                        Outcome.Cancelled -> Unit
                    }
                }
            },
        )
        val failure = errorCode
        if (failure != null) ErrorText(errorText(resources, failure))
        val shown = preview
        if (shown != null) {
            Line(shown, mono = true)
            HintText(L10n.generatorStrength(resources, bits.toString()))
            PrimaryButton(
                L10n.generatorUse(resources),
                onClick = {
                    val secret = holder.secret
                    if (secret != null) {
                        val bytes = secret.exposeBytes()
                        try {
                            onUse(bytes)
                        } finally {
                            bytes.fill(0)
                        }
                        holder.replace(null)
                        preview = null
                    }
                },
            )
        }
    }
}
