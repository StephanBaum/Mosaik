package io.github.stephanbaum.mosaic.autofill

import android.accessibilityservice.AccessibilityServiceInfo
import android.app.Instrumentation
import android.graphics.Rect
import android.os.ParcelFileDescriptor
import android.view.accessibility.AccessibilityNodeInfo
import io.github.stephanbaum.mosaic.ffi.LoginDraft
import io.github.stephanbaum.mosaic.ffi.MosaicCore
import io.github.stephanbaum.mosaic.ffi.OtpEdit
import io.github.stephanbaum.mosaic.ffi.ResetConfirm
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.ffi.UrlDraft
import org.junit.Assert.assertNotNull

/**
 * The helpers the Autofill instrumented tests share: shell commands, the accessibility tree of the popup of the
 * system, the client app of this test APK, and the account of the test hooks.
 */
class AutofillTestKit(private val instrumentation: Instrumentation) {
    private val context = instrumentation.targetContext
    private val clientPackage = instrumentation.context.packageName
    private val service = "${context.packageName}/${MosaicAutofillService::class.java.name}"

    fun draft(vault: String, title: String, user: String, secret: String, url: String?) =
        LoginDraft(
            vaultId = vault,
            itemId = null,
            title = title,
            username = user,
            password = secret.toByteArray(),
            urls = if (url == null) emptyList() else listOf(UrlDraft(url, false, false)),
            notes = null,
            otp = OtpEdit.Keep,
            favorite = false,
        )

    fun resetAccount(core: MosaicCore) {
        val existing =
            when (val state = core.state()) {
                is SessionState.Locked -> state.email
                is SessionState.Unlocked -> state.email
                else -> null
            }
        if (existing != null) core.resetDevice(ResetConfirm(typedEmail = existing))
    }

    fun shell(command: String): String {
        val descriptor = instrumentation.uiAutomation.executeShellCommand(command)
        return ParcelFileDescriptor.AutoCloseInputStream(descriptor).use { String(it.readBytes()) }
    }

    // ---- the popup of the system ---------------------------------------------------------------------

    private fun windowNodes(): List<AccessibilityNodeInfo> {
        val automation = instrumentation.uiAutomation
        val info = automation.serviceInfo
        if (info != null && info.flags and AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS == 0) {
            info.flags = info.flags or AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS
            automation.serviceInfo = info
        }
        return automation.windows.mapNotNull { it.root }
    }

    private fun texts(node: AccessibilityNodeInfo, out: MutableList<String>) {
        node.text?.let { out.add(it.toString()) }
        for (index in 0 until node.childCount) node.getChild(index)?.let { texts(it, out) }
    }

    fun findText(text: String): AccessibilityNodeInfo? {
        for (root in windowNodes()) {
            val found = root.findAccessibilityNodeInfosByText(text)
            if (found.isNotEmpty()) return found[0]
        }
        return null
    }

    fun <T : Any> waitFor(timeoutMs: Long, probe: () -> T?): T? {
        val end = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < end) {
            probe()?.let { return it }
            Thread.sleep(250)
        }
        return null
    }

    fun tap(node: AccessibilityNodeInfo) {
        val bounds = Rect()
        node.getBoundsInScreen(bounds)
        shell("input tap ${bounds.centerX()} ${bounds.centerY()}")
    }

    /** Starts the client app, enables the service, and focuses the user name field the way a user would. */
    fun showClientAndFocusUsername() {
        shell("settings put secure autofill_service $service")
        val start = "am start -n $clientPackage/${AutofillClientActivity::class.java.name}"
        shell(start)
        var field = waitFor(10_000) { findText("client-username") }
        if (field == null) {
            // The single instance can come back late after the previous test: ask once more.
            shell("wm dismiss-keyguard")
            shell(start)
            field = waitFor(30_000) { findText("client-username") }
        }
        val state = shell("dumpsys window").lines().filter { it.contains("mCurrentFocus") || it.contains("isKeyguardShowing") }
        assertNotNull("the client app did not show; focus: $state; the screen shows: ${screenTexts()}", field)
        tap(field!!)
    }

    fun screenTexts(): List<String> {
        val seen = ArrayList<String>()
        for (root in windowNodes()) texts(root, seen)
        return seen
    }
}
