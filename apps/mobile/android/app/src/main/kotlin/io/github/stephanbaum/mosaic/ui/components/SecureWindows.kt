package io.github.stephanbaum.mosaic.ui.components

import android.view.View
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.platform.LocalView

/**
 * Every dialog and menu of the app goes through this file (scripts/mobile/check-secure-windows.sh fails the build
 * on a bare `AlertDialog`, `DropdownMenu`, `Dialog`, `Popup` or `ModalBottomSheet` anywhere else).
 *
 * Why: `SecureActivity` drops touches that reach the activity window while another app's window covers it
 * (tapjacking, RT-android-4), but a Compose dialog or menu is a window of its own and does not inherit that
 * setting. [ObscuredTouchFilter] sets it on the window's own view tree.
 */
@Composable
fun SecureAlertDialog(
    onDismissRequest: () -> Unit,
    confirmButton: @Composable () -> Unit,
    dismissButton: @Composable (() -> Unit)? = null,
    title: @Composable (() -> Unit)? = null,
    text: @Composable (() -> Unit)? = null,
) {
    AlertDialog(
        onDismissRequest = onDismissRequest,
        confirmButton = {
            ObscuredTouchFilter()
            confirmButton()
        },
        dismissButton = dismissButton,
        title = title,
        text = text,
    )
}

/** A `DropdownMenu` whose popup window drops touches while another app's window covers it. */
@Composable
fun SecureDropdownMenu(
    expanded: Boolean,
    onDismissRequest: () -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    DropdownMenu(expanded = expanded, onDismissRequest = onDismissRequest) {
        ObscuredTouchFilter()
        content()
    }
}

/**
 * Sets `filterTouchesWhenObscured` on the view of the window this composable is in: inside a dialog or a popup,
 * `LocalView` is the compose view of that window, and its root view is the dialog's decor view or the popup's
 * own layout. Both are set, now and again when the view is attached (a popup's view is attached after its
 * content is composed).
 */
@Composable
private fun ObscuredTouchFilter() {
    val view = LocalView.current
    DisposableEffect(view) {
        fun apply() {
            view.filterTouchesWhenObscured = true
            view.rootView.filterTouchesWhenObscured = true
        }
        val listener =
            object : View.OnAttachStateChangeListener {
                override fun onViewAttachedToWindow(v: View) = apply()

                override fun onViewDetachedFromWindow(v: View) = Unit
            }
        apply()
        view.addOnAttachStateChangeListener(listener)
        onDispose { view.removeOnAttachStateChangeListener(listener) }
    }
}
