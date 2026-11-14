package io.github.stephanbaum.mosaic.ui.vault

import android.content.res.Resources
import android.text.format.DateUtils
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.ItemSummary
import io.github.stephanbaum.mosaic.ffi.SyncOverview
import io.github.stephanbaum.mosaic.ffi.SyncWarningKind
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.MosaicTheme
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.Line
import io.github.stephanbaum.mosaic.ui.components.SectionTitle
import io.github.stephanbaum.mosaic.ui.components.TopBar
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicRadius
import io.github.stephanbaum.mosaic.ui.theme.MosaicSize
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import io.github.stephanbaum.mosaic.ui.theme.MosaicType
import kotlinx.coroutines.launch

/** The sync state as one line: where the vault lives, the last error, or when it last synced. */
fun syncLine(r: Resources, overview: SyncOverview?, nowMs: Long): String {
    if (overview == null) return ""
    if (!overview.configured) return L10n.syncLocalOnly(r)
    val error = overview.lastError
    if (error != null) return errorText(r, error)
    val last = overview.lastSyncedAtMs ?: return L10n.syncNever(r)
    val relative = DateUtils.getRelativeTimeSpanString(last.toLong(), nowMs, DateUtils.MINUTE_IN_MILLIS).toString()
    return L10n.syncLast(r, relative)
}

/** The sync warnings as texts: what sync found wrong, never a secret (kinds, counts and times only). */
fun syncWarningTexts(r: Resources, overview: SyncOverview?): List<String> =
    overview?.warnings.orEmpty().map { warning ->
        when (warning.kind) {
            SyncWarningKind.FREEZE_SUSPECTED -> L10n.syncWarningFreezeSuspected(r)
            SyncWarningKind.NO_COMMON_VIEW -> L10n.syncWarningNoCommonView(r)
            SyncWarningKind.ROLLBACK_DETECTED -> L10n.syncWarningRollbackDetected(r)
            SyncWarningKind.IDENTITY_DUPLICATED -> L10n.syncWarningIdentityDuplicated(r)
            SyncWarningKind.SERVER_INCONSISTENT -> L10n.syncWarningServerInconsistent(r)
            SyncWarningKind.SERVER_ROLLBACK -> L10n.syncWarningServerRollback(r)
            SyncWarningKind.UNDECRYPTABLE -> L10n.syncWarningUndecryptable(r, (warning.count ?: 1u).toInt())
            SyncWarningKind.ACCOUNT_ROLLBACK -> L10n.syncWarningAccountRollback(r)
        }
    }.distinct()

/** The vault tab: loads the items, shows the sync state, and syncs on a pull. */
@Composable
fun VaultScreen(
    app: MosaicApp,
    version: Int,
    onOpen: (ItemSummary) -> Unit,
    onAdd: (login: Boolean) -> Unit,
) {
    val resources = LocalContext.current.resources
    val scope = rememberCoroutineScope()
    var items by remember { mutableStateOf<List<ItemSummary>?>(null) }
    var overview by remember { mutableStateOf<SyncOverview?>(null) }
    var failure by remember { mutableStateOf<String?>(null) }
    var refreshing by remember { mutableStateOf(false) }

    suspend fun load() {
        when (val result = app.vault.items()) {
            is Outcome.Ok -> {
                items = result.value
                failure = null
            }
            is Outcome.Failed -> failure = result.code
            Outcome.Cancelled -> Unit
        }
        val sync = app.vault.syncOverview()
        if (sync is Outcome.Ok) overview = sync.value
    }

    LaunchedEffect(version) { load() }

    VaultListContent(
        items = items,
        syncText = syncLine(resources, overview, System.currentTimeMillis()),
        pendingUploads = overview?.pendingUploads?.toInt() ?: 0,
        error = failure?.let { errorText(resources, it) },
        refreshing = refreshing,
        onRefresh = {
            scope.launch {
                refreshing = true
                when (val result = app.vault.syncNow()) {
                    is Outcome.Ok -> {
                        overview = result.value
                        failure = null
                    }
                    is Outcome.Failed -> if (result.code != "syncNotConfigured") failure = result.code
                    Outcome.Cancelled -> Unit
                }
                load()
                refreshing = false
            }
        },
        onOpen = onOpen,
        onAdd = onAdd,
    )
}

/** The stateless vault list (also drawn by the screenshot test). */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VaultListContent(
    items: List<ItemSummary>?,
    syncText: String,
    pendingUploads: Int,
    error: String?,
    refreshing: Boolean,
    onRefresh: () -> Unit,
    onOpen: (ItemSummary) -> Unit,
    onAdd: (login: Boolean) -> Unit,
) {
    val resources = LocalContext.current.resources
    Box(
        modifier = Modifier.fillMaxSize().background(MosaicTheme.colors.bg),
        contentAlignment = Alignment.TopCenter,
    ) {
        Column(
            modifier = Modifier.widthIn(max = MosaicSize.form).fillMaxWidth().padding(horizontal = MosaicSpace.s5),
            verticalArrangement = Arrangement.spacedBy(MosaicSpace.s3),
        ) {
            TopBar(title = L10n.vaultTitle(resources), onBack = null) {
                AddMenu(onAdd = onAdd)
            }
            if (items != null) HintText(L10n.vaultCount(resources, items.size))
            if (syncText.isNotEmpty()) HintText(syncText)
            if (pendingUploads > 0) HintText(L10n.syncPending(resources, pendingUploads))
            if (error != null) ErrorText(error)
            PullToRefreshBox(isRefreshing = refreshing, onRefresh = onRefresh, modifier = Modifier.weight(1f)) {
                LazyColumn(
                    modifier = Modifier.fillMaxSize(),
                    contentPadding = PaddingValues(bottom = MosaicSpace.s5),
                    verticalArrangement = Arrangement.spacedBy(MosaicSpace.s2),
                ) {
                    if (items != null && items.isEmpty()) {
                        item {
                            Column(
                                modifier = Modifier.fillMaxWidth().padding(top = MosaicSpace.s7),
                                verticalArrangement = Arrangement.spacedBy(MosaicSpace.s3),
                            ) {
                                SectionTitle(L10n.vaultEmptyTitle(resources))
                                Line(L10n.vaultEmptyBody(resources), muted = true)
                            }
                        }
                    } else if (items != null) {
                        items(items, key = { it.vaultId + "/" + it.itemId }) { summary ->
                            ItemRow(summary = summary, onClick = { onOpen(summary) })
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun AddMenu(onAdd: (login: Boolean) -> Unit) {
    val resources = LocalContext.current.resources
    var open by remember { mutableStateOf(false) }
    Box {
        TextButton(onClick = { open = true }, modifier = Modifier.heightIn(min = MosaicSpace.s8)) {
            Text(L10n.vaultAdd(resources))
        }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            DropdownMenuItem(
                text = { Text(L10n.vaultAddLogin(resources)) },
                onClick = {
                    open = false
                    onAdd(true)
                },
            )
            DropdownMenuItem(
                text = { Text(L10n.vaultAddNote(resources)) },
                onClick = {
                    open = false
                    onAdd(false)
                },
            )
        }
    }
}

/** One item: an initial in a tile, the title, a second line, and a favorite mark. */
@Composable
fun ItemRow(summary: ItemSummary, onClick: () -> Unit) {
    val resources = LocalContext.current.resources
    val title = summary.title.ifEmpty { L10n.vaultUntitled(resources) }
    val second = summary.subtitle ?: summary.primaryHost ?: kindLabel(resources, summary.kind)
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = MosaicSpace.s9)
                .clickable(role = Role.Button, onClick = onClick)
                .padding(vertical = MosaicSpace.s3),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s4),
    ) {
        Box(
            modifier =
                Modifier
                    .size(MosaicSize.controlLg)
                    .background(MosaicTheme.colors.bgSelected, RoundedCornerShape(MosaicRadius.field)),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = title.take(1).uppercase(),
                color = MosaicTheme.colors.text,
                fontFamily = MosaicTheme.fonts.ui,
                fontWeight = FontWeight.SemiBold,
                fontSize = MosaicType.md.size,
                textAlign = TextAlign.Center,
            )
        }
        Column(modifier = Modifier.weight(1f)) {
            Line(title, maxLines = 1)
            HintText(second)
        }
        if (summary.favorite) {
            val label = L10n.vaultFavorite(resources)
            Text(
                text = "★",
                color = MosaicTheme.colors.warning,
                modifier = Modifier.semantics { contentDescription = label },
            )
        }
    }
}
