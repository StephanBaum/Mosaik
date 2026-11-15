package io.github.stephanbaum.mosaic.session

/**
 * The redirect of a cloud-folder sign-in (RFC 8252 section 7.1, private-use scheme): the provider's page ends on
 * `io.github.stephanbaum.mosaic.oauth:/cb?code=...&state=...`. Any app can register a scheme, so this check is only
 * a size and shape filter; what protects the sign-in is the PKCE verifier and the `state` the core checks.
 */
object OAuthCallback {
    const val SCHEME = "io.github.stephanbaum.mosaic.oauth"

    /** The longest callback the core is asked to parse. */
    const val MAX_LENGTH = 4096

    private const val PREFIX = "$SCHEME:/cb?"

    /** True only for the exact callback shape of at most [MAX_LENGTH] characters. */
    fun accepts(text: String): Boolean = text.length <= MAX_LENGTH && text.startsWith(PREFIX)
}
