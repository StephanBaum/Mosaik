package io.github.stephanbaum.mosaic.security

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
    }

    override fun dispatchTouchEvent(ev: MotionEvent): Boolean {
        (application as? MosaicApp)?.lockPolicy?.onUserInteraction()
        return super.dispatchTouchEvent(ev)
    }
}
