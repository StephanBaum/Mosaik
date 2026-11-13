package io.github.stephanbaum.mosaic.security

import io.github.stephanbaum.mosaic.ffi.MobileException
import io.github.stephanbaum.mosaic.ui.l10n.L10n

/** Maps a core error to its camelCase code, the `error.<code>` key of the catalog (docs/MOBILE.md, errors). */
object ErrorCodes {
    /** The code of [error], or `internal` for anything that is not a known core error. */
    fun of(error: Throwable): String {
        if (error !is MobileException) return INTERNAL
        // The message of a flat error is its Display text, which is `MobileError::code()`.
        val message = error.message
        if (message != null && message in L10n.errorCodes) return message
        return fromVariantName(error.javaClass.simpleName)
    }

    /** `WrongCredentials` becomes `wrongCredentials`; an unknown name becomes `internal`. */
    fun fromVariantName(name: String): String {
        val code = name.replaceFirstChar { it.lowercaseChar() }
        return if (code in L10n.errorCodes) code else INTERNAL
    }

    const val INTERNAL = "internal"
}
