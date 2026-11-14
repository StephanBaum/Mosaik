package io.github.stephanbaum.mosaic.ui.vault

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.FieldView
import io.github.stephanbaum.mosaic.ffi.ItemDetail
import io.github.stephanbaum.mosaic.ffi.ItemKind
import io.github.stephanbaum.mosaic.ffi.OtpScheme
import io.github.stephanbaum.mosaic.ffi.SecretValue
import io.github.stephanbaum.mosaic.security.SecretText
import io.github.stephanbaum.mosaic.security.SensitiveWhile
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.MosaicTheme
import io.github.stephanbaum.mosaic.ui.components.ActionButton
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.Line
import io.github.stephanbaum.mosaic.ui.components.Notice
import io.github.stephanbaum.mosaic.ui.components.PageColumn
import io.github.stephanbaum.mosaic.ui.components.QuietButton
import io.github.stephanbaum.mosaic.ui.components.SecondaryButton
import io.github.stephanbaum.mosaic.ui.components.SectionTitle
import io.github.stephanbaum.mosaic.ui.components.SecureAlertDialog
import io.github.stephanbaum.mosaic.ui.components.TopBar
import io.github.stephanbaum.mosaic.ui.components.Working
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import io.github.stephanbaum.mosaic.ui.theme.MosaicType
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

/** Revealed values hide again after this long (UX_GUIDELINES section 3). */
private const val REVEAL_MS = 60_000L
private const val NOTICE_MS = 4_000L
private const val DOTS = "••••••••"

/** The revealed secrets of one detail screen: the core's values (wiped on hide) and the text drawn from them. */
internal class RevealState {
    val shown = mutableStateMapOf<String, String>()
    private val secrets = HashMap<String, SecretValue>()

    fun show(key: String, secret: SecretValue) {
        secrets.remove(key)?.wipe()
        secrets[key] = secret
        shown[key] = SecretText.display(secret)
    }

    fun hide(key: String) {
        secrets.remove(key)?.wipe()
        shown.remove(key)
    }

    fun hideAll() {
        for (secret in secrets.values) secret.wipe()
        secrets.clear()
        shown.clear()
    }
}

/** A one-time code on screen. */
class OtpDisplay(val code: String, val remaining: Int, val period: Int)

/** The core's code as it is shown: 6 digits as 3 and 3, 8 as 4 and 4. */
fun groupCode(code: String): String =
    when (code.length) {
        6 -> code.substring(0, 3) + " " + code.substring(3)
        8 -> code.substring(0, 4) + " " + code.substring(4)
        else -> code
    }

private class OtpHolder {
    var secret: SecretValue? = null

    fun replace(next: SecretValue?) {
        secret?.wipe()
        secret = next
    }
}

/** The item detail: loads the item, reveals and copies fields, runs the one-time code, deletes. */
@Composable
fun ItemDetailScreen(
    app: MosaicApp,
    vaultId: String,
    itemId: String,
    version: Int,
    onBack: () -> Unit,
    onEdit: (ItemKind) -> Unit,
    onLink: () -> Unit,
    onChanged: () -> Unit,
) {
    val resources = LocalContext.current.resources
    val scope = rememberCoroutineScope()
    val foreground by app.foreground.collectAsState()
    var detail by remember { mutableStateOf<ItemDetail?>(null) }
    var failure by remember { mutableStateOf<String?>(null) }
    var notice by remember { mutableStateOf<String?>(null) }
    var otp by remember { mutableStateOf<OtpDisplay?>(null) }
    val reveal = remember { RevealState() }
    val timers = remember { HashMap<String, Job>() }
    val otpHolder = remember { OtpHolder() }

    DisposableEffect(Unit) {
        onDispose {
            reveal.hideAll()
            otpHolder.replace(null)
        }
    }
    LaunchedEffect(foreground) {
        if (!foreground) {
            reveal.hideAll()
            otp = null
            otpHolder.replace(null)
        }
    }
    LaunchedEffect(notice) {
        if (notice != null) {
            delay(NOTICE_MS)
            notice = null
        }
    }
    LaunchedEffect(vaultId, itemId, version) {
        when (val result = app.vault.item(vaultId, itemId)) {
            is Outcome.Ok -> {
                detail = result.value
                failure = null
            }
            is Outcome.Failed -> failure = result.code
            Outcome.Cancelled -> Unit
        }
    }

    val hasOtp = detail?.otp != null
    val isHotp = detail?.otp?.scheme == OtpScheme.HOTP
    LaunchedEffect(hasOtp, foreground) {
        if (!hasOtp || !foreground) return@LaunchedEffect
        while (isActive) {
            val result = app.vault.otpCode(vaultId, itemId)
            if (result !is Outcome.Ok) {
                if (result is Outcome.Failed) failure = result.code
                return@LaunchedEffect
            }
            val fetched = result.value
            val text = SecretText.display(fetched.code)
            otpHolder.replace(fetched.code)
            val period = fetched.periodS.toInt()
            var left = maxOf(fetched.remainingS.toInt(), 1)
            otp = OtpDisplay(text, left, period)
            if (isHotp) return@LaunchedEffect
            while (left > 0 && isActive) {
                delay(1000)
                left -= 1
                otp = OtpDisplay(text, left, period)
            }
        }
    }

    val seconds = app.lockSettings.clipboardSeconds
    fun copyDone() {
        notice = L10n.copyDone(resources, seconds)
    }

    ItemDetailContent(
        detail = detail,
        failure = failure?.let { errorText(resources, it) },
        revealed = reveal.shown,
        otp = otp,
        notice = notice,
        onBack = onBack,
        onEdit = onEdit,
        onLink = onLink,
        onToggleFavorite = {
            val current = detail
            if (current != null) {
                scope.launch {
                    val result = app.vault.setFavorite(vaultId, itemId, !current.summary.favorite)
                    if (result is Outcome.Failed) failure = result.code else onChanged()
                }
            }
        },
        onDelete = {
            scope.launch {
                val result = app.vault.trash(vaultId, itemId)
                if (result is Outcome.Failed) {
                    failure = result.code
                } else {
                    onChanged()
                    onBack()
                }
            }
        },
        onReveal = { field ->
            scope.launch {
                when (val result = app.vault.reveal(vaultId, itemId, field.key)) {
                    is Outcome.Ok -> {
                        reveal.show(field.key, result.value)
                        timers.remove(field.key)?.cancel()
                        timers[field.key] =
                            scope.launch {
                                delay(REVEAL_MS)
                                reveal.hide(field.key)
                            }
                        notice = L10n.itemSecretRevealed(resources, fieldLabel(resources, field), (REVEAL_MS / 1000).toInt())
                    }
                    is Outcome.Failed -> failure = result.code
                    Outcome.Cancelled -> Unit
                }
            }
        },
        onHide = { field ->
            timers.remove(field.key)?.cancel()
            reveal.hide(field.key)
        },
        onCopy = { field ->
            val value = field.value
            if (!field.concealed && value != null) {
                app.clipboard.copy(value, seconds)
                copyDone()
            } else {
                scope.launch {
                    when (val result = app.vault.reveal(vaultId, itemId, field.key)) {
                        is Outcome.Ok -> {
                            try {
                                SecretText.copy(app.clipboard, result.value, seconds)
                            } finally {
                                result.value.wipe()
                            }
                            copyDone()
                        }
                        is Outcome.Failed -> failure = result.code
                        Outcome.Cancelled -> Unit
                    }
                }
            }
        },
        onCopyOtp = {
            val secret = otpHolder.secret
            if (secret != null) {
                SecretText.copy(app.clipboard, secret, seconds)
                copyDone()
            }
        },
    )
}

/** The stateless detail (also drawn by the screenshot test). */
@Composable
fun ItemDetailContent(
    detail: ItemDetail?,
    failure: String?,
    revealed: Map<String, String>,
    otp: OtpDisplay?,
    notice: String?,
    onBack: () -> Unit,
    onEdit: (ItemKind) -> Unit,
    onLink: () -> Unit,
    onToggleFavorite: () -> Unit,
    onDelete: () -> Unit,
    onReveal: (FieldView) -> Unit,
    onHide: (FieldView) -> Unit,
    onCopy: (FieldView) -> Unit,
    onCopyOtp: () -> Unit,
) {
    val resources = LocalContext.current.resources
    var confirmDelete by remember { mutableStateOf(false) }
    // A revealed value or a one-time code is on screen: the window is sensitive for that time (M10-G2 L7).
    SensitiveWhile(revealed.isNotEmpty() || otp != null)
    Box(modifier = Modifier.fillMaxSize()) {
        PageColumn {
            val summary = detail?.summary
            val title = summary?.title?.ifEmpty { null } ?: L10n.vaultUntitled(resources)
            TopBar(title = title, onBack = onBack) {
                if (summary != null) {
                    ActionButton(
                        text = if (summary.favorite) "★" else "☆",
                        description = if (summary.favorite) L10n.detailFavoriteRemove(resources) else L10n.detailFavoriteAdd(resources),
                        onClick = onToggleFavorite,
                    )
                    if (summary.kind == ItemKind.LOGIN || summary.kind == ItemKind.SECURE_NOTE) {
                        ActionButton(
                            text = L10n.commonEdit(resources),
                            description = L10n.commonEdit(resources),
                            onClick = { onEdit(summary.kind) },
                        )
                    }
                }
            }
            if (failure != null) ErrorText(failure)
            if (detail == null) {
                if (failure == null) Working(L10n.commonWorking(resources))
            } else {
                HintText(kindLabel(resources, detail.summary.kind))
                if (detail.otp != null) OtpBlock(otp = otp, onCopy = onCopyOtp)
                for (field in detail.fields) {
                    FieldRow(field = field, revealedValue = revealed[field.key], onReveal = onReveal, onHide = onHide, onCopy = onCopy)
                }
                if (detail.urls.isNotEmpty()) {
                    SectionTitle(L10n.itemUrls(resources, detail.urls.size))
                    for (url in detail.urls) {
                        if (url.isApp) {
                            Line(L10n.detailLinkedApp(resources, appUrlName(url.url)))
                            if (url.needsLinking) HintText(L10n.detailNeedsLinking(resources))
                        } else {
                            Line(url.url)
                        }
                    }
                }
                if (detail.tags.isNotEmpty()) {
                    SectionTitle(L10n.itemTags(resources))
                    Line(detail.tags.joinToString(", "))
                }
                for (passkey in detail.passkeys) {
                    Line(L10n.detailPasskey(resources, passkey.rpId))
                    HintText(passkey.userName)
                }
                if (detail.attachments.isNotEmpty()) HintText(L10n.detailAttachments(resources, detail.attachments.size))
                if (detail.summary.kind == ItemKind.LOGIN) SecondaryButton(L10n.detailLinkApp(resources), onClick = onLink)
                QuietButton(L10n.detailDelete(resources), onClick = { confirmDelete = true })
            }
        }
        if (notice != null) {
            Box(modifier = Modifier.align(Alignment.BottomCenter).padding(MosaicSpace.s5)) { Notice(notice) }
        }
    }
    if (confirmDelete && detail != null) {
        SecureAlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = {
                Text(L10n.detailDeleteTitle(resources, detail.summary.title.ifBlank { L10n.vaultUntitled(resources) }))
            },
            text = { Text(L10n.detailDeleteBody(resources)) },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmDelete = false
                        onDelete()
                    },
                ) {
                    Text(L10n.detailDelete(resources))
                }
            },
            dismissButton = { TextButton(onClick = { confirmDelete = false }) { Text(L10n.commonCancel(resources)) } },
        )
    }
}

@Composable
private fun FieldRow(
    field: FieldView,
    revealedValue: String?,
    onReveal: (FieldView) -> Unit,
    onHide: (FieldView) -> Unit,
    onCopy: (FieldView) -> Unit,
) {
    val resources = LocalContext.current.resources
    val label = fieldLabel(resources, field)
    val hiddenLabel = L10n.itemSecretHiddenLabel(resources)
    Column(verticalArrangement = Arrangement.spacedBy(MosaicSpace.s1)) {
        HintText(label)
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s2)) {
            if (field.concealed) {
                if (revealedValue != null) {
                    Line(revealedValue, modifier = Modifier.weight(1f), mono = true)
                    ActionButton(L10n.commonHide(resources), L10n.itemSecretHideAction(resources, label), { onHide(field) })
                } else {
                    Line(DOTS, modifier = Modifier.weight(1f).semantics { contentDescription = hiddenLabel }, mono = true)
                    ActionButton(L10n.commonShow(resources), L10n.itemSecretRevealAction(resources, label), { onReveal(field) })
                }
            } else {
                Line(field.value.orEmpty(), modifier = Modifier.weight(1f))
            }
            if (field.copyable) {
                ActionButton(L10n.commonCopy(resources), L10n.itemCopy(resources, label), { onCopy(field) })
            }
        }
    }
}

/** The one-time code with its countdown ring. Screen readers read neither the code nor the time until it is copied. */
@Composable
private fun OtpBlock(otp: OtpDisplay?, onCopy: () -> Unit) {
    val resources = LocalContext.current.resources
    val label = L10n.itemKindTotp(resources)
    val view = LocalView.current
    val readCode = L10n.detailReadCode(resources)
    Column(verticalArrangement = Arrangement.spacedBy(MosaicSpace.s1)) {
        HintText(label)
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s4)) {
            Row(
                modifier =
                    Modifier.weight(1f).clearAndSetSemantics {
                        contentDescription = label
                        // Only on the user's explicit request does the code reach the screen reader, digit by digit.
                        if (otp != null) {
                            customActions =
                                listOf(
                                    CustomAccessibilityAction(readCode) {
                                        // A one-off announcement on request. The suggested replacement (a live
                                        // region) would keep the code in the semantics tree, which is worse for a
                                        // secret, so the deprecated call stays, suppressed at this one line.
                                        @Suppress("DEPRECATION")
                                        view.announceForAccessibility(otp.code.toList().joinToString(" "))
                                        true
                                    },
                                )
                        }
                    },
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s4),
            ) {
                if (otp != null) {
                    CountdownRing(fraction = otp.remaining.toFloat() / maxOf(otp.period, 1).toFloat())
                    Text(
                        text = groupCode(otp.code),
                        color = MosaicTheme.colors.text,
                        fontFamily = MosaicTheme.fonts.mono,
                        fontWeight = FontWeight.Medium,
                        fontSize = MosaicType.xl.size,
                        lineHeight = MosaicType.xl.lineHeight,
                    )
                } else {
                    Line(DOTS, mono = true)
                }
            }
            ActionButton(L10n.commonCopy(resources), L10n.itemCopy(resources, label), onCopy, enabled = otp != null)
        }
        if (otp != null) HintText(L10n.detailOtpLeft(resources, otp.remaining))
    }
}

@Composable
private fun CountdownRing(fraction: Float) {
    val track = MosaicTheme.colors.border
    val arc = MosaicTheme.colors.accent
    Canvas(modifier = Modifier.size(MosaicSpace.s7)) {
        val strokeWidth = MosaicSpace.s2.toPx()
        val stroke = Stroke(width = strokeWidth, cap = StrokeCap.Round)
        val inset = strokeWidth / 2f
        val topLeft = Offset(inset, inset)
        val box = Size(size.width - strokeWidth, size.height - strokeWidth)
        drawArc(color = track, startAngle = 0f, sweepAngle = 360f, useCenter = false, topLeft = topLeft, size = box, style = stroke)
        drawArc(color = arc, startAngle = -90f, sweepAngle = 360f * fraction.coerceIn(0f, 1f), useCenter = false, topLeft = topLeft, size = box, style = stroke)
    }
}
