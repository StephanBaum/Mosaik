package io.github.stephanbaum.mosaic.security

import io.github.stephanbaum.mosaic.ffi.SecretValue
import java.nio.ByteBuffer

/**
 * Where a secret has to become text. A revealed value is drawn by a `Text` and the clipboard takes a
 * `CharSequence`, so these two exits make a short-lived copy and zero every buffer they own. The `String` made
 * for display cannot be wiped (CRYPTO_SPEC 6.9.5); the screens drop it on hide, background and lock.
 */
object SecretText {
    /** The value as a `String` for display. The temporary byte copy is zeroed. */
    fun display(secret: SecretValue): String {
        val bytes = secret.exposeBytes()
        try {
            return String(bytes, Charsets.UTF_8)
        } finally {
            bytes.fill(0)
        }
    }

    /** Copies the value to the clipboard through [guard] without a `String`, and zeroes the buffers. */
    fun copy(guard: ClipboardGuard, secret: SecretValue, clearAfterSeconds: Int) {
        val bytes = secret.exposeBytes()
        val chars = Charsets.UTF_8.decode(ByteBuffer.wrap(bytes))
        try {
            guard.copy(chars, clearAfterSeconds)
        } finally {
            bytes.fill(0)
            if (chars.hasArray()) chars.array().fill('\u0000')
        }
    }
}
