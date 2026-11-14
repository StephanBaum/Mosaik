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

    /** True when [installer] (the installing package of the app, or null) is a trusted store. */
    fun isTrustedInstaller(installer: String?): Boolean =
        anyInstallerTrusted || (installer != null && installer in TRUSTED_INSTALLERS)

    /**
     * Whether a link candidate may be offered (at the list and again at the tap). A candidate verified by the
     * website is not gated by the installer; an imported one (no certificate) only when the app comes from a
     * trusted store. Without such an installer the user keeps the link screen, which shows it and "Not verified".
     */
    fun mayOffer(imported: Boolean, installer: String?): Boolean = !imported || isTrustedInstaller(installer)
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
