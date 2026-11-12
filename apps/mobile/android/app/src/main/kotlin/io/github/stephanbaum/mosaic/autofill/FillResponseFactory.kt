package io.github.stephanbaum.mosaic.autofill

import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.service.autofill.Dataset
import android.service.autofill.FillResponse
import android.view.View
import android.view.autofill.AutofillId
import android.view.autofill.AutofillValue
import android.widget.RemoteViews
import io.github.stephanbaum.mosaic.R
import io.github.stephanbaum.mosaic.ffi.FillCandidate
import io.github.stephanbaum.mosaic.ui.MainActivity
import io.github.stephanbaum.mosaic.ui.l10n.L10n

/** What one dataset of the last response held, kept in debug builds only: counts, never a value. */
class DatasetAudit(val kind: String, val authenticated: Boolean, val nonNullValues: Int)

/**
 * The test seam of docs/MOBILE.md section 6.3: in a debuggable build the factory remembers what the datasets of
 * its last response hold, so a test can check that no dataset carries a value before the user taps it. The
 * platform's `Dataset` cannot be read back by an app. Release builds record nothing.
 */
object FillAudit {
    @Volatile
    var last: List<DatasetAudit> = emptyList()

    fun record(context: Context, datasets: List<DatasetAudit>) {
        if ((context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE) != 0) last = datasets
    }
}

/**
 * Builds the responses and datasets of the Autofill service. Per-dataset authentication throughout: a dataset in
 * a response has `null` values and an authentication intent, and a value exists only in the dataset
 * [FillAuthActivity] returns after the user tapped one (ADR-0092). Never compatibility mode, inline suggestions
 * or a save request.
 */
@Suppress("DEPRECATION")
class FillResponseFactory(private val context: Context) {
    private val resources = context.resources

    /** The one "Unlock Mosaic" entry; its authentication opens [FillAuthActivity] in list mode. */
    fun unlockResponse(info: FillRequestInfo): FillResponse {
        val sender = FillIntents.pending(context, info.toIntent(context, FillIntents.MODE_LIST)).intentSender
        FillAudit.record(context, emptyList())
        return FillResponse.Builder()
            .setAuthentication(
                info.fields.all.toTypedArray(),
                sender,
                presentation(L10n.autofillUnlockEntry(resources), null),
            )
            .build()
    }

    /**
     * One authenticated dataset per matching login (and per one-time code for a field that takes one). With no match
     * in an app: the "No login for ..." entry. With no match in a browser page ([noLoginLabel] null): no response,
     * so the browser keeps its own suggestions. [warning] adds the address warning to every line.
     */
    fun candidatesResponse(
        info: FillRequestInfo,
        candidates: List<FillCandidate>,
        warning: Boolean,
        noLoginLabel: String?,
    ): FillResponse? {
        val builder = FillResponse.Builder()
        val audits = ArrayList<DatasetAudit>()
        for (candidate in candidates) {
            if (info.fields.hasLogin) {
                val ids = listOfNotNull(info.fields.username, info.fields.password)
                val intent = FillIntents.withItem(info.toIntent(context, FillIntents.MODE_FILL), candidate.key, FillIntents.KIND_LOGIN)
                builder.addDataset(authenticated(ids, candidate.title, subtitle(candidate.username, warning), intent))
                audits.add(DatasetAudit(FillIntents.KIND_LOGIN, true, 0))
            }
            val otpId = info.fields.otp
            if (otpId != null && candidate.hasOtp) {
                val intent = FillIntents.withItem(info.toIntent(context, FillIntents.MODE_FILL), candidate.key, FillIntents.KIND_OTP)
                builder.addDataset(authenticated(listOf(otpId), candidate.title, L10n.autofillOtpEntry(resources), intent))
                audits.add(DatasetAudit(FillIntents.KIND_OTP, true, 0))
            }
        }
        if (audits.isEmpty()) {
            if (noLoginLabel == null) return null
            FillAudit.record(context, listOf(DatasetAudit("noLogin", true, 0)))
            return FillResponse.Builder().addDataset(noLoginDataset(info, noLoginLabel)).build()
        }
        FillAudit.record(context, audits)
        return builder.build()
    }

    /** One entry that only says the app could not be checked; a tap closes it. */
    fun cannotCheckResponse(info: FillRequestInfo): FillResponse {
        FillAudit.record(context, listOf(DatasetAudit("cannotCheck", true, 0)))
        val intent = info.toIntent(context, FillIntents.MODE_DISMISS)
        return FillResponse.Builder()
            .addDataset(authenticated(info.fields.all, L10n.autofillCannotCheck(resources), null, intent))
            .build()
    }

    /** The dataset [FillAuthActivity] returns after a tap: the real values, for the fields the request found. */
    fun valuesDataset(info: FillRequestInfo, kind: String, username: String?, secret: String?): Dataset {
        val builder = Dataset.Builder(presentation(L10n.appName(resources), null))
        if (kind == FillIntents.KIND_OTP) {
            val id = info.fields.otp
            if (id != null && secret != null) builder.setValue(id, AutofillValue.forText(secret))
        } else {
            val userId = info.fields.username
            val passwordId = info.fields.password
            if (userId != null && username != null) builder.setValue(userId, AutofillValue.forText(username))
            if (passwordId != null && secret != null) builder.setValue(passwordId, AutofillValue.forText(secret))
        }
        return builder.build()
    }

    private fun noLoginDataset(info: FillRequestInfo, label: String): Dataset {
        val link =
            Intent(context, MainActivity::class.java)
                .putExtra(FillIntents.EXTRA_LINK_PACKAGE, info.callerPackage)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
        val builder =
            Dataset.Builder(presentation(L10n.autofillNoLogin(resources, label), L10n.autofillNoLoginHint(resources)))
        for (id in info.fields.all) builder.setValue(id, null)
        builder.setAuthentication(FillIntents.pending(context, link).intentSender)
        return builder.build()
    }

    private fun authenticated(ids: List<AutofillId>, title: String, subtitle: String?, intent: Intent): Dataset {
        val builder = Dataset.Builder(presentation(title, subtitle))
        // The IDs only say which fields the dataset belongs to; null is the value of a dataset that needs
        // authentication, so nothing is readable before the tap.
        for (id in ids) builder.setValue(id, null)
        builder.setAuthentication(FillIntents.pending(context, intent).intentSender)
        return builder.build()
    }

    private fun subtitle(username: String?, warning: Boolean): String {
        val name = username ?: L10n.autofillNoUsername(resources)
        return if (warning) L10n.autofillWarning(resources) + " | " + name else name
    }

    private fun presentation(title: String, subtitle: String?): RemoteViews {
        val views = RemoteViews(context.packageName, R.layout.autofill_item)
        views.setTextViewText(R.id.autofill_title, title)
        if (subtitle == null) {
            views.setViewVisibility(R.id.autofill_subtitle, View.GONE)
        } else {
            views.setTextViewText(R.id.autofill_subtitle, subtitle)
        }
        return views
    }
}
