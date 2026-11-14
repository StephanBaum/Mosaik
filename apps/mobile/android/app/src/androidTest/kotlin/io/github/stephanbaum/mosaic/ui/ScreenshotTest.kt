package io.github.stephanbaum.mosaic.ui

import android.content.res.Configuration
import android.graphics.Bitmap
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import io.github.stephanbaum.mosaic.ffi.FieldView
import io.github.stephanbaum.mosaic.ffi.ItemDetail
import io.github.stephanbaum.mosaic.ffi.ItemKind
import io.github.stephanbaum.mosaic.ffi.ItemSummary
import io.github.stephanbaum.mosaic.ffi.MobileSettings
import io.github.stephanbaum.mosaic.ffi.OtpScheme
import io.github.stephanbaum.mosaic.ffi.OtpView
import io.github.stephanbaum.mosaic.ffi.UrlView
import io.github.stephanbaum.mosaic.security.SecretFieldState
import io.github.stephanbaum.mosaic.ui.settings.SettingsContent
import io.github.stephanbaum.mosaic.ui.vault.ItemDetailContent
import io.github.stephanbaum.mosaic.ui.vault.OtpDisplay
import io.github.stephanbaum.mosaic.ui.vault.VaultListContent
import java.io.File
import java.io.FileOutputStream
import java.util.Locale
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

private class Variant(val language: String, val dark: Boolean, val screen: Int)

/**
 * The main screens, drawn from sample data in light and dark, English and German, and written as PNG files to
 * the app's media folder for review. The screens are the stateless contents the app itself uses, hosted by the
 * Compose test activity, so no secure window blanks the capture and no real account is involved (the test
 * needs no change to a release build).
 */
@RunWith(AndroidJUnit4::class)
class ScreenshotTest {
    @get:Rule
    val compose = createComposeRule()

    private fun summary(id: String, title: String, subtitle: String?, favorite: Boolean, otp: Boolean) =
        ItemSummary(
            vaultId = "vault",
            itemId = id,
            kind = ItemKind.LOGIN,
            title = title,
            subtitle = subtitle,
            favorite = favorite,
            hasOtp = otp,
            hasPasskey = false,
            primaryHost = null,
            updatedAtMs = 0L,
            protected = false,
        )

    private val items =
        listOf(
            summary("1", "GitHub", "octocat", true, true),
            summary("2", "Mailbox", "me@example.com", false, false),
            summary("3", "Bank", null, false, false),
        )

    private val detail =
        ItemDetail(
            summary = items[0],
            fields =
                listOf(
                    FieldView("login.username", "field.login.username", null, false, "octocat", true),
                    FieldView("login.password", "field.login.password", null, true, null, true),
                    FieldView("notes", "field.notes", null, false, "Recovery codes are in the drawer.", true),
                ),
            urls = listOf(UrlView("https://github.com", false, false, false, false)),
            tags = listOf("work"),
            passkeys = emptyList(),
            attachments = emptyList(),
            otp = OtpView(OtpScheme.TOTP, 6u, 30u, "GitHub", "octocat"),
            historyCount = 0u,
        )

    private val settings = MobileSettings(0u, 5u, 30u, 14u, false, false)

    private var variant by mutableStateOf(Variant("en", false, 0))

    @Test
    fun mainScreensLightDarkEnDe() {
        val base = InstrumentationRegistry.getInstrumentation().targetContext
        val directory = File(base.getExternalFilesDir(null), "screenshots")
        assertTrue("cannot create ${directory.path}", directory.isDirectory || directory.mkdirs())
        val passwordState = SecretFieldState()

        compose.setContent {
            val current = variant
            val configuration = Configuration(base.resources.configuration)
            configuration.setLocale(Locale.forLanguageTag(current.language))
            val localized = base.createConfigurationContext(configuration)
            CompositionLocalProvider(LocalContext provides localized) {
                MosaicTheme(darkTheme = current.dark) {
                    Box(modifier = Modifier.size(360.dp, 720.dp)) {
                        when (current.screen) {
                            0 ->
                                VaultListContent(
                                    items = items,
                                    syncText = "",
                                    pendingUploads = 0,
                                    error = null,
                                    refreshing = false,
                                    onRefresh = {},
                                    onOpen = {},
                                    onAdd = {},
                                )
                            1 ->
                                ItemDetailContent(
                                    detail = detail,
                                    failure = null,
                                    revealed = mapOf("login.password" to "correct-horse-battery"),
                                    otp = OtpDisplay("123456", 17, 30),
                                    notice = null,
                                    onBack = {},
                                    onEdit = {},
                                    onLink = {},
                                    onToggleFavorite = {},
                                    onDelete = {},
                                    onReveal = {},
                                    onHide = {},
                                    onCopy = {},
                                    onCopyOtp = {},
                                )
                            else ->
                                SettingsContent(
                                    settings = settings,
                                    syncText = "",
                                    pendingUploads = 0,
                                    quickEnrolled = false,
                                    biometricAvailable = false,
                                    askPassword = false,
                                    passwordState = passwordState,
                                    version = "0.1.0",
                                    busy = false,
                                    error = null,
                                    onAutoLock = {},
                                    onIdle = {},
                                    onClipboard = {},
                                    onQuickStart = {},
                                    onQuickConfirm = {},
                                    onQuickCancel = {},
                                    onQuickDisable = {},
                                    onSync = {},
                                    onLock = {},
                                    onLicences = {},
                                )
                        }
                    }
                }
            }
        }

        var written = 0
        for (language in listOf("en", "de")) {
            for (dark in listOf(false, true)) {
                for (screen in 0..2) {
                    compose.runOnUiThread { variant = Variant(language, dark, screen) }
                    compose.waitForIdle()
                    val bitmap = compose.onRoot().captureToImage().asAndroidBitmap()
                    val pixels = IntArray(bitmap.width * bitmap.height)
                    bitmap.getPixels(pixels, 0, bitmap.width, 0, 0, bitmap.width, bitmap.height)
                    assertTrue("screen $screen ($language, dark=$dark) is blank", pixels.toSet().size > 8)
                    val name = "screen$screen-$language-${if (dark) "dark" else "light"}.png"
                    FileOutputStream(File(directory, name)).use { out -> bitmap.compress(Bitmap.CompressFormat.PNG, 100, out) }
                    written += 1
                }
            }
        }
        assertTrue("expected 12 screenshots", written == 12)
    }
}
