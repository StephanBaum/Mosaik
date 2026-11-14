package io.github.stephanbaum.mosaic.core

import java.nio.CharBuffer

/**
 * A secret kept as bytes, never as a `String` (strings cannot be wiped). The bytes are handed out once,
 * through [consume], and zeroed afterwards. Not `Parcelable`, and `toString` never shows the content.
 */
class SecretBytes private constructor(
    private var bytes: ByteArray?,
) : AutoCloseable {
    /** True once the secret has been wiped. */
    val isWiped: Boolean get() = bytes == null

    /** Length in bytes, or 0 after a wipe. */
    val size: Int get() = bytes?.size ?: 0

    /** Hands the bytes to [block], then wipes them. Throws if the secret was already wiped. */
    fun <T> consume(block: (ByteArray) -> T): T {
        val current = checkNotNull(bytes) { "secret already wiped" }
        try {
            return block(current)
        } finally {
            wipe()
        }
    }

    /** Zeroes the bytes. Safe to call twice. */
    fun wipe() {
        bytes?.fill(0)
        bytes = null
    }

    override fun close() = wipe()

    override fun toString(): String = "SecretBytes([REDACTED])"

    companion object {
        /** Takes ownership of [source]: the caller must not keep or reuse the array. */
        fun of(source: ByteArray): SecretBytes = SecretBytes(source)

        /** Encodes [chars] as UTF-8 without creating a `String`, then zeroes [chars] and the encoder buffer. */
        fun ofUtf8(chars: CharArray): SecretBytes {
            val encoded = Charsets.UTF_8.encode(CharBuffer.wrap(chars))
            val exact = ByteArray(encoded.remaining())
            encoded.get(exact)
            if (encoded.hasArray()) encoded.array().fill(0)
            chars.fill('\u0000')
            return SecretBytes(exact)
        }
    }
}
