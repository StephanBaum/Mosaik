package io.github.stephanbaum.mosaic.autofill

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import io.github.stephanbaum.mosaic.AppForeground
import io.github.stephanbaum.mosaic.DeviceLock
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.AndroidCaller
import io.github.stephanbaum.mosaic.ffi.FieldRefusal
import io.github.stephanbaum.mosaic.ffi.FillTarget
import io.github.stephanbaum.mosaic.ffi.FillTargetKind
import io.github.stephanbaum.mosaic.ffi.ItemKey
import io.github.stephanbaum.mosaic.ffi.MosaicTestHooks
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
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
    private val kit = AutofillTestKit(instrumentation)
    private val email = "autofill@example.invalid"
    private val password = "correct horse battery staple"
    private lateinit var app: MosaicApp

    @Before
    fun setUp() {
        // The PIN stays after the class: removing a lock screen deletes the Keystore key of the secure storage, which
        // the core of this process has cached, and the tests that follow use the same core (see DeviceLock).
        DeviceLock.ensure()
        // A secure device can lock its screen again between two tests; the client must be visible to be tapped.
        kit.shell("input keyevent KEYCODE_WAKEUP")
        kit.shell("wm dismiss-keyguard")
        FillAudit.last = emptyList()
        app = context.applicationContext as MosaicApp
        AppForeground.hold(app)
        val signers = checkNotNull(CallerReader.signersOf(context, clientPackage)) { "the test package is not visible" }
        runBlocking {
            app.holder.call { core ->
              AppForeground.explainLock(app) {
                kit.resetAccount(core)
                val account = MosaicTestHooks(core).createLocalAccount(email, password.toByteArray())
                account.secretKey.wipe()
                val vault = core.vaults().first { it.available }.vaultId
                // A login of a website with the same name: it must never fill the app.
                core.saveLogin(kit.draft(vault, "GitHub web", "web-user", "web-secret", "https://github.com"))
                val linked = core.saveLogin(kit.draft(vault, "GitHub", "octocat", "hunter2", null))
                core.linkAndroidApp(linked.vaultId, linked.itemId, AndroidCaller(clientPackage, signers, null, null))
              }
            }
        }
    }

    @After
    fun tearDown() {
        kit.shell("settings delete secure autofill_service")
        kit.shell("input keyevent KEYCODE_HOME")
        runCatching { runBlocking { app.holder.call { kit.resetAccount(it) } } }
    }

    // ---- tests --------------------------------------------------------------------------------------

    @Test
    fun fillsLinkedAppAfterDatasetTap() {
        kit.showClientAndFocusUsername()
        val entry = kit.waitFor(30_000) { kit.findText("octocat") }
        assertNotNull("the dataset did not show; the screen shows: ${kit.screenTexts()}", entry)
        // Before the tap the fields are empty: no value reached the client.
        assertTrue(kit.findText("client-username")?.text.isNullOrEmpty())
        assertTrue(kit.findText("client-password")?.text.isNullOrEmpty())
        kit.tap(entry!!)
        val filled =
            kit.waitFor(30_000) {
                val user = kit.findText("client-username")?.text?.toString()
                val secret = kit.findText("client-password")?.text?.toString()
                if (user == "octocat" && secret != null) Pair(user, secret) else null
            }
        assertNotNull("nothing was filled after the tap; the screen shows: ${kit.screenTexts()}", filled)
        assertEquals("octocat", filled!!.first)
        assertEquals("hunter2", filled.second)
    }

    @Test
    fun noDatasetValuesBeforeTap() {
        kit.showClientAndFocusUsername()
        // The service records what its datasets hold (a seam of debuggable builds); the response is the real one.
        val datasets = kit.waitFor(30_000) { FillAudit.last.takeIf { it.isNotEmpty() } }
        assertNotNull("the service made no response; the screen shows: ${kit.screenTexts()}", datasets)
        for (dataset in datasets!!) {
            assertTrue("every dataset needs authentication", dataset.authenticated)
            assertEquals("no dataset carries a value before the tap", 0, dataset.nonNullValues)
        }
        // Nothing reached the client either.
        assertTrue(kit.findText("client-username")?.text.isNullOrEmpty())
        assertTrue(kit.findText("client-password")?.text.isNullOrEmpty())
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
        val webTarget = FillTarget(FillTargetKind.Web("https://a.example"), "a.example", null)
        runBlocking {
            // The page of a.example holds a user name field; a frame of b.example holds a password field.
            val page = ParsedStructure(clientPackage, listOf(node("a.example", "username", true), node("b.example", "password", false)))
            val selection = app.holder.call { it.selectFillFields(page.fieldNodes(), webTarget) }
            assertNull("no refusal while the focus is on the page", selection.refused)
            assertEquals(0u, selection.username)
            assertNull("the password field of the foreign frame is not filled", selection.password)

            // The focus is in the foreign frame: the request is refused.
            val framed = ParsedStructure(clientPackage, listOf(node("a.example", "username", false), node("b.example", "password", true)))
            val refused = app.holder.call { it.selectFillFields(framed.fieldNodes(), webTarget) }
            assertEquals(FieldRefusal.FOREIGN_FRAME, refused.refused)
            assertNull(refused.username)
            assertNull(refused.password)
        }
    }
}
