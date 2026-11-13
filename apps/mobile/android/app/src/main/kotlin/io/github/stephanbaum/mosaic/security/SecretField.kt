package io.github.stephanbaum.mosaic.security

import android.graphics.drawable.GradientDrawable
import android.text.InputType
import android.util.TypedValue
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import java.nio.ByteBuffer
import java.nio.CharBuffer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.viewinterop.AndroidView
import io.github.stephanbaum.mosaic.core.SecretBytes
import io.github.stephanbaum.mosaic.ui.MosaicTheme
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
 * autofill services, never saved in instance state).
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
    val submit = rememberUpdatedState(onSubmit)
    val strokePx = with(density) { MosaicSpace.s1.toPx().toInt().coerceAtLeast(1) }
    val radiusPx = with(density) { MosaicRadius.field.toPx() }
    val padPx = with(density) { MosaicSpace.s5.toPx().toInt() }

    DisposableEffect(state) {
        onDispose {
            state.clear()
            state.editText = null
        }
    }

    AndroidView(
        modifier = modifier.fillMaxWidth().heightIn(min = MosaicSize.inputXl),
        factory = { context ->
            EditText(context).apply {
                inputType =
                    if (multiline) {
                        InputType.TYPE_CLASS_TEXT or
                            InputType.TYPE_TEXT_FLAG_MULTI_LINE or
                            InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
                    } else if (numeric) {
                        InputType.TYPE_CLASS_NUMBER or InputType.TYPE_NUMBER_VARIATION_PASSWORD
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
}
