package io.github.stephanbaum.mosaic.ui

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.session.OAuthCallback

/**
 * Catches the redirect of a cloud-folder sign-in and hands it to the core, then brings the app back. It draws
 * nothing (translucent theme), is not kept in the history, never logs, and never puts the address into saved
 * state: the address holds a one-time authorization code, valid only together with the verifier the core keeps.
 */
class OAuthRedirectActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        // No saved state in, none out.
        super.onCreate(null)
        val text = intent?.dataString
        // The intent is dropped from this activity as soon as it is read.
        intent = Intent()
        if (text != null && OAuthCallback.accepts(text)) {
            (application as MosaicApp).session.deliverOAuthCallback(text)
        }
        startActivity(
            Intent(this, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP),
        )
        finish()
    }
}
