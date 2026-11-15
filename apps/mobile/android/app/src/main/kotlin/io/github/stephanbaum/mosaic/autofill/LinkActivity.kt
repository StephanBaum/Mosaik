package io.github.stephanbaum.mosaic.autofill

import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.security.SecureActivity
import io.github.stephanbaum.mosaic.ui.AppRoot
import io.github.stephanbaum.mosaic.ui.MosaicTheme

/**
 * The "No login for this app" entry of the Autofill service opens this activity (M10-G2 M3). It is not exported, so
 * only the service's pending intent reaches it, and it carries a one-time token ([LinkTokens]) instead of a package
 * name: the exported main activity ignores the link extra completely. After the unlock (the normal one of
 * [AppRoot]) the user chooses the login to link to the app.
 */
class LinkActivity : SecureActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val app = application as MosaicApp
        val packageName = app.linkTokens.redeem(intent.getStringExtra(FillIntents.EXTRA_LINK_TOKEN))
        if (packageName == null) {
            finish()
            return
        }
        setContent {
            MosaicTheme {
                AppRoot(app, packageName)
            }
        }
    }
}
