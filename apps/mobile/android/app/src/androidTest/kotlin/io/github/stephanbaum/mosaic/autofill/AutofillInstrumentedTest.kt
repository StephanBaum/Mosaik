package io.github.stephanbaum.mosaic.autofill

import android.accessibilityservice.AccessibilityServiceInfo
import android.graphics.Rect
import android.os.ParcelFileDescriptor
import android.view.accessibility.AccessibilityNodeInfo
import android.view.autofill.AutofillManager
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import io.github.stephanbaum.mosaic.DeviceLock
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.AndroidCaller
import io.github.stephanbaum.mosaic.ffi.FieldRefusal
import io.github.stephanbaum.mosaic.ffi.FillTargetKind
import io.github.stephanbaum.mosaic.ffi.ItemKey
import io.github.stephanbaum.mosaic.ffi.LoginDraft
import io.github.stephanbaum.mosaic.ffi.MosaicTestHooks
import io.github.stephanbaum.mosaic.ffi.OtpEdit
import io.github.stephanbaum.mosaic.ffi.ResetConfirm
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.ffi.UrlDraft
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The Autofill service against a real client app (docs/MOBILE.md section 6.3). The client is
 * [AutofillClientActivity] in this test APK; a login is linked to its package and its signer, the way the link
 * screen does it. The account is a local test account from the test hooks of the debug core library.
 */
@RunWith(AndroidJUnit4::class)
class AutofillInstrumentedTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val context = instrumentation.targetContext
    private val clientPackage = instrumentation.context.packageName
    private val service = "${context.packageName}/${MosaicAutofillService::class.java.name}"
    private val email = "autofill@example.invalid"
    private val password = "correct horse battery staple"
    private var setPin = false
    private lateinit var app: MosaicApp
    private lateinit var linked: ItemKey

    @Before
    fun setUp() {
        setPin = DeviceLock.ensure()
        app = context.applicationContext as MosaicApp
        val signers = checkNotNull(CallerReader.signersOf(context, clientPackage)) { "the test package is not visible" }
        runBlocking {
            app.holder.call { core ->
                resetAccount(core)
                val account = MosaicTestHooks(core).createLocalAccount(email, password.toByteArray())
                account.secretKey.wipe()
                val vault = core.vaults().first { it.available }.vaultId
                // A login of a website with the same name: it must never fill the app.
                core.saveLogin(draft(vault, "GitHub web", "web-user", "web-secret", "https://github.com"))
                linked = core.saveLogin(draft(vault, "GitHub", "octocat", "hunter2", null))
                core.linkAndroidApp(linked.vaultId, linked.itemId, AndroidCaller(clientPackage, signers, null, null))
            }
        }
    }

    @After
    fun tearDown() {
        shell("settings delete secure autofill_service")
        runBlocking { app.holder.call { resetAccount(it) } }
        if (setPin) DeviceLock.release()
    }

    private fun draft(vault: String, title: String, user: String, secret: String, url: String?) =
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

    private fun resetAccount(core: io.github.stephanbaum.mosaic.ffi.MosaicCore) {
        val existing =
            when (val state = core.state()) {
                is SessionState.Locked -> state.email
                is SessionState.Unlocked -> state.email
                else -> null
            }
        if (existing != null) core.resetDevice(ResetConfirm(typedEmail = existing))
    }

    private fun shell(command: String): String {
        val descriptor = instrumentation.uiAutomation.executeShellCommand(command)
        return ParcelFileDescriptor.AutoCloseInputStream(descriptor).use { String(it.readBytes()) }
    }

    // ---- the system's popup -----------------------------------------------------------------------

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

    private fun findText(text: String): AccessibilityNodeInfo? {
        for (root in windowNodes()) {
            val found = root.findAccessibilityNodeInfosByText(text)
            if (found.isNotEmpty()) return found[0]
        }
        return null
    }

    private fun <T : Any> waitFor(timeoutMs: Long, probe: () -> T?): T? {
        val end = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < end) {
            probe()?.let { return it }
            Thread.sleep(250)
        }
        return null
    }

    private fun tap(node: AccessibilityNodeInfo) {
        val bounds = Rect()
        node.getBoundsInScreen(bounds)
        shell("input tap ${bounds.centerX()} ${bounds.centerY()}")
    }

    // ---- tests --------------------------------------------------------------------------------------

    @Test
    fun fillsLinkedAppAfterDatasetTap() {
        shell("settings put secure autofill_service $service")
        ActivityScenario.launch(AutofillClientActivity::class.java).use { scenario ->
            scenario.onActivity { activity ->
                activity.username.requestFocus()
                activity.getSystemService(AutofillManager::class.java).requestAutofill(activity.username)
            }
            val entry = waitFor(30_000) { findText("GitHub") }
            if (entry == null) {
                val seen = ArrayList<String>()
                for (root in windowNodes()) texts(root, seen)
                fail("the dataset did not show; the screen shows: $seen")
            }
            // Before the tap the fields are empty: no value reached the client.
            scenario.onActivity {
                assertEquals("", it.username.text.toString())
                assertEquals("", it.password.text.toString())
            }
            tap(entry!!)
            val filled =
                waitFor(30_000) {
                    var values: Pair<String, String>? = null
                    scenario.onActivity { values = Pair(it.username.text.toString(), it.password.text.toString()) }
                    values?.takeIf { it.first.isNotEmpty() }
                }
            assertNotNull("nothing was filled after the tap", filled)
            assertEquals("octocat", filled!!.first)
            assertEquals("hunter2", filled.second)
        }
    }

    @Test
    fun noDatasetValuesBeforeTap() {
        ActivityScenario.launch(AutofillClientActivity::class.java).use { scenario ->
            var fields: FillFields? = null
            scenario.onActivity { fields = FillFields(it.username.autofillId, it.password.autofillId, null) }
            val info = FillRequestInfo(clientPackage, null, null, checkNotNull(fields))
            val planner = FillPlanner(app.holder)
            val factory = FillResponseFactory(context)
            runBlocking {
                val target = checkNotNull(planner.target(context, info)) { "the client app could not be checked" }
                val found = planner.candidates(target)
                assertEquals("only the linked login matches the app", 1, found.candidates.size)
                assertEquals("octocat", found.candidates[0].username)

                assertNotNull(factory.candidatesResponse(info, found.candidates, false, "Client"))
                val datasets = FillAudit.last
                assertTrue("the response has datasets", datasets.isNotEmpty())
                for (dataset in datasets) {
                    assertTrue("every dataset needs authentication", dataset.authenticated)
                    assertEquals("no dataset carries a value before the tap", 0, dataset.nonNullValues)
                }

                factory.unlockResponse(info)
                assertTrue("the locked response holds no dataset", FillAudit.last.isEmpty())
            }
        }
    }

    @Test
    fun ignoresWebDomainFromNonBrowserApp() {
        val planner = FillPlanner(app.holder)
        val info = FillRequestInfo(clientPackage, "github.com", "https", FillFields(null, null, null))
        runBlocking {
            val target = checkNotNull(planner.target(context, info)) { "the client app could not be checked" }
            // The app claims github.com, but it is no allowlisted browser: the claim is ignored.
            assertTrue("the target is the app, not the claimed site", target.kind is FillTargetKind.AndroidApp)
            val found = planner.candidates(target)
            assertEquals(listOf("GitHub"), found.candidates.map { it.title })
        }
    }

    @Test
    fun excludesForeignFrameFields() {
        fun node(domain: String, hint: String?, focused: Boolean) =
            RawNode(
                autofillId = null,
                focused = focused,
                visible = true,
                webDomain = domain,
                webScheme = "https",
                inputType = 0,
                autofillHints = if (hint == null) emptyList() else listOf(hint),
                htmlTag = "input",
                htmlAttrs = emptyList(),
                idEntry = null,
                hint = null,
            )
        runBlocking {
            // The page of a.example holds a user name field; a frame of b.example holds a password field.
            val page = ParsedStructure(clientPackage, listOf(node("a.example", "username", true), node("b.example", "password", false)))
            val selection = app.holder.call { it.selectFillFields(page.fieldNodes()) }
            assertNull("no refusal while the focus is on the page", selection.refused)
            assertEquals(0u, selection.username)
            assertNull("the password field of the foreign frame is not filled", selection.password)

            // The focus is in the foreign frame: the request is refused.
            val framed = ParsedStructure(clientPackage, listOf(node("a.example", "username", false), node("b.example", "password", true)))
            val refused = app.holder.call { it.selectFillFields(framed.fieldNodes()) }
            assertEquals(FieldRefusal.FOREIGN_FRAME, refused.refused)
            assertNull(refused.username)
            assertNull(refused.password)
        }
    }
}
