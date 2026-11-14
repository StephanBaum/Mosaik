package io.github.stephanbaum.mosaic.autofill

import java.net.URI
import java.net.URISyntaxException

/**
 * Reads the origin a privileged browser gives through `CallingAppInfo.getOrigin` (`scheme://host[:port]`) into the
 * web domain and scheme the core's `fill_target_android` wants. Anything that is not exactly an origin (a path, a
 * query, user information, another scheme) is refused, so nothing but a clean origin becomes a web target.
 */
object PlatformOrigin {
    class Parts(val domain: String, val scheme: String)

    fun parse(origin: String?): Parts? {
        if (origin == null) return null
        val uri =
            try {
                URI(origin)
            } catch (e: URISyntaxException) {
                return null
            }
        val scheme = uri.scheme?.lowercase()
        if (scheme != "https" && scheme != "http") return null
        if (uri.rawUserInfo != null || !uri.rawPath.isNullOrEmpty() || uri.rawQuery != null || uri.rawFragment != null) {
            return null
        }
        val host = uri.host ?: return null
        val domain = if (uri.port == -1) host else "$host:${uri.port}"
        return Parts(domain, scheme)
    }
}
