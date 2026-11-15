package io.github.stephanbaum.mosaic.autofill

import android.content.Context
import android.content.pm.ApplicationInfo
import android.content.res.Resources
import io.github.stephanbaum.mosaic.ffi.LinkCandidate
import io.github.stephanbaum.mosaic.ffi.LinkOffer
import io.github.stephanbaum.mosaic.ffi.LinkReason
import io.github.stephanbaum.mosaic.ui.l10n.L10n

/** How a one-tap link is confirmed: the system biometric prompt, or the confirm screen of [FillAuthActivity]. */
enum class LinkConfirm { Biometric, Screen }

/** A link the user tapped, waiting for the confirmation. Lives in the memory of the screen only; holds no secret. */
class LinkPending(
    val sessionId: String,
    val offer: LinkOffer,
    val chosen: LinkCandidate,
    val appLabel: String,
)

/** The decisions of the one-tap link flow that need no Android classes (ADR-0092 amendment). */
object LinkFlow {
    /** At most this many link entries per response (spec section 5.5.2). */
    const val MAX_ENTRIES = 3

    /**
     * The stores whose review is the only barrier for an imported login (no certificate on the stored URL):
     * Google Play and the Samsung Galaxy Store. Compared byte for byte.
     */
    val TRUSTED_INSTALLERS: Set<String> = setOf("com.android.vending", "com.sec.android.app.samsungapps")

    fun confirmMode(strongBiometricAvailable: Boolean): LinkConfirm =
        if (strongBiometricAvailable) LinkConfirm.Biometric else LinkConfirm.Screen

    /** Only the biometric confirmation fills in the same step (spec section 5.5.2). */
    fun fillsAfterLink(mode: LinkConfirm): Boolean = mode == LinkConfirm.Biometric

    /**
     * Test seam of debuggable builds only (like [FillAudit]): the instrumented test app is installed by adb, never
     * by a store, so a test can ask for every installer to count as trusted. Release builds ignore the request.
     */
    @Volatile
    private var anyInstallerTrusted = false

    fun trustAnyInstallerForTests(context: Context, trusted: Boolean) {
        anyInstallerTrusted = trusted && (context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE) != 0
    }

    /** True when [installer] (a package name, or null) is a trusted store. */
    fun isTrustedInstaller(installer: String?): Boolean = installer != null && installer in TRUSTED_INSTALLERS

    /**
     * Whether the install source of an app counts as a trusted store. From API 30 both the installing and the
     * initiating package must be trusted: `installingPackageName` alone can be set to a store by a privileged
     * installer (adb, a shell session), while the initiating package names who started the install. Before API 30
     * the initiator is not available, so nothing is trusted there and the user keeps the link screen.
     */
    fun installSourceTrusted(sdkInt: Int, installing: String?, initiating: String?): Boolean =
        anyInstallerTrusted ||
            (sdkInt >= 30 && isTrustedInstaller(installing) && isTrustedInstaller(initiating))

    /**
     * Whether a link candidate may be offered (at the list and again at the tap). A candidate verified by the
     * website is not gated by the install source; an imported one (no certificate) only when [installTrusted].
     * Without it the user keeps the link screen, which shows the installer and "Not verified".
     */
    fun mayOffer(imported: Boolean, installTrusted: Boolean): Boolean = !imported || installTrusted

    /** At most [MAX_ENTRIES] of [candidates] that may be offered; [installTrusted] only matters for imported ones. */
    fun <T> selectEntries(candidates: List<T>, isImported: (T) -> Boolean, installTrusted: Boolean): List<T> =
        candidates.filter { mayOffer(isImported(it), installTrusted) }.take(MAX_ENTRIES)
}

/**
 * The line that names why a login is offered: the website that lists the app, or that the login was imported.
 * [forConfirmation] is the text of the biometric prompt and the confirm screen: for an imported login it says that
 * the website has not verified the app.
 */
fun linkReasonText(resources: Resources, reason: LinkReason, forConfirmation: Boolean): String =
    when (reason) {
        is LinkReason.VerifiedBy -> L10n.linkVerified(resources, reason.host)
        is LinkReason.ImportedForApp ->
            if (forConfirmation) L10n.linkImportedNotVerified(resources) else L10n.linkImported(resources)
    }

fun isImported(reason: LinkReason): Boolean = reason is LinkReason.ImportedForApp
