package io.github.stephanbaum.mosaic.ui

import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.security.SecureActivity

/** The one activity of the app: onboarding, unlock and (from T8) the vault, all inside the secure window. */
class MainActivity : SecureActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val app = application as MosaicApp
        setContent {
            MosaicTheme {
                AppRoot(app)
            }
        }
    }
}
