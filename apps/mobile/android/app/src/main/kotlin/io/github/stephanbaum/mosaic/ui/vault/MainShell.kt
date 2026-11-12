package io.github.stephanbaum.mosaic.ui.vault

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.ItemKind
import io.github.stephanbaum.mosaic.ui.MosaicTheme
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.link.LinkAppScreen
import io.github.stephanbaum.mosaic.ui.settings.LicencesScreen
import io.github.stephanbaum.mosaic.ui.settings.SettingsScreen
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace

/** The three tabs of the bottom navigation (UX_GUIDELINES section 3). */
enum class Tab { Vault, Search, Settings }

/** The screens above the tabs. Only identifiers travel here, never a value. */
sealed interface Screen {
    class Detail(val vaultId: String, val itemId: String) : Screen

    /** [vaultId] and [itemId] are null for a new item; the editor picks the vault. */
    class EditLogin(val vaultId: String?, val itemId: String?) : Screen

    class EditNote(val vaultId: String?, val itemId: String?) : Screen

    class Link(val vaultId: String, val itemId: String, val initialPackage: String?) : Screen

    data object Licences : Screen
}

/**
 * The signed-in app: bottom navigation over Vault, Search and Settings, and a back stack of full screens above
 * them (item detail, editors, link app, licences). The stack lives in memory only; a lock replaces the whole
 * shell, which wipes every revealed value on its way out.
 */
@Composable
fun MainShell(app: MosaicApp, quickUnlockEnrolled: Boolean, initialLink: Screen.Link? = null) {
    val resources = LocalContext.current.resources
    var tab by remember { mutableStateOf(Tab.Vault) }
    var version by remember { mutableIntStateOf(0) }
    val stack = remember { mutableStateListOf<Screen>().also { if (initialLink != null) it.add(initialLink) } }
    val top: Screen? = stack.lastOrNull()

    BackHandler(enabled = stack.isNotEmpty()) { stack.removeAt(stack.lastIndex) }

    fun pop() {
        if (stack.isNotEmpty()) stack.removeAt(stack.lastIndex)
    }

    Column(modifier = Modifier.fillMaxSize().background(MosaicTheme.colors.bg).safeDrawingPadding()) {
        Box(modifier = Modifier.weight(1f)) {
            when (top) {
                null ->
                    when (tab) {
                        Tab.Vault ->
                            VaultScreen(
                                app = app,
                                version = version,
                                onOpen = { stack.add(Screen.Detail(it.vaultId, it.itemId)) },
                                onAdd = { login -> stack.add(if (login) Screen.EditLogin(null, null) else Screen.EditNote(null, null)) },
                            )
                        Tab.Search ->
                            SearchScreen(
                                app = app,
                                version = version,
                                onOpen = { stack.add(Screen.Detail(it.vaultId, it.itemId)) },
                            )
                        Tab.Settings ->
                            SettingsScreen(
                                app = app,
                                quickEnrolled = quickUnlockEnrolled,
                                onLicences = { stack.add(Screen.Licences) },
                            )
                    }
                is Screen.Detail ->
                    ItemDetailScreen(
                        app = app,
                        vaultId = top.vaultId,
                        itemId = top.itemId,
                        version = version,
                        onBack = { pop() },
                        onEdit = { kind ->
                            stack.add(
                                if (kind == ItemKind.SECURE_NOTE) {
                                    Screen.EditNote(top.vaultId, top.itemId)
                                } else {
                                    Screen.EditLogin(top.vaultId, top.itemId)
                                },
                            )
                        },
                        onLink = { stack.add(Screen.Link(top.vaultId, top.itemId, null)) },
                        onChanged = { version += 1 },
                    )
                is Screen.EditLogin ->
                    LoginEditorScreen(
                        app = app,
                        vaultId = top.vaultId,
                        itemId = top.itemId,
                        onSaved = { key ->
                            version += 1
                            pop()
                            if (top.itemId == null) stack.add(Screen.Detail(key.vaultId, key.itemId))
                        },
                        onCancel = { pop() },
                    )
                is Screen.EditNote ->
                    NoteEditorScreen(
                        app = app,
                        vaultId = top.vaultId,
                        itemId = top.itemId,
                        onSaved = { key ->
                            version += 1
                            pop()
                            if (top.itemId == null) stack.add(Screen.Detail(key.vaultId, key.itemId))
                        },
                        onCancel = { pop() },
                    )
                is Screen.Link ->
                    LinkAppScreen(
                        app = app,
                        vaultId = top.vaultId,
                        itemId = top.itemId,
                        initialPackage = top.initialPackage,
                        onDone = {
                            version += 1
                            pop()
                        },
                        onCancel = { pop() },
                    )
                Screen.Licences -> LicencesScreen(onBack = { pop() })
            }
        }
        if (top == null) {
            NavigationBar(
                containerColor = MosaicTheme.colors.surface,
                windowInsets = WindowInsets(0, 0, 0, 0),
            ) {
                for (entry in Tab.entries) {
                    NavigationBarItem(
                        selected = tab == entry,
                        onClick = { tab = entry },
                        icon = { TabIcon(entry, MosaicTheme.colors.text) },
                        label = {
                            Text(
                                when (entry) {
                                    Tab.Vault -> L10n.navVault(resources)
                                    Tab.Search -> L10n.navSearch(resources)
                                    Tab.Settings -> L10n.navSettings(resources)
                                },
                            )
                        },
                    )
                }
            }
        }
    }
}

/** Small line icons drawn on a canvas, so the app needs no icon library. */
@Composable
private fun TabIcon(tab: Tab, color: Color) {
    Canvas(modifier = Modifier.size(MosaicSpace.s6)) {
        val w = size.width
        val stroke = Stroke(width = MosaicSpace.s1.toPx(), cap = StrokeCap.Round)
        when (tab) {
            Tab.Vault -> {
                drawRoundRect(
                    color = color,
                    topLeft = Offset(w * 0.12f, w * 0.2f),
                    size = Size(w * 0.76f, w * 0.6f),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(w * 0.12f),
                    style = stroke,
                )
                drawCircle(color = color, radius = w * 0.1f, center = Offset(w * 0.5f, w * 0.5f), style = stroke)
            }
            Tab.Search -> {
                drawCircle(color = color, radius = w * 0.26f, center = Offset(w * 0.44f, w * 0.44f), style = stroke)
                drawLine(color, Offset(w * 0.64f, w * 0.64f), Offset(w * 0.86f, w * 0.86f), strokeWidth = stroke.width, cap = StrokeCap.Round)
            }
            Tab.Settings -> {
                for (row in 0..2) {
                    val y = w * (0.25f + 0.25f * row)
                    drawLine(color, Offset(w * 0.12f, y), Offset(w * 0.88f, y), strokeWidth = stroke.width, cap = StrokeCap.Round)
                }
            }
        }
    }
}
