package io.github.stephanbaum.mosaic.autofill

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.view.autofill.AutofillId
import androidx.core.content.IntentCompat
import io.github.stephanbaum.mosaic.ffi.ItemKey
import java.security.SecureRandom

/** The autofill fields a request can fill (the IDs the platform gave), chosen by the core. */
class FillFields(val username: AutofillId?, val password: AutofillId?, val otp: AutofillId?) {
    /** True when a user name or password field can take a login. */
    val hasLogin: Boolean get() = username != null || password != null

    val all: List<AutofillId> get() = listOfNotNull(username, password, otp)
}

/**
 * What the datasets of one request need to find the way back: the caller, as the structure described it, and the
 * fields. The extras never carry a value, and never an item: the activity looks the matches up again in the core.
 * [FillAuthActivity] is not exported, so only the platform's pending intents reach it.
 */
class FillRequestInfo(
    val callerPackage: String,
    val webDomain: String?,
    val webScheme: String?,
    val fields: FillFields,
) {
    fun toIntent(context: Context, mode: String): Intent =
        Intent(context, FillAuthActivity::class.java)
            .putExtra(FillIntents.EXTRA_MODE, mode)
            .putExtra(FillIntents.EXTRA_PACKAGE, callerPackage)
            .putExtra(FillIntents.EXTRA_WEB_DOMAIN, webDomain)
            .putExtra(FillIntents.EXTRA_WEB_SCHEME, webScheme)
            .putExtra(FillIntents.EXTRA_USERNAME_ID, fields.username)
            .putExtra(FillIntents.EXTRA_PASSWORD_ID, fields.password)
            .putExtra(FillIntents.EXTRA_OTP_ID, fields.otp)

    companion object {
        fun fromIntent(intent: Intent): FillRequestInfo? {
            val pkg = intent.getStringExtra(FillIntents.EXTRA_PACKAGE)
            if (pkg.isNullOrEmpty()) return null
            return FillRequestInfo(
                callerPackage = pkg,
                webDomain = intent.getStringExtra(FillIntents.EXTRA_WEB_DOMAIN),
                webScheme = intent.getStringExtra(FillIntents.EXTRA_WEB_SCHEME),
                fields =
                    FillFields(
                        username = IntentCompat.getParcelableExtra(intent, FillIntents.EXTRA_USERNAME_ID, AutofillId::class.java),
                        password = IntentCompat.getParcelableExtra(intent, FillIntents.EXTRA_PASSWORD_ID, AutofillId::class.java),
                        otp = IntentCompat.getParcelableExtra(intent, FillIntents.EXTRA_OTP_ID, AutofillId::class.java),
                    ),
            )
        }
    }
}

/** The pending intents of datasets and the keys of their extras. */
object FillIntents {
    /** The unlock entry: after the unlock, answer with the datasets of the matching logins. */
    const val MODE_LIST = "list"

    /** A dataset tap: answer with the values of one login. */
    const val MODE_FILL = "fill"

    /** An entry with nothing to do (the app could not be checked): close at once. */
    const val MODE_DISMISS = "dismiss"

    /** A dataset of a login (user name and password). */
    const val KIND_LOGIN = "login"

    /** A dataset of a one-time code. */
    const val KIND_OTP = "otp"

    const val EXTRA_MODE = "io.github.stephanbaum.mosaic.autofill.MODE"
    const val EXTRA_KIND = "io.github.stephanbaum.mosaic.autofill.KIND"
    const val EXTRA_ITEM_VAULT = "io.github.stephanbaum.mosaic.autofill.ITEM_VAULT"
    const val EXTRA_ITEM_ID = "io.github.stephanbaum.mosaic.autofill.ITEM_ID"
    const val EXTRA_PACKAGE = "io.github.stephanbaum.mosaic.autofill.PACKAGE"
    const val EXTRA_WEB_DOMAIN = "io.github.stephanbaum.mosaic.autofill.WEB_DOMAIN"
    const val EXTRA_WEB_SCHEME = "io.github.stephanbaum.mosaic.autofill.WEB_SCHEME"
    const val EXTRA_USERNAME_ID = "io.github.stephanbaum.mosaic.autofill.USERNAME_ID"
    const val EXTRA_PASSWORD_ID = "io.github.stephanbaum.mosaic.autofill.PASSWORD_ID"
    const val EXTRA_OTP_ID = "io.github.stephanbaum.mosaic.autofill.OTP_ID"

    /** [LinkActivity] redeems this one-time token (see [LinkTokens]) for the package of the "choose a login" screen. */
    const val EXTRA_LINK_TOKEN = "io.github.stephanbaum.mosaic.autofill.LINK_TOKEN"

    private val random = SecureRandom()

    /**
     * A mutable pending intent (the platform adds the authentication extras) for an explicit component. A random
     * request code per entry keeps the platform from merging the intents of two datasets, and an app cannot guess it
     * to create a pending intent first; `FLAG_CANCEL_CURRENT` voids an earlier one with the same intent
     * (M10-G2 L5).
     */
    fun pending(context: Context, intent: Intent): PendingIntent =
        PendingIntent.getActivity(
            context,
            random.nextInt(),
            intent,
            PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_CANCEL_CURRENT,
        )

    fun withItem(intent: Intent, key: ItemKey, kind: String): Intent =
        intent
            .putExtra(EXTRA_ITEM_VAULT, key.vaultId)
            .putExtra(EXTRA_ITEM_ID, key.itemId)
            .putExtra(EXTRA_KIND, kind)
}
