package io.github.stephanbaum.mosaic.ui

import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.assertExists
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import io.github.stephanbaum.mosaic.DeviceLock
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.LoginDraft
import io.github.stephanbaum.mosaic.ffi.MosaicTestHooks
import io.github.stephanbaum.mosaic.ffi.OtpEdit
import io.github.stephanbaum.mosaic.ffi.ResetConfirm
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.ffi.UrlDraft
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Unlocking with the master password shows the vault (docs/MOBILE.md). The account is a local test account made
 * through the test hooks of the debug core library (no server); the test sets a device PIN first, because the
 * secure storage needs a lock screen, then locks the core and drives the real unlock screen.
 */
@RunWith(AndroidJUnit4::class)
class UnlockFlowTest {
    @get:Rule
    val compose = createEmptyComposeRule()

    private val email = "unlock-flow@example.invalid"
    private val password = "correct horse battery staple"
    private var setPin = false
    private lateinit var app: MosaicApp

    @Before
    fun setUp() {
        setPin = DeviceLock.ensure()
        app = InstrumentationRegistry.getInstrumentation().targetContext.applicationContext as MosaicApp
        runBlocking {
            app.holder.call { core ->
                val state = core.state()
                val existing =
                    when (state) {
                        is SessionState.Locked -> state.email
                        is SessionState.Unlocked -> state.email
                        else -> null
                    }
                if (existing != null) core.resetDevice(ResetConfirm(typedEmail = existing))
                val account = MosaicTestHooks(core).createLocalAccount(email, password.toByteArray())
                account.secretKey.wipe()
                val vault = core.vaults().first { it.available }.vaultId
                core.saveLogin(
                    LoginDraft(
                        vaultId = vault,
                        itemId = null,
                        title = "GitHub",
                        username = "octocat",
                        password = "hunter2".toByteArray(),
                        urls = listOf(UrlDraft("https://github.com", false, false)),
                        notes = null,
                        otp = OtpEdit.Keep,
                        favorite = false,
                    ),
                )
            }
        }
        app.lockNow()
    }

    @After
    fun tearDown() {
        runBlocking {
            app.holder.call { core ->
                val state = core.state()
                val existing =
                    when (state) {
                        is SessionState.Locked -> state.email
                        is SessionState.Unlocked -> state.email
                        else -> null
                    }
                if (existing != null) core.resetDevice(ResetConfirm(typedEmail = existing))
            }
        }
        if (setPin) DeviceLock.release()
    }

    private fun firstEditText(view: View): EditText? {
        if (view is EditText) return view
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                val found = firstEditText(view.getChildAt(index))
                if (found != null) return found
            }
        }
        return null
    }

    @Test
    fun passwordUnlockShowsVault() {
        val resources = app.resources
        val unlock = L10n.unlockAction(resources)
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            compose.waitUntil(30_000) { compose.onAllNodesWithText(unlock).fetchSemanticsNodes().isNotEmpty() }
            scenario.onActivity { activity ->
                val field = firstEditText(activity.window.decorView)
                checkNotNull(field) { "the unlock screen has no password field" }
                field.setText(password)
            }
            compose.onNodeWithText(unlock).performClick()
            // Argon2id runs here; the emulator is slow, so the wait is long.
            compose.waitUntil(120_000) { compose.onAllNodesWithText("GitHub").fetchSemanticsNodes().isNotEmpty() }
            compose.onNodeWithText(L10n.vaultTitle(resources)).assertExists()
        }
    }
}
