package io.github.stephanbaum.mosaic.ui.link

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.autofill.CallerReader
import io.github.stephanbaum.mosaic.ffi.ItemKind
import io.github.stephanbaum.mosaic.ffi.ItemSummary
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.PageColumn
import io.github.stephanbaum.mosaic.ui.components.TextInput
import io.github.stephanbaum.mosaic.ui.components.TopBar
import io.github.stephanbaum.mosaic.ui.components.Working
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import io.github.stephanbaum.mosaic.ui.vault.ItemRow

/**
 * The first step of linking from the "No login for this app" entry (ADR-0092): the user picks the login. The next
 * screen shows who the app is and what the website says, and the user confirms there. Only logins are listed.
 */
@Composable
fun PickLoginScreen(
    packageName: String,
    app: MosaicApp,
    onPick: (ItemSummary) -> Unit,
    onCancel: () -> Unit,
) {
    val context = LocalContext.current
    val resources = context.resources
    var filter by rememberSaveable { mutableStateOf("") }
    var logins by remember { mutableStateOf<List<ItemSummary>?>(null) }
    var failure by remember { mutableStateOf<String?>(null) }
    val label = remember(packageName) { CallerReader.labelOf(context, packageName) }

    LaunchedEffect(Unit) {
        when (val outcome = app.vault.items()) {
            is Outcome.Ok -> logins = outcome.value.filter { it.kind == ItemKind.LOGIN }
            is Outcome.Failed -> failure = outcome.code
            Outcome.Cancelled -> Unit
        }
    }

    PageColumn(scroll = false) {
        TopBar(title = L10n.linkTitle(resources), onBack = onCancel)
        BodyText(L10n.linkPickLogin(resources, label))
        TextInput(value = filter, onValueChange = { filter = it }, label = L10n.searchPlaceholder(resources))
        val error = failure
        if (error != null) ErrorText(errorText(resources, error))
        val all = logins
        if (all == null) {
            if (error == null) Working(L10n.commonWorking(resources))
        } else {
            val shown =
                all.filter {
                    filter.isBlank() ||
                        it.title.contains(filter.trim(), ignoreCase = true) ||
                        (it.subtitle?.contains(filter.trim(), ignoreCase = true) ?: false)
                }
            if (shown.isEmpty()) HintText(L10n.searchNone(resources, filter.trim()))
            LazyColumn(
                modifier = Modifier.weight(1f),
                contentPadding = PaddingValues(bottom = MosaicSpace.s5),
                verticalArrangement = Arrangement.spacedBy(MosaicSpace.s2),
            ) {
                items(shown, key = { it.vaultId + "/" + it.itemId }) { summary ->
                    ItemRow(summary = summary, onClick = { onPick(summary) })
                }
            }
        }
    }
}
