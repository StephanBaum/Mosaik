package io.github.stephanbaum.mosaic.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import io.github.stephanbaum.mosaic.ui.MosaicTheme
import io.github.stephanbaum.mosaic.ui.theme.MosaicRadius
import io.github.stephanbaum.mosaic.ui.theme.MosaicSize
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import io.github.stephanbaum.mosaic.ui.theme.MosaicType

/** One screen: token background, safe-area padding, one scrolling column no wider than the form width. */
@Composable
fun ScreenFrame(content: @Composable ColumnScope.() -> Unit) {
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .background(MosaicTheme.colors.bg)
                .safeDrawingPadding(),
        contentAlignment = Alignment.TopCenter,
    ) {
        Column(
            modifier =
                Modifier
                    .widthIn(max = MosaicSize.column)
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .padding(MosaicSpace.s6),
            verticalArrangement = Arrangement.spacedBy(MosaicSpace.s5),
            content = content,
        )
    }
}

@Composable
fun TitleText(text: String) {
    Text(
        text = text,
        color = MosaicTheme.colors.text,
        fontFamily = MosaicTheme.fonts.ui,
        fontWeight = FontWeight.SemiBold,
        fontSize = MosaicType.xl.size,
        lineHeight = MosaicType.xl.lineHeight,
        modifier = Modifier.semantics { heading() },
    )
}

@Composable
fun BodyText(text: String) {
    Text(
        text = text,
        color = MosaicTheme.colors.textSecondary,
        fontFamily = MosaicTheme.fonts.ui,
        fontSize = MosaicType.md.size,
        lineHeight = MosaicType.md.lineHeight,
    )
}

@Composable
fun HintText(text: String) {
    Text(
        text = text,
        color = MosaicTheme.colors.textMuted,
        fontFamily = MosaicTheme.fonts.ui,
        fontSize = MosaicType.sm.size,
        lineHeight = MosaicType.sm.lineHeight,
    )
}

/** An error text; announced politely to screen readers when it appears. */
@Composable
fun ErrorText(text: String) {
    Text(
        text = text,
        color = MosaicTheme.colors.danger,
        fontFamily = MosaicTheme.fonts.ui,
        fontSize = MosaicType.sm.size,
        lineHeight = MosaicType.sm.lineHeight,
        modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite },
    )
}

@Composable
fun PrimaryButton(text: String, onClick: () -> Unit, enabled: Boolean = true) {
    Button(
        onClick = onClick,
        enabled = enabled,
        shape = RoundedCornerShape(MosaicRadius.control),
        modifier = Modifier.fillMaxWidth().heightIn(min = MosaicSpace.s8),
    ) {
        Text(text)
    }
}

@Composable
fun SecondaryButton(text: String, onClick: () -> Unit, enabled: Boolean = true) {
    OutlinedButton(
        onClick = onClick,
        enabled = enabled,
        shape = RoundedCornerShape(MosaicRadius.control),
        modifier = Modifier.fillMaxWidth().heightIn(min = MosaicSpace.s8),
    ) {
        Text(text)
    }
}

@Composable
fun QuietButton(text: String, onClick: () -> Unit, enabled: Boolean = true) {
    TextButton(
        onClick = onClick,
        enabled = enabled,
        modifier = Modifier.fillMaxWidth().heightIn(min = MosaicSpace.s8),
    ) {
        Text(text)
    }
}

/** A plain (non-secret) text field: server address, email, device name. */
@Composable
fun TextInput(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    enabled: Boolean = true,
    placeholder: String? = null,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label) },
        placeholder = placeholder?.let { hint -> { Text(hint) } },
        singleLine = true,
        enabled = enabled,
        keyboardOptions = keyboardOptions,
        modifier = Modifier.fillMaxWidth().heightIn(min = MosaicSize.inputXl),
    )
}

/** The label above a secret field. */
@Composable
fun FieldLabel(text: String) {
    Text(
        text = text,
        color = MosaicTheme.colors.text,
        fontFamily = MosaicTheme.fonts.ui,
        fontWeight = FontWeight.Medium,
        fontSize = MosaicType.sm.size,
        lineHeight = MosaicType.sm.lineHeight,
    )
}

/** A checkbox with its label as one 48 dp target. */
@Composable
fun CheckRow(checked: Boolean, onCheckedChange: (Boolean) -> Unit, label: String, enabled: Boolean = true) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = MosaicSpace.s8)
                .toggleable(value = checked, enabled = enabled, role = Role.Checkbox, onValueChange = onCheckedChange),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s4),
    ) {
        Checkbox(checked = checked, onCheckedChange = null, enabled = enabled)
        Text(
            text = label,
            color = MosaicTheme.colors.text,
            fontFamily = MosaicTheme.fonts.ui,
            fontSize = MosaicType.md.size,
            lineHeight = MosaicType.md.lineHeight,
        )
    }
}

/** A spinner with a text, for the Argon2id and network waits. */
@Composable
fun Working(text: String) {
    Row(
        modifier = Modifier.fillMaxWidth().semantics { liveRegion = LiveRegionMode.Polite },
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s4),
    ) {
        CircularProgressIndicator()
        HintText(text)
    }
}
