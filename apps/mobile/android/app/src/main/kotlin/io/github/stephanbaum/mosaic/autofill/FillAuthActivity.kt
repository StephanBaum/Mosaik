package io.github.stephanbaum.mosaic.autofill

import android.content.Intent
import android.os.Bundle
import android.service.autofill.FillResponse
import android.view.autofill.AutofillManager
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.ItemKey
import io.github.stephanbaum.mosaic.provider.ProviderFlows
import io.github.stephanbaum.mosaic.provider.ProviderGate
import io.github.stephanbaum.mosaic.security.ErrorCodes
import io.github.stephanbaum.mosaic.security.SecureActivity
import io.github.stephanbaum.mosaic.ui.MosaicTheme
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import kotlinx.coroutines.CancellationException

/**
 * The authentication activity of the Autofill service (ADR-0092). The platform starts it through the pending
 * intents of the response, in one of three modes:
 *
 *  - **list** (the "Unlock Mosaic" entry): unlock, then answer with a response of per-dataset-authenticated
 *    datasets. The flow stays open, because the next step is the user's tap.
 *  - **fill** (a tap on a dataset): unlock if the session ended, look the target up again, check that the tapped
 *    login is still a candidate, and answer with **one** dataset holding that login's values. Then the flow ends.
 *  - **dismiss**: close.
 *
 * Everything is derived again from the caller's package and the core: the intent carries no value and no match.
 */
class FillAuthActivity : SecureActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val app = application as MosaicApp
        val mode = intent.getStringExtra(FillIntents.EXTRA_MODE)
        val info = FillRequestInfo.fromIntent(intent)
        if (mode == null || mode == FillIntents.MODE_DISMISS || info == null) {
            setResult(RESULT_CANCELED)
            finish()
            return
        }
        setContent {
            MosaicTheme {
                ProviderGate(app, ProviderFlows.AUTOFILL) {
                    Runner(app, mode, info)
                }
            }
        }
    }

    @Composable
    private fun Runner(app: MosaicApp, mode: String, info: FillRequestInfo) {
        val resources = LocalContext.current.resources
        var failure by remember { mutableStateOf<String?>(null) }
        LaunchedEffect(Unit) {
            try {
                val data = work(app, mode, info)
                finishWith(app, mode, data)
            } catch (e: CancellationException) {
                throw e
            } catch (e: NoLongerMatches) {
                failure = "noMatch"
            } catch (e: Exception) {
                failure = ErrorCodes.of(e)
            }
        }
        val code = failure
        if (code == null) {
            ScreenFrame { }
        } else {
            ScreenFrame {
                TitleText(L10n.appName(resources))
                ErrorText(errorText(resources, code))
                PrimaryButton(L10n.commonClose(resources), onClick = { finishWith(app, mode, null) })
            }
        }
    }

    private suspend fun work(app: MosaicApp, mode: String, info: FillRequestInfo): Intent? {
        val planner = FillPlanner(app.holder)
        val factory = FillResponseFactory(this)
        val target = planner.target(this, info)
        if (target == null) {
            return if (mode == FillIntents.MODE_LIST) result(factory.cannotCheckResponse(info)) else null
        }
        val candidates = planner.candidates(target)
        return if (mode == FillIntents.MODE_LIST) {
            val warning = planner.warning(target) != null
            val label = if (planner.isApp(target)) CallerReader.labelOf(this, info.callerPackage) else null
            val response = factory.candidatesResponse(info, candidates.candidates, warning, label)
            if (response == null) null else result(response)
        } else {
            fill(app, factory, info, candidates.sessionId, candidates.candidates.map { it.key })
        }
    }

    /** The values of the tapped login, checked against the fresh candidates, as the one dataset to return. */
    private suspend fun fill(
        app: MosaicApp,
        factory: FillResponseFactory,
        info: FillRequestInfo,
        sessionId: String,
        allowed: List<ItemKey>,
    ): Intent? {
        val vault = intent.getStringExtra(FillIntents.EXTRA_ITEM_VAULT)
        val item = intent.getStringExtra(FillIntents.EXTRA_ITEM_ID)
        val kind = intent.getStringExtra(FillIntents.EXTRA_KIND)
        if (vault == null || item == null || kind == null) return null
        val key = ItemKey(vault, item)
        if (key !in allowed) throw NoLongerMatches()
        var username: String? = null
        var secret: String? = null
        if (kind == FillIntents.KIND_OTP) {
            val code = app.holder.call { it.fillOtp(sessionId, key) }
            try {
                secret = toText(code.exposeBytes())
            } finally {
                code.wipe()
            }
        } else {
            val values = app.holder.call { it.fillValues(sessionId, key) }
            username = values.username
            val password = values.password
            if (password != null) {
                try {
                    secret = toText(password.exposeBytes())
                } finally {
                    password.wipe()
                }
            }
        }
        val dataset = factory.valuesDataset(info, kind, username, secret)
        return Intent().putExtra(AutofillManager.EXTRA_AUTHENTICATION_RESULT, dataset)
    }

    /** The platform takes the value as text, so it becomes a `String` here; the bytes from the core are wiped. */
    private fun toText(bytes: ByteArray): String {
        try {
            return String(bytes, Charsets.UTF_8)
        } finally {
            bytes.fill(0)
        }
    }

    private fun result(response: FillResponse): Intent =
        Intent().putExtra(AutofillManager.EXTRA_AUTHENTICATION_RESULT, response)

    private fun finishWith(app: MosaicApp, mode: String, data: Intent?) {
        if (data == null) setResult(RESULT_CANCELED) else setResult(RESULT_OK, data)
        // A tap ends the flow; the unlock step (list) leaves it open for the tap that follows.
        if (mode == FillIntents.MODE_FILL) app.lockPolicy.closeProviderFlow(ProviderFlows.AUTOFILL)
        finish()
    }

    /** The tapped login is no longer a candidate for this request (the match changed since the list was shown). */
    private class NoLongerMatches : Exception()
}
