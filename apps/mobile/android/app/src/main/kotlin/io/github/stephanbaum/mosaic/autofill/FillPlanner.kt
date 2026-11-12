package io.github.stephanbaum.mosaic.autofill

import android.content.Context
import io.github.stephanbaum.mosaic.core.CoreHolder
import io.github.stephanbaum.mosaic.ffi.AndroidCaller
import io.github.stephanbaum.mosaic.ffi.FillCandidates
import io.github.stephanbaum.mosaic.ffi.FillTarget
import io.github.stephanbaum.mosaic.ffi.FillTargetKind
import io.github.stephanbaum.mosaic.ffi.OriginWarning
import kotlinx.coroutines.CancellationException

/** What a fill request is for, as far as it can be known without the vault. */
sealed interface Prepared {
    /** Nothing here to fill: no fillable field, or a field of a foreign frame was focused. */
    data object NothingToFill : Prepared

    /** The caller could not be identified (package not visible, no or several signers, an origin that is refused). */
    class CannotCheck(val info: FillRequestInfo) : Prepared

    class Ready(val info: FillRequestInfo, val target: FillTarget) : Prepared
}

/**
 * Asks the core what a request is for. The core decides everything: which fields are fillable
 * (`select_fill_fields`), what the target is (`fill_target_android`, which ignores a `webDomain` of any app that is
 * not an allowlisted browser) and which logins match (`fill_candidates`). This class never matches by itself.
 */
class FillPlanner(private val holder: CoreHolder) {
    /** Works while locked (both core calls are pure). */
    suspend fun prepare(context: Context, parsed: ParsedStructure): Prepared {
        if (parsed.callerPackage.isEmpty() || parsed.nodes.isEmpty()) return Prepared.NothingToFill
        val selection = holder.call { it.selectFillFields(parsed.fieldNodes()) }
        if (selection.refused != null) return Prepared.NothingToFill
        val fields =
            FillFields(
                username = parsed.idOf(selection.username),
                password = parsed.idOf(selection.password),
                otp = parsed.idOf(selection.otp),
            )
        if (fields.all.isEmpty()) return Prepared.NothingToFill
        val info = FillRequestInfo(parsed.callerPackage, parsed.webDomain, parsed.webScheme, fields)
        val target = target(context, info) ?: return Prepared.CannotCheck(info)
        return Prepared.Ready(info, target)
    }

    /** The target of [info], read again from the platform: current signers, then the core. Null: cannot check. */
    suspend fun target(context: Context, info: FillRequestInfo): FillTarget? {
        val signers = CallerReader.signersOf(context, info.callerPackage) ?: return null
        val caller = AndroidCaller(info.callerPackage, signers, info.webDomain, info.webScheme)
        return try {
            holder.call { it.fillTargetAndroid(caller) }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            null
        }
    }

    /** The logins that may fill [target], and the fill session their values are asked with. Needs an open session. */
    suspend fun candidates(target: FillTarget): FillCandidates = holder.call { it.fillCandidates(target) }

    /** The warning for a look-alike or punycode address, if any. */
    suspend fun warning(target: FillTarget): OriginWarning? =
        try {
            holder.call { it.fillWarning(target) }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            null
        }

    /** True when [target] is an app (not a page in a browser): only then can the user link a login to it. */
    fun isApp(target: FillTarget): Boolean = target.kind is FillTargetKind.AndroidApp
}
