package io.github.stephanbaum.mosaic.autofill

import android.content.pm.Signature
import android.content.pm.SigningInfo
import java.security.MessageDigest

/**
 * Which signing certificates identify a caller (ADR-0092, amendment G1-9). Kotlin chooses the **current** signers
 * and hands them to the core, which refuses more than one: all of `apkContentsSigners` when the app has several
 * signers, otherwise only the newest entry of `signingCertificateHistory`, never an old certificate of the history.
 */
object Signers {
    /** The pure choice: [contents] when the app has [multiple] signers, otherwise the last of [history]. */
    fun choose(multiple: Boolean, contents: List<ByteArray>, history: List<ByteArray>): List<ByteArray> =
        if (multiple) contents else listOfNotNull(history.lastOrNull())

    /** SHA-256 of a certificate. */
    fun digest(certificate: ByteArray): ByteArray = MessageDigest.getInstance("SHA-256").digest(certificate)

    /** The SHA-256 digests of the current signers of [signing]; empty when there is no signing information. */
    fun of(signing: SigningInfo?): List<ByteArray> {
        if (signing == null) return emptyList()
        val contents: Array<Signature>? = signing.apkContentsSigners
        val history: Array<Signature>? = signing.signingCertificateHistory
        return choose(
            multiple = signing.hasMultipleSigners(),
            contents = contents.orEmpty().map { it.toByteArray() },
            history = history.orEmpty().map { it.toByteArray() },
        ).map { digest(it) }
    }
}
