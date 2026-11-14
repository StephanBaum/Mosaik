package io.github.stephanbaum.mosaic.security

import android.content.Context
import android.content.ContextWrapper
import android.graphics.drawable.GradientDrawable
import android.text.InputType
import android.util.TypedValue
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import java.nio.ByteBuffer
import java.nio.CharBuffer
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.runtime.Composable
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.LifecycleOwner
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.viewinterop.AndroidView
import io.github.stephanbaum.mosaic.core.SecretBytes
import io.github.stephanbaum.mosaic.ui.MosaicTheme
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicRadius
import io.github.stephanbaum.mosaic.ui.theme.MosaicSize
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import io.github.stephanbaum.mosaic.ui.theme.MosaicType

/**
 * The state of a [SecretField]: reads the secret out as bytes and wipes the field (docs/MOBILE.md, CRYPTO_SPEC
 * 6.9.5). A Compose `TextField` creates a `String` per keystroke; the native `EditText` behind this keeps the
 * text in an `Editable` whose characters can be overwritten.
 */
class SecretFieldState {
    internal var editText: EditText? = null
    private var pending: CharSequence? = null

    internal fun attach(view: EditText) {
        editText = view
        val waiting = pending
        if (waiting != null) {
            view.setText(waiting)
            pending = null
        }
    }

    /** Sets the text (an existing note to edit); applied when the view exists. */
    fun set(text: CharSequence) {
        val view = editText
        if (view != null) view.setText(text) else pending = text
    }

    /** Puts secret bytes (UTF-8) into the field without a `String`, then zeroes the temporary buffers. */
    fun put(secret: ByteArray) {
        val decoded = Charsets.UTF_8.decode(ByteBuffer.wrap(secret))
        val chars = CharArray(decoded.remaining())
        decoded.get(chars)
        if (decoded.hasArray()) decoded.array().fill('\u0000')
        val view = editText
        if (view != null) view.setText(CharBuffer.wrap(chars))
        chars.fill('\u0000')
    }

    /** The number of typed characters. */
    val length: Int get() = editText?.text?.length ?: 0

    /**
     * Reads the typed secret into bytes (UTF-8, no `String`) and wipes the field: `Editable.getChars` into a
     * `CharArray`, a UTF-8 encoder into a `ByteArray`, both arrays zeroed, then the editable overwritten and
     * cleared. Null when the field is empty.
     */
    fun take(): SecretBytes? {
        val editable = editText?.text ?: return null
        val count = editable.length
        if (count == 0) return null
        val chars = CharArray(count)
        editable.getChars(0, count, chars, 0)
        val secret = SecretBytes.ofUtf8(chars)
        wipeEditable()
        return secret
    }

    /** Overwrites and empties the field. */
    fun clear() {
        wipeEditable()
    }

    private fun wipeEditable() {
        val editable = editText?.text ?: return
        val count = editable.length
        if (count > 0) editable.replace(0, count, "\u0000".repeat(count))
        editable.clear()
    }
}

/**
 * A native password field (`TYPE_TEXT_VARIATION_PASSWORD`, no personalized IME learning, excluded from other
 * autofill services, never saved in instance state). A text field has a Show/Hide button; the text goes back to
 * hidden whenever the app leaves the foreground (M10-G4 item 10). A [numeric] field (the 2FA code) is a plain
 * number field, shown as typed: the code is short-lived and typing six digits blind helps nobody. The screens
 * stay `FLAG_SECURE` through [SecureActivity], whether the text is shown or not.
 */
@Composable
fun SecretField(
    state: SecretFieldState,
    label: String,
    modifier: Modifier = Modifier,
    numeric: Boolean = false,
    multiline: Boolean = false,
    enabled: Boolean = true,
    onSubmit: () -> Unit = {},
) {
    val colors = MosaicTheme.colors
    val density = LocalDensity.current
    val context = LocalContext.current
    val resources = context.resources
    val submit = rememberUpdatedState(onSubmit)
    val canReveal = !multiline && !numeric
    var shown by remember { mutableStateOf(false) }
    val appliedShown = remember { booleanArrayOf(false) }

    // Back to hidden when the app goes to the background.
    val owner = context.findLifecycleOwner()
    DisposableEffect(owner) {
        val observer =
            LifecycleEventObserver { _, event ->
                if (event == Lifecycle.Event.ON_STOP) shown = false
            }
        owner?.lifecycle?.addObserver(observer)
        onDispose { owner?.lifecycle?.removeObserver(observer) }
    }
    val strokePx = with(density) { MosaicSpace.s1.toPx().toInt().coerceAtLeast(1) }
    val radiusPx = with(density) { MosaicRadius.field.toPx() }
    val padPx = with(density) { MosaicSpace.s5.toPx().toInt() }

    DisposableEffect(state) {
        onDispose {
            state.clear()
            state.editText = null
        }
    }

    Row(
        modifier = modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s2),
    ) {
    AndroidView(
        modifier = Modifier.weight(1f).heightIn(min = MosaicSize.inputXl),
        factory = { context ->
            EditText(context).apply {
                inputType =
                    if (multiline) {
                        InputType.TYPE_CLASS_TEXT or
                            InputType.TYPE_TEXT_FLAG_MULTI_LINE or
                            InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
                    } else if (numeric) {
                        InputType.TYPE_CLASS_NUMBER
                    } else {
                        InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
                    }
                imeOptions =
                    if (multiline) {
                        EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
                    } else {
                        EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING or EditorInfo.IME_ACTION_DONE
                    }
                importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
                isSaveEnabled = false
                if (multiline) {
                    setSingleLine(false)
                    minLines = 4
                    gravity = android.view.Gravity.TOP or android.view.Gravity.START
                } else {
                    setSingleLine(true)
                }
                setTextSize(TypedValue.COMPLEX_UNIT_SP, MosaicType.md.size.value)
                setPadding(padPx, padPx, padPx, padPx)
                setOnEditorActionListener { _, actionId, _ ->
                    if (actionId == EditorInfo.IME_ACTION_DONE) {
                        submit.value()
                        true
                    } else {
                        false
                    }
                }
                ScreenSecurity.markSensitive(this)
                state.attach(this)
            }
        },
        update = { view ->
            view.hint = label
            view.contentDescription = label
            view.isEnabled = enabled
            if (canReveal && appliedShown[0] != shown) {
                appliedShown[0] = shown
                val caret = view.selectionEnd
                view.inputType =
                    InputType.TYPE_CLASS_TEXT or
                        InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS or
                        (if (shown) InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD else InputType.TYPE_TEXT_VARIATION_PASSWORD)
                view.setSingleLine(true)
                if (caret >= 0) view.setSelection(caret.coerceAtMost(view.length()))
            }
            view.setTextColor(colors.text.toArgb())
            view.setHintTextColor(colors.textMuted.toArgb())
            view.background =
                GradientDrawable().apply {
                    setColor(colors.surface.toArgb())
                    setStroke(strokePx, colors.borderStrong.toArgb())
                    cornerRadius = radiusPx
                }
        },
    )
    if (canReveal) {
        val action = if (shown) L10n.commonHideField(resources, label) else L10n.commonShowField(resources, label)
        TextButton(
            onClick = { shown = !shown },
            enabled = enabled,
            modifier = Modifier.heightIn(min = MosaicSize.inputXl).semantics { contentDescription = action },
        ) {
            Text(if (shown) L10n.commonHide(resources) else L10n.commonShow(resources))
        }
    }
    }
}

private fun Context.findLifecycleOwner(): LifecycleOwner? {
    var current: Context? = this
    while (current is ContextWrapper) {
        if (current is LifecycleOwner) return current
        current = current.baseContext
    }
    return null
}
