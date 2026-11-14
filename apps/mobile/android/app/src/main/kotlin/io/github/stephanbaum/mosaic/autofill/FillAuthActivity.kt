package io.github.stephanbaum.mosaic.autofill

import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.service.autofill.FillResponse
import android.view.autofill.AutofillManager
import androidx.activity.OnBackPressedCallback
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.FillCandidates
import io.github.stephanbaum.mosaic.ffi.FillTarget
import io.github.stephanbaum.mosaic.ffi.ItemKey
import io.github.stephanbaum.mosaic.ffi.OriginWarning
import io.github.stephanbaum.mosaic.provider.ProviderFlows
import io.github.stephanbaum.mosaic.provider.ProviderGate
import io.github.stephanbaum.mosaic.security.BiometricGate
import io.github.stephanbaum.mosaic.security.ErrorCodes
import io.github.stephanbaum.mosaic.security.PromptText
import io.github.stephanbaum.mosaic.security.ProviderActivity
import io.github.stephanbaum.mosaic.security.SecureActivity
import io.github.stephanbaum.mosaic.ui.MosaicTheme
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.Line
import io.github.stephanbaum.mosaic.ui.components.Notice
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.QuietButton
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.Working
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.link.AppIdentityPanel
import io.github.stephanbaum.mosaic.ui.link.installerOf
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

/**
 * The authentication activity of the Autofill service (ADR-0092). The platform starts it through the pending
 * intents of the response, in one of four modes:
 *
 *  - **list** (the "Unlock Mosaic" entry): unlock, then answer with a response of per-dataset-authenticated
 *    datasets. The flow stays open, because the next step is the user's tap.
 *  - **fill** (a tap on a dataset): unlock if the session ended, look the target up again, check that the tapped
 *    login is still a candidate, and answer with **one** dataset holding that login's values. Then the flow ends.
 *    A look-alike or punycode address is asked about first, on a confirm screen of this activity (M10-G4 item 4):
 *    the warning is derived again from the core on every tap, a failure to derive it stops the fill, and the
 *    answer lives in this screen's memory only, so no intent extra can skip it. No dataset holds a value before.
 *  - **link** (a tap on a "Link <title> and fill" entry, ADR-0092 amendment): read the caller and the offer again,
 *    check the tapped login is still in the offer (and, for an imported login, that the installer is still a trusted
 *    store), then ask for the confirmation. A strong biometric prompt links through the fill session and fills in the
 *    same step; without one, a confirm screen links only. Nothing is written before the confirmation.
 *  - **dismiss**: close.
 *
 * Everything is derived again from the caller's package and the core: the intent carries no value and no match.
 */
class FillAuthActivity : SecureActivity(), ProviderActivity {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val app = application as MosaicApp
        // Back abandons the flow, whatever screen is showing: a flow left open would grant the provider grace for
        // a step that never comes (RT-android-2).
        onBackPressedDispatcher.addCallback(
            this,
            object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() = abandon(app)
            },
        )
        val mode = intent.getStringExtra(FillIntents.EXTRA_MODE)
        val info = FillRequestInfo.fromIntent(intent)
        if (mode == null || mode == FillIntents.MODE_DISMISS || info == null) {
            setResult(RESULT_CANCELED)
            finish()
            return
        }
        setContent {
            MosaicTheme {
                ProviderGate(app, ProviderFlows.AUTOFILL, onClose = { abandon(app) }) {
                    Runner(app, mode, info)
                }
            }
        }
    }

    @Composable
    private fun Runner(app: MosaicApp, mode: String, info: FillRequestInfo) {
        val resources = LocalContext.current.resources
        var failure by remember { mutableStateOf<String?>(null) }
        var warning by remember { mutableStateOf<OriginWarning?>(null) }
        var confirmed by remember { mutableStateOf(false) }
        var linkPending by remember { mutableStateOf<LinkPending?>(null) }
        LaunchedEffect(confirmed) {
            try {
                val data = work(app, mode, info, confirmed)
                finishWith(app, mode, data)
            } catch (e: CancellationException) {
                throw e
            } catch (e: NeedsConfirmation) {
                warning = e.warning
            } catch (e: NeedsLinkConfirm) {
                linkPending = e.pending
            } catch (e: NoLongerMatches) {
                failure = "noMatch"
            } catch (e: Exception) {
                failure = ErrorCodes.of(e)
            }
        }
        val code = failure
        val pending = warning
        if (code == null && pending != null) {
            ScreenFrame {
                TitleText(L10n.autofillWarning(resources))
                BodyText(
                    when (pending) {
                        is OriginWarning.Punycode -> L10n.extWarnPunycode(resources, pending.unicode)
                        is OriginWarning.Lookalike -> L10n.extWarnLookalike(resources, pending.savedHost)
                    },
                )
                val address = info.webDomain
                if (address != null) BodyText(address)
                PrimaryButton(
                    L10n.extConfirmFillButton(resources),
                    onClick = {
                        warning = null
                        confirmed = true
                    },
                )
                QuietButton(L10n.commonCancel(resources), onClick = { abandon(app) })
            }
        } else if (code == null && linkPending != null) {
            LinkConfirmation(app, info, checkNotNull(linkPending), onFailure = { failure = it })
        } else if (code == null) {
            ScreenFrame { Working(L10n.commonWorking(resources)) }
        } else {
            ScreenFrame {
                TitleText(L10n.appName(resources))
                ErrorText(errorText(resources, code))
                PrimaryButton(L10n.commonClose(resources), onClick = { abandon(app) })
            }
        }
    }

    private suspend fun work(app: MosaicApp, mode: String, info: FillRequestInfo, confirmed: Boolean): Intent? {
        val planner = FillPlanner(app.holder)
        val factory = FillResponseFactory(this, app.linkTokens)
        val target = planner.target(this, info)
        if (target == null) {
            return if (mode == FillIntents.MODE_LIST) result(factory.cannotCheckResponse(info)) else null
        }
        val candidates = planner.candidates(target)
        if (mode == FillIntents.MODE_LINK) throw NeedsLinkConfirm(prepareLink(app, planner, target, info, candidates))
        return if (mode == FillIntents.MODE_LIST) {
            val warning = planner.warning(target) != null
            val label = planner.linkLabel(this, target, info.callerPackage)
            val links =
                if (candidates.candidates.isEmpty() && label != null) {
                    planner.linkEntries(this, candidates.sessionId, info.callerPackage)
                } else {
                    emptyList()
                }
            val response = factory.candidatesResponse(info, candidates.candidates, warning, label, links)
            if (response == null) null else result(response)
        } else {
            if (!confirmed) {
                // Not through the planner's lenient reader: an error here must stop the fill, never skip the question.
                val warning = app.holder.call { it.fillWarning(target) }
                if (warning != null) throw NeedsConfirmation(warning)
            }
            fill(app, factory, info, candidates.sessionId, candidates.candidates.map { it.key })
        }
    }

    /**
     * The link the user tapped, checked against the present state: the caller's signers were read again by the
     * planner (so the session holds the signer of the app in front of the user), the app has no matching login now,
     * the offer is recomputed in this new session and holds the tapped login, and an imported login still needs a
     * trusted installer. Anything else: [NoLongerMatches]. Errors of the core stop the flow (no lenient reader).
     */
    private suspend fun prepareLink(
        app: MosaicApp,
        planner: FillPlanner,
        target: FillTarget,
        info: FillRequestInfo,
        found: FillCandidates,
    ): LinkPending {
        if (found.candidates.isNotEmpty()) throw NoLongerMatches()
        val label = planner.linkLabel(this, target, info.callerPackage) ?: throw NoLongerMatches()
        val vault = intent.getStringExtra(FillIntents.EXTRA_ITEM_VAULT)
        val item = intent.getStringExtra(FillIntents.EXTRA_ITEM_ID)
        if (vault == null || item == null) throw NoLongerMatches()
        val key = ItemKey(vault, item)
        val offer = app.holder.call { it.fillLinkOffer(found.sessionId) }
        val chosen = offer.candidates.firstOrNull { it.key == key } ?: throw NoLongerMatches()
        if (!LinkFlow.mayOffer(isImported(chosen.reason), installerOf(packageManager, info.callerPackage))) {
            throw NoLongerMatches()
        }
        return LinkPending(found.sessionId, offer, chosen, label)
    }

    /**
     * The confirmation of a link (never before the tap): the strong biometric prompt, or the confirm screen. The
     * prompt links and fills; the screen links only, and the user taps the field again.
     */
    @Composable
    private fun LinkConfirmation(app: MosaicApp, info: FillRequestInfo, link: LinkPending, onFailure: (String) -> Unit) {
        val context = LocalContext.current
        val resources = context.resources
        val mode =
            remember {
                // Strong biometrics are only certain from API 30 (BIOMETRIC_STRONG); earlier phones use the screen.
                LinkFlow.confirmMode(Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && BiometricGate().isAvailable(context))
            }
        val reason = linkReasonText(resources, link.chosen.reason, true)
        if (mode == LinkConfirm.Biometric) {
            LaunchedEffect(link) {
                try {
                    val text =
                        PromptText(
                            title = L10n.linkConfirmTitle(resources, link.chosen.title, link.appLabel),
                            subtitle = L10n.linkConfirmSubtitle(resources, link.offer.packageName, link.offer.signerPrefix),
                            negativeButton = L10n.commonCancel(resources),
                        )
                    if (!BiometricGate().confirm(context, text, reason)) {
                        abandon(app)
                    } else {
                        finishWith(app, FillIntents.MODE_LINK, linkThenMaybeFill(app, info, link, mode))
                    }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Exception) {
                    onFailure(ErrorCodes.of(e))
                }
            }
            ScreenFrame { Working(L10n.commonWorking(resources)) }
        } else {
            val scope = rememberCoroutineScope()
            var busy by remember { mutableStateOf(false) }
            var linked by remember { mutableStateOf(false) }
            val installer = remember(link) { installerOf(packageManager, link.offer.packageName) }
            ScreenFrame {
                TitleText(L10n.linkTitle(resources))
                AppIdentityPanel(
                    label = link.appLabel,
                    packageName = link.offer.packageName,
                    installer = installer,
                    fingerprint = link.offer.signerPrefix,
                )
                Line(reason)
                if (linked) {
                    Notice(L10n.linkDone(resources, link.chosen.title))
                    PrimaryButton(L10n.commonDone(resources), onClick = { abandon(app) })
                } else {
                    BodyText(L10n.linkConfirmFallbackHint(resources))
                    PrimaryButton(
                        L10n.linkConfirm(resources, link.chosen.title),
                        enabled = !busy,
                        onClick = {
                            busy = true
                            scope.launch {
                                try {
                                    linkThenMaybeFill(app, info, link, mode)
                                    linked = true
                                } catch (e: CancellationException) {
                                    throw e
                                } catch (e: Exception) {
                                    onFailure(ErrorCodes.of(e))
                                }
                            }
                        },
                    )
                    QuietButton(L10n.commonCancel(resources), onClick = { abandon(app) })
                }
            }
        }
    }

    /** Links through the fill session (its signer, not a request's), then fills when the confirmation was the biometric. */
    private suspend fun linkThenMaybeFill(app: MosaicApp, info: FillRequestInfo, link: LinkPending, mode: LinkConfirm): Intent? {
        app.holder.call { it.linkAppInFillSession(link.sessionId, link.chosen.key) }
        if (!LinkFlow.fillsAfterLink(mode)) return null
        return fill(app, FillResponseFactory(this, app.linkTokens), info, link.sessionId, listOf(link.chosen.key))
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
        if (mode == FillIntents.MODE_FILL || mode == FillIntents.MODE_LINK) app.lockPolicy.closeProviderFlow(ProviderFlows.AUTOFILL)
        finish()
    }

    /** The user left without a result (Back, or Close on a failure screen): cancel and end the flow. */
    private fun abandon(app: MosaicApp) {
        setResult(RESULT_CANCELED)
        app.lockPolicy.closeProviderFlow(ProviderFlows.AUTOFILL)
        finish()
    }

    /** The address looks like another site or uses punycode: the user confirms before any value is read. */
    private class NeedsConfirmation(val warning: OriginWarning) : Exception()

    /** A link entry was tapped: the user confirms it (prompt or screen) before anything is written. */
    private class NeedsLinkConfirm(val pending: LinkPending) : Exception()

    /** The tapped login is no longer a candidate for this request (the match changed since the list was shown). */
    private class NoLongerMatches : Exception()
}
