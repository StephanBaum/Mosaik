package io.github.stephanbaum.mosaic.ui

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import io.github.stephanbaum.mosaic.ffi.coreVersion
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import io.github.stephanbaum.mosaic.ui.theme.MosaicType
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** The skeleton screen: the app name and the version of the Rust core (proves the native library loads). */
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            MosaicTheme {
                CoreVersionScreen()
            }
        }
    }
}

@Composable
private fun CoreVersionScreen() {
    val resources = LocalContext.current.resources
    // Core calls block: never on the main thread.
    val version by produceState<String?>(initialValue = null) {
        value = withContext(Dispatchers.IO) { coreVersion() }
    }
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MosaicTheme.colors.bg)
                .safeDrawingPadding()
                .padding(MosaicSpace.s6),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = L10n.appName(resources),
            color = MosaicTheme.colors.text,
            fontSize = MosaicType.xxl.size,
            lineHeight = MosaicType.xxl.lineHeight,
        )
        version?.let {
            Text(
                text = L10n.settingsVersion(resources, it),
                color = MosaicTheme.colors.textMuted,
                fontSize = MosaicType.sm.size,
                lineHeight = MosaicType.sm.lineHeight,
                modifier = Modifier.padding(top = 8.dp),
            )
        }
    }
}
