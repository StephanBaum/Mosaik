package io.github.stephanbaum.mosaic.ui

import android.content.res.Resources
import io.github.stephanbaum.mosaic.ui.l10n.L10n

/** The text of a core error code (`error.<code>`), or the generic internal-error text for an unknown code. */
fun errorText(resources: Resources, code: String): String =
    L10n.errorMessage(resources, code) ?: L10n.errorInternal(resources)
