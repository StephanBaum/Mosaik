package io.github.stephanbaum.mosaic.security

import android.os.Build
import android.os.Bundle
import android.view.MotionEvent
import androidx.activity.ComponentActivity
import io.github.stephanbaum.mosaic.MosaicApp

/**
 * The base of every activity of the app, the provider activities included: `FLAG_SECURE` and the recents
 * setting are applied before any content exists, and every touch resets the foreground idle timer of
 * [LockPolicy] (ADR-0094). ScreenSecurityTest checks that each declared activity extends this class.
 */
@Suppress("NULLABILITY_MISMATCH_BASED_ON_JAVA_ANNOTATIONS")
abstract class SecureActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        ScreenSecurity.protect(this)
        super.onCreate(savedInstanceState)
        // API 31+: other apps' overlay windows are hidden while this window is in front (needs the
        // HIDE_OVERLAY_WINDOWS permission of the manifest), so nothing can draw over a secret.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) window.setHideOverlayWindows(true)
        // Touches that reach this window while another app's window covers it are dropped. This is the defense on
        // API 28-30, which have no setHideOverlayWindows, and it holds on newer versions too (RT-android-4). It
        // covers this activity's window only: a Compose dialog or menu is a window of its own, so every one of them
        // uses SecureAlertDialog or SecureDropdownMenu (ui/components/SecureWindows.kt), which set it there too.
        window.decorView.filterTouchesWhenObscured = true
    }

    override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
        (application as? MosaicApp)?.lockPolicy?.onUserInteraction()
        return super.dispatchTouchEvent(ev)
    }
}
