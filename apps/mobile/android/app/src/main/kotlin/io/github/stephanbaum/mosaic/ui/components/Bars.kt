package io.github.stephanbaum.mosaic.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import io.github.stephanbaum.mosaic.ui.MosaicTheme
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicBorderWidth
import io.github.stephanbaum.mosaic.ui.theme.MosaicRadius
import io.github.stephanbaum.mosaic.ui.theme.MosaicSize
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import io.github.stephanbaum.mosaic.ui.theme.MosaicType

/**
 * One page inside the signed-in shell: token background and one column no wider than the form width. The shell
 * applies the safe-area padding, so the page does not.
 */
@Composable
fun PageColumn(scroll: Boolean = true, content: @Composable ColumnScope.() -> Unit) {
    val scrollState = rememberScrollState()
    Box(
        modifier = Modifier.fillMaxSize().background(MosaicTheme.colors.bg),
        contentAlignment = Alignment.TopCenter,
    ) {
        val base = Modifier.widthIn(max = MosaicSize.form).fillMaxWidth()
        Column(
            modifier = (if (scroll) base.verticalScroll(scrollState) else base).padding(MosaicSpace.s5),
            verticalArrangement = Arrangement.spacedBy(MosaicSpace.s4),
            content = content,
        )
    }
}

/** The title row of a page: an optional back button, the title, and actions on the right. */
@Composable
fun TopBar(title: String, onBack: (() -> Unit)?, actions: @Composable RowScope.() -> Unit = {}) {
    val resources = LocalContext.current.resources
    Row(
        modifier = Modifier.fillMaxWidth().heightIn(min = MosaicSpace.s8),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s3),
    ) {
        if (onBack != null) {
            TextButton(onClick = onBack, modifier = Modifier.heightIn(min = MosaicSpace.s8)) {
                Text(L10n.commonBack(resources))
            }
        }
        Text(
            text = title,
            color = MosaicTheme.colors.text,
            fontFamily = MosaicTheme.fonts.ui,
            fontWeight = FontWeight.SemiBold,
            fontSize = MosaicType.lg.size,
            lineHeight = MosaicType.lg.lineHeight,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f).semantics { heading() },
        )
        actions()
    }
}

/** A section heading. */
@Composable
fun SectionTitle(text: String) {
    Text(
        text = text,
        color = MosaicTheme.colors.text,
        fontFamily = MosaicTheme.fonts.ui,
        fontWeight = FontWeight.SemiBold,
        fontSize = MosaicType.md.size,
        lineHeight = MosaicType.md.lineHeight,
        modifier = Modifier.semantics { heading() },
    )
}

/** A line of text in the UI font: [muted] for secondary lines, [mono] for secrets and codes. */
@Composable
fun Line(
    text: String,
    modifier: Modifier = Modifier,
    muted: Boolean = false,
    mono: Boolean = false,
    maxLines: Int = Int.MAX_VALUE,
) {
    val family: FontFamily = if (mono) MosaicTheme.fonts.mono else MosaicTheme.fonts.ui
    Text(
        text = text,
        color = if (muted) MosaicTheme.colors.textMuted else MosaicTheme.colors.text,
        fontFamily = family,
        fontSize = MosaicType.md.size,
        lineHeight = MosaicType.md.lineHeight,
        maxLines = maxLines,
        overflow = TextOverflow.Ellipsis,
        modifier = modifier,
    )
}

/** A bordered surface panel. */
@Composable
fun Panel(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .background(MosaicTheme.colors.surface, RoundedCornerShape(MosaicRadius.card))
                .border(MosaicBorderWidth.hairline, MosaicTheme.colors.border, RoundedCornerShape(MosaicRadius.card))
                .padding(MosaicSpace.s5),
        verticalArrangement = Arrangement.spacedBy(MosaicSpace.s4),
        content = content,
    )
}

/** One option of a single-choice group, as one 48 dp target. */
@Composable
fun ChoiceRow(selected: Boolean, label: String, onSelect: () -> Unit, enabled: Boolean = true) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = MosaicSpace.s8)
                .selectable(selected = selected, enabled = enabled, role = Role.RadioButton, onClick = onSelect),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(MosaicSpace.s4),
    ) {
        RadioButton(selected = selected, onClick = null, enabled = enabled)
        Text(
            text = label,
            color = MosaicTheme.colors.text,
            fontFamily = MosaicTheme.fonts.ui,
            fontSize = MosaicType.md.size,
            lineHeight = MosaicType.md.lineHeight,
        )
    }
}

/**
 * A short text button (Show, Hide, Copy) that is at least 48 dp square. [description] is what a screen reader
 * says ("Copy Password"), because the visible word alone does not name the field.
 */
@Composable
fun ActionButton(text: String, description: String, onClick: () -> Unit, enabled: Boolean = true) {
    TextButton(
        onClick = onClick,
        enabled = enabled,
        modifier =
            Modifier
                .heightIn(min = MosaicSpace.s8)
                .widthIn(min = MosaicSpace.s8)
                .semantics { contentDescription = description },
    ) {
        Text(text)
    }
}

/** A short confirmation that screen readers announce when it appears ("Copied. Clears in 30 seconds."). */
@Composable
fun Notice(text: String) {
    Text(
        text = text,
        color = MosaicTheme.colors.text,
        fontFamily = MosaicTheme.fonts.ui,
        fontSize = MosaicType.sm.size,
        lineHeight = MosaicType.sm.lineHeight,
        modifier =
            Modifier
                .fillMaxWidth()
                .background(MosaicTheme.colors.successSubtle, RoundedCornerShape(MosaicRadius.control))
                .padding(MosaicSpace.s4)
                .semantics { liveRegion = LiveRegionMode.Polite },
    )
}
