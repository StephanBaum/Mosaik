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
        // This activity is exported: it takes nothing from its intent. The link screen of the "No login for this
        // app" entry is LinkActivity, which is not exported (M10-G2 M3).
        setContent {
            MosaicTheme {
                AppRoot(app)
            }
        }
    }
}
