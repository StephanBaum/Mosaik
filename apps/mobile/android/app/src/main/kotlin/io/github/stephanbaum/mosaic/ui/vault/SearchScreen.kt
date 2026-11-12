package io.github.stephanbaum.mosaic.ui.vault

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.ImeAction
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.ItemSummary
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.PageColumn
import io.github.stephanbaum.mosaic.ui.components.TextInput
import io.github.stephanbaum.mosaic.ui.components.TopBar
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import kotlinx.coroutines.delay

private const val DEBOUNCE_MS = 150L

/** The search tab: the core searches titles, user names, URLs and tags; the query stays on this screen. */
@Composable
fun SearchScreen(app: MosaicApp, version: Int, onOpen: (ItemSummary) -> Unit) {
    val resources = LocalContext.current.resources
    var query by rememberSaveable { mutableStateOf("") }
    var results by remember { mutableStateOf<List<ItemSummary>>(emptyList()) }
    var failure by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(query, version) {
        if (query.isBlank()) {
            results = emptyList()
            failure = null
            return@LaunchedEffect
        }
        delay(DEBOUNCE_MS)
        when (val outcome = app.vault.search(query.trim())) {
            is Outcome.Ok -> {
                results = outcome.value
                failure = null
            }
            is Outcome.Failed -> failure = outcome.code
            Outcome.Cancelled -> Unit
        }
    }

    PageColumn(scroll = false) {
        TopBar(title = L10n.navSearch(resources), onBack = null)
        TextInput(
            value = query,
            onValueChange = { query = it },
            label = L10n.searchPlaceholder(resources),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
        )
        val error = failure
        if (error != null) ErrorText(errorText(resources, error))
        if (query.isBlank()) {
            HintText(L10n.searchHint(resources))
        } else if (results.isEmpty() && error == null) {
            HintText(L10n.searchNone(resources, query.trim()))
        }
        LazyColumn(
            modifier = Modifier.weight(1f),
            contentPadding = PaddingValues(bottom = MosaicSpace.s5),
            verticalArrangement = Arrangement.spacedBy(MosaicSpace.s2),
        ) {
            items(results, key = { it.vaultId + "/" + it.itemId }) { summary ->
                ItemRow(summary = summary, onClick = { onOpen(summary) })
            }
        }
    }
}
