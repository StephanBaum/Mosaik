package io.github.stephanbaum.mosaic.ui.link

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.Line
import io.github.stephanbaum.mosaic.ui.components.Panel
import io.github.stephanbaum.mosaic.ui.l10n.L10n

/** Who the app is, for a link decision: label, package, installer and the start of the signing certificate. */
@Composable
fun AppIdentityPanel(label: String, packageName: String, installer: String?, fingerprint: String) {
    val resources = LocalContext.current.resources
    Panel {
        HintText(L10n.linkApp(resources))
        Line(label)
        HintText(L10n.linkPackage(resources))
        Line(packageName, mono = true)
        HintText(L10n.linkInstaller(resources))
        Line(installer ?: L10n.linkInstallerUnknown(resources))
        HintText(L10n.linkCertificate(resources))
        Line(fingerprint, mono = true)
    }
}
