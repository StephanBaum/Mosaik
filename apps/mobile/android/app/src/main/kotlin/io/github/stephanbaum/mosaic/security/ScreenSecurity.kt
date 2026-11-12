package io.github.stephanbaum.mosaic.security

import android.app.Activity
import android.os.Build
import android.view.View
import android.view.WindowManager

/** Screen protection of docs/MOBILE.md (ADR-0094): no screenshots, no recents thumbnail, sensitive views. */
object ScreenSecurity {
    /** `FLAG_SECURE` on the window, and no recents screenshot on API 33+. Called by [SecureActivity]. */
    fun protect(activity: Activity) {
        activity.window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            activity.setRecentsScreenshotEnabled(false)
        }
    }

    /** Marks a view that shows or takes a secret (API 35+: content sensitivity). */
    fun markSensitive(view: View) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.VANILLA_ICE_CREAM) {
            view.setContentSensitivity(View.CONTENT_SENSITIVITY_SENSITIVE)
        }
    }
}
