package io.github.stephanbaum.mosaic.autofill

import android.os.CancellationSignal
import android.service.autofill.AutofillService
import android.service.autofill.FillCallback
import android.service.autofill.FillRequest
import android.service.autofill.FillResponse
import android.service.autofill.SaveCallback
import android.service.autofill.SaveRequest
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.SessionState
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/**
 * The Autofill service (ADR-0092). It answers a fill request with datasets that carry no value: the values are
 * released by [FillAuthActivity] after the user taps one (brief section 4: no fill without a trusted gesture).
 *
 *  - Locked: one "Unlock Mosaic" entry; the unlock is the app's own unlock flow, in [FillAuthActivity].
 *  - Unlocked: one authenticated dataset per login that the core says matches.
 *  - No login: "No login for <app>", which opens the app's link screen.
 *
 * The core decides which fields are fillable, what the target is and what matches; this class only reads the
 * structure and builds the response. No compatibility mode, no inline suggestions, no save request.
 */
class MosaicAutofillService : AutofillService() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    override fun onFillRequest(request: FillRequest, cancellationSignal: CancellationSignal, callback: FillCallback) {
        val job =
            scope.launch {
                val response =
                    try {
                        respond(request)
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        null
                    }
                callback.onSuccess(response)
            }
        cancellationSignal.setOnCancelListener { job.cancel() }
    }

    /** Mosaic never asks to save: the response carries no save information, so the platform never calls this. */
    override fun onSaveRequest(request: SaveRequest, callback: SaveCallback) {
        callback.onSuccess()
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    private suspend fun respond(request: FillRequest): FillResponse? {
        val app = application as MosaicApp
        val context = request.fillContexts.lastOrNull() ?: return null
        val parsed = StructureReader.read(context.structure)
        val planner = FillPlanner(app.holder)
        val factory = FillResponseFactory(this)
        return when (val prepared = planner.prepare(this, parsed)) {
            Prepared.NothingToFill -> null
            is Prepared.CannotCheck -> factory.cannotCheckResponse(prepared.info)
            is Prepared.Ready -> {
                if (app.holder.state() !is SessionState.Unlocked) {
                    factory.unlockResponse(prepared.info)
                } else {
                    val candidates = planner.candidates(prepared.target)
                    val warning = planner.warning(prepared.target) != null
                    val label =
                        if (planner.isApp(prepared.target)) CallerReader.labelOf(this, prepared.info.callerPackage) else null
                    factory.candidatesResponse(prepared.info, candidates.candidates, warning, label)
                }
            }
        }
    }
}
