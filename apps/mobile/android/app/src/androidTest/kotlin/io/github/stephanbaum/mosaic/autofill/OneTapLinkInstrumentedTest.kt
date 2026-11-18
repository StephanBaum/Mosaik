package io.github.stephanbaum.mosaic.autofill

import android.util.Base64
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import io.github.stephanbaum.mosaic.AppForeground
import io.github.stephanbaum.mosaic.DeviceLock
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.AndroidCaller
import io.github.stephanbaum.mosaic.ffi.MosaicTestHooks
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * One-tap linking (ADR-0092 amendment) against the client app of this test APK. The only login is an imported one,
 * `androidapp://<client package>` without a certificate, so nothing matches and the app is "unlinked". The CI
 * emulator has no enrolled biometric, so the confirm screen runs; the biometric path is a manual check
 * (docs/MOBILE.md section 7). The test APK comes from adb, not from a store: the tests that need a trusted
 * installer ask the debug seam of [LinkFlow] for it.
 */
@RunWith(AndroidJUnit4::class)
class OneTapLinkInstrumentedTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val context = instrumentation.targetContext
    private val res = context.resources
    private val clientPackage = instrumentation.context.packageName
    private val kit = AutofillTestKit(instrumentation)
    private val email = "onetap@example.invalid"
    private val password = "correct horse battery staple"
    private lateinit var app: MosaicApp
    private lateinit var vaultId: String
    private lateinit var itemId: String

    @Before
    fun setUp() {
        DeviceLock.ensure()
        kit.shell("input keyevent KEYCODE_WAKEUP")
        kit.shell("wm dismiss-keyguard")
        FillAudit.last = emptyList()
        LinkFlow.trustAnyInstallerForTests(context, false)
        app = context.applicationContext as MosaicApp
        AppForeground.hold(app)
        runBlocking {
            app.holder.call { core ->
                AppForeground.explainLock(app) {
                    kit.resetAccount(core)
                    val account = MosaicTestHooks(core).createLocalAccount(email, password.toByteArray())
                    account.secretKey.wipe()
                    val vault = core.vaults().first { it.available }.vaultId
                    val saved =
                        core.saveLogin(kit.draft(vault, "GitHub", "octocat", "hunter2", "androidapp://$clientPackage"))
                    vaultId = saved.vaultId
                    itemId = saved.itemId
                }
            }
        }
    }

    @After
    fun tearDown() {
        LinkFlow.trustAnyInstallerForTests(context, false)
        kit.shell("settings delete secure autofill_service")
        kit.shell("input keyevent KEYCODE_HOME")
        runCatching { runBlocking { app.holder.call { kit.resetAccount(it) } } }
    }

    /**
     * The cert-bound URL of the client app: the SHA-256 of its current signer, base64url, behind `?cert=`.
     * [CallerReader.signersOf] already gives the SHA-256 digests ([Signers.of]), so they are not hashed again.
     */
    private fun boundUrl(): String {
        val digest = checkNotNull(CallerReader.signersOf(context, clientPackage)).single()
        val cert = Base64.encodeToString(digest, Base64.URL_SAFE or Base64.NO_PADDING or Base64.NO_WRAP)
        return "androidapp://$clientPackage?cert=$cert"
    }

    /**
     * Cancel ends the flow and closes Mosaic's last screen, so the release lock policy locks the session (background
     * delay 0, ADR-0094) and the next request would get "Unlock Mosaic". The test waits for that background, marks
     * the app as foreground (as in [setUp]) and unlocks again with the stored Secret Key, the way the unlock screen
     * would.
     */
    private fun unlockAfterTheFlowClosed() {
        kit.waitFor(10_000) { true.takeIf { !app.foreground.value } }
        AppForeground.hold(app)
        runBlocking { app.holder.call { it.unlockPassword(password.toByteArray(), null) } }
    }

    /** True when the login of the test holds the cert-bound URL of the client app. */
    private fun boundUrlPresent(): Boolean {
        val expected = boundUrl()
        val detail = runBlocking { app.holder.call { it.item(vaultId, itemId) } }
        return detail.urls.any { it.url == expected }
    }

    @Test
    fun noValuesBeforeConfirm() {
        LinkFlow.trustAnyInstallerForTests(context, true)
        kit.showClientAndFocusUsername()
        val datasets = kit.waitFor(30_000) { FillAudit.last.takeIf { list -> list.any { it.kind == "link" } } }
        assertNotNull("no link entry; the screen shows: ${kit.screenTexts()}", datasets)
        for (dataset in datasets!!) {
            assertTrue(dataset.authenticated)
            assertEquals(0, dataset.nonNullValues)
        }
        assertEquals(1, datasets.count { it.kind == "link" })
        assertEquals(1, datasets.count { it.kind == "noLogin" })
        assertNotNull(kit.waitFor(10_000) { kit.findText(L10n.autofillLinkEntry(res, "GitHub")) })
        assertTrue(kit.findText("client-username")?.text.isNullOrEmpty())
        assertTrue(kit.findText("client-password")?.text.isNullOrEmpty())
    }

    @Test
    fun importedLoginFromAnUntrustedInstallerGetsNoLinkEntry() {
        // The seam is off: the test APK comes from adb, which is no trusted store.
        kit.showClientAndFocusUsername()
        val datasets = kit.waitFor(30_000) { FillAudit.last.takeIf { list -> list.any { it.kind == "noLogin" } } }
        assertNotNull("no response; the screen shows: ${kit.screenTexts()}", datasets)
        assertEquals(0, datasets!!.count { it.kind == "link" })
        assertNull(kit.findText(L10n.autofillLinkEntry(res, "GitHub")))
        assertFalse(boundUrlPresent())
    }

    @Test
    fun oneTapLinkNeedsConfirm() {
        LinkFlow.trustAnyInstallerForTests(context, true)
        kit.showClientAndFocusUsername()
        kit.tap(checkNotNull(kit.waitFor(30_000) { kit.findText(L10n.autofillLinkEntry(res, "GitHub")) }))
        val confirm = kit.waitFor(30_000) { kit.findText(L10n.linkConfirm(res, "GitHub")) }
        assertNotNull("no confirmation; the screen shows: ${kit.screenTexts()}", confirm)
        // An imported login: the screen says that the website has not confirmed the app.
        assertNotNull(kit.findText(L10n.linkImportedNotVerified(res)))
        assertFalse("linked before the confirm", boundUrlPresent())
        kit.tap(checkNotNull(kit.findText(L10n.commonCancel(res))))
        unlockAfterTheFlowClosed()
        assertFalse("linked after Cancel", boundUrlPresent())
        // Again, and confirm this time.
        kit.showClientAndFocusUsername()
        kit.tap(checkNotNull(kit.waitFor(30_000) { kit.findText(L10n.autofillLinkEntry(res, "GitHub")) }))
        kit.tap(checkNotNull(kit.waitFor(30_000) { kit.findText(L10n.linkConfirm(res, "GitHub")) }))
        assertNotNull(kit.waitFor(30_000) { kit.findText(L10n.linkDone(res, "GitHub")) })
        assertTrue(boundUrlPresent())
        // The screen path links only: nothing was filled.
        assertTrue(kit.findText("client-password")?.text.isNullOrEmpty())
    }

    @Test
    fun installerTrustedAtListButNotAtTapLinksNothing() {
        LinkFlow.trustAnyInstallerForTests(context, true)
        kit.showClientAndFocusUsername()
        val entry = checkNotNull(kit.waitFor(30_000) { kit.findText(L10n.autofillLinkEntry(res, "GitHub")) })
        // The install source is checked again at the tap: now it is untrusted.
        LinkFlow.trustAnyInstallerForTests(context, false)
        kit.tap(entry)
        // The probe runs first: a message argument would read the screen before the tap took effect.
        val confirm = kit.waitFor(8_000) { kit.findText(L10n.linkConfirm(res, "GitHub")) }
        assertNull("the confirmation opened for an untrusted installer; the screen shows: ${kit.screenTexts()}", confirm)
        assertFalse("linked for an untrusted installer", boundUrlPresent())
    }

    @Test
    fun aLoginThatMatchesByTheTapRefusesTheLink() {
        LinkFlow.trustAnyInstallerForTests(context, true)
        kit.showClientAndFocusUsername()
        val entry = checkNotNull(kit.waitFor(30_000) { kit.findText(L10n.autofillLinkEntry(res, "GitHub")) })
        // Between the list and the tap the app gets a login of its own (cert-bound), so it is no longer unlinked.
        // The binding goes through the link step: saving a cert-bound URL directly is refused (red team F1, #108).
        val signers = checkNotNull(CallerReader.signersOf(context, clientPackage))
        runBlocking {
            app.holder.call { core ->
                AppForeground.explainLock(app) {
                    val other = core.saveLogin(kit.draft(vaultId, "Other", "someone", "hunter3", null))
                    core.linkAndroidApp(other.vaultId, other.itemId, AndroidCaller(clientPackage, signers, null, null))
                }
            }
        }
        kit.tap(entry)
        val confirm = kit.waitFor(8_000) { kit.findText(L10n.linkConfirm(res, "GitHub")) }
        assertNull("the confirmation opened for an app that matches now; the screen shows: ${kit.screenTexts()}", confirm)
        assertFalse("linked although the app matches a login", boundUrlPresent())
    }
}
