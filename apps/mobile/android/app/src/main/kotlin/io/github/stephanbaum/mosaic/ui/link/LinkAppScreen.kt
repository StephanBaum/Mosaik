package io.github.stephanbaum.mosaic.ui.link

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.Role
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.AssetLinksStatus
import io.github.stephanbaum.mosaic.ffi.ItemDetail
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.Line
import io.github.stephanbaum.mosaic.ui.components.Notice
import io.github.stephanbaum.mosaic.ui.components.PageColumn
import io.github.stephanbaum.mosaic.ui.components.Panel
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.TextInput
import io.github.stephanbaum.mosaic.ui.components.TopBar
import io.github.stephanbaum.mosaic.ui.components.Working
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Links an Android app to a login (docs/MOBILE.md, ADR-0092): the user picks the app (or arrives with one from
 * the "No login for this app" dataset), sees who it is and whether the website vouches for it, and confirms.
 * The link is written on the login itself; there is no "fill anyway".
 */
@Composable
fun LinkAppScreen(
    app: MosaicApp,
    vaultId: String,
    itemId: String,
    initialPackage: String?,
    onDone: () -> Unit,
    onCancel: () -> Unit,
) {
    val context = LocalContext.current
    val resources = context.resources
    val scope = rememberCoroutineScope()
    var login by remember { mutableStateOf<ItemDetail?>(null) }
    var choices by remember { mutableStateOf<List<AppChoice>?>(null) }
    var filter by rememberSaveable { mutableStateOf("") }
    var selected by rememberSaveable { mutableStateOf(initialPackage) }
    var details by remember { mutableStateOf<AppDetails?>(null) }
    var status by remember { mutableStateOf<AssetLinksStatus?>(null) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var linked by remember { mutableStateOf(false) }

    LaunchedEffect(vaultId, itemId) {
        when (val result = app.vault.item(vaultId, itemId)) {
            is Outcome.Ok -> login = result.value
            is Outcome.Failed -> errorCode = result.code
            Outcome.Cancelled -> Unit
        }
    }
    LaunchedEffect(selected == null) {
        if (selected == null && choices == null) {
            choices = withContext(Dispatchers.IO) { launchableApps(context) }
        }
    }
    LaunchedEffect(selected) {
        val pkg = selected
        details = null
        status = null
        if (pkg == null) return@LaunchedEffect
        val found = withContext(Dispatchers.IO) { appDetails(context, pkg) }
        details = found
        if (found == null) {
            errorCode = "notFound"
            return@LaunchedEffect
        }
        when (val result = app.vault.assetLinksStatus(vaultId, itemId, found.caller)) {
            is Outcome.Ok -> status = result.value
            is Outcome.Failed -> status = AssetLinksStatus.UNAVAILABLE
            Outcome.Cancelled -> Unit
        }
    }

    val loginTitle = login?.summary?.title.orEmpty()
    val host = login?.summary?.primaryHost
    val chosen = details
    val failure = errorCode

    if (chosen == null) {
        PageColumn(scroll = false) {
            TopBar(title = L10n.linkTitle(resources), onBack = onCancel)
            BodyText(L10n.linkChoose(resources, loginTitle))
            if (failure != null) ErrorText(errorText(resources, failure))
            TextInput(value = filter, onValueChange = { filter = it }, label = L10n.linkFilter(resources))
            val all = choices
            if (all == null) {
                Working(L10n.commonWorking(resources))
            } else {
                val shown = all.filter { filter.isBlank() || it.label.contains(filter.trim(), ignoreCase = true) }
                if (shown.isEmpty()) HintText(L10n.linkNoApps(resources))
                LazyColumn(
                    modifier = Modifier.weight(1f),
                    contentPadding = PaddingValues(bottom = MosaicSpace.s5),
                ) {
                    items(shown, key = { it.packageName }) { choice ->
                        Column(
                            modifier =
                                Modifier
                                    .fillMaxWidth()
                                    .heightIn(min = MosaicSpace.s9)
                                    .clickable(role = Role.Button, onClick = { selected = choice.packageName })
                                    .padding(vertical = MosaicSpace.s3),
                        ) {
                            Line(choice.label, maxLines = 1)
                            HintText(choice.packageName)
                        }
                    }
                }
            }
        }
    } else {
        PageColumn {
            TopBar(
                title = L10n.linkTitle(resources),
                onBack = {
                    if (initialPackage == null) selected = null else onCancel()
                },
            )
            Panel {
                HintText(L10n.linkApp(resources))
                Line(chosen.label)
                HintText(L10n.linkPackage(resources))
                Line(chosen.packageName, mono = true)
                HintText(L10n.linkInstaller(resources))
                Line(chosen.installer ?: L10n.linkInstallerUnknown(resources))
                HintText(L10n.linkCertificate(resources))
                Line(chosen.certificateHex, mono = true)
            }
            val current = status
            val statusText =
                when (current) {
                    null -> L10n.linkChecking(resources)
                    AssetLinksStatus.VERIFIED -> L10n.linkVerified(resources, host.orEmpty())
                    AssetLinksStatus.NOT_LISTED -> L10n.linkNotVerified(resources)
                    AssetLinksStatus.UNAVAILABLE -> L10n.linkUnavailable(resources)
                }
            Line(statusText)
            if (current != null && current != AssetLinksStatus.VERIFIED) {
                BodyText(if (host != null) L10n.linkWarning(resources, host) else L10n.linkWarningNoHost(resources))
            }
            if (failure != null) ErrorText(errorText(resources, failure))
            if (linked) {
                Notice(L10n.linkDone(resources, loginTitle))
                PrimaryButton(L10n.commonDone(resources), onClick = onDone)
            } else if (busy) {
                Working(L10n.commonWorking(resources))
            } else {
                PrimaryButton(
                    L10n.linkConfirm(resources, loginTitle),
                    enabled = current != null && chosen.certificateHex.isNotEmpty(),
                    onClick = {
                        scope.launch {
                            busy = true
                            errorCode = null
                            when (val result = app.vault.linkApp(vaultId, itemId, chosen.caller)) {
                                is Outcome.Ok -> linked = true
                                is Outcome.Failed -> errorCode = result.code
                                Outcome.Cancelled -> Unit
                            }
                            busy = false
                        }
                    },
                )
            }
        }
    }
}
