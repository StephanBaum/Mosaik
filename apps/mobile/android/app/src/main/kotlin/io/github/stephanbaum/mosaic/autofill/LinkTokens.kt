package io.github.stephanbaum.mosaic.autofill

import java.security.SecureRandom

/**
 * One-time tokens for the "No login for X" entry (M10-G2 M3). The entry used to open the exported main activity with
 * the package name in an extra, which any app could send. Now the entry opens [LinkActivity], which is not
 * exported, with a random token; the token maps to the package here, in memory, and is valid for [TTL_MS] and
 * once. A token that is unknown, used or expired gives nothing.
 */
class LinkTokens(
    private val clock: () -> Long,
    private val random: () -> String = { LinkTokens.secureToken() },
) {
    private class Entry(val packageName: String, val expiresAt: Long)

    private val entries = HashMap<String, Entry>()

    /** A token for [packageName]. */
    @Synchronized
    fun issue(packageName: String): String {
        val now = clock()
        purge(now)
        // A bound on the map: the oldest entries go first, so a flood of requests cannot grow it.
        while (entries.size >= MAX_ENTRIES) {
            val oldest = entries.minByOrNull { it.value.expiresAt }?.key ?: break
            entries.remove(oldest)
        }
        val token = random()
        entries[token] = Entry(packageName, now + TTL_MS)
        return token
    }

    /** The package of [token], or null; the token is used up either way. */
    @Synchronized
    fun redeem(token: String?): String? {
        if (token == null) return null
        val now = clock()
        purge(now)
        return entries.remove(token)?.packageName
    }

    private fun purge(now: Long) {
        entries.entries.removeAll { it.value.expiresAt <= now }
    }

    companion object {
        /** How long a token can be redeemed: two minutes. */
        const val TTL_MS = 120_000L

        private const val MAX_ENTRIES = 16
        private const val TOKEN_BYTES = 16
        private val secure = SecureRandom()

        /** 128 random bits, hex. */
        fun secureToken(): String {
            val bytes = ByteArray(TOKEN_BYTES)
            secure.nextBytes(bytes)
            return bytes.joinToString("") { "%02x".format(it.toInt() and 0xFF) }
        }
    }
}
