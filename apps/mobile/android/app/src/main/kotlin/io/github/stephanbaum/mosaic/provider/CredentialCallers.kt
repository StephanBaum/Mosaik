package io.github.stephanbaum.mosaic.provider

import android.os.Build
import androidx.annotation.RequiresApi
import androidx.credentials.provider.CallingAppInfo
import io.github.stephanbaum.mosaic.autofill.PlatformOrigin
import io.github.stephanbaum.mosaic.autofill.Signers
import io.github.stephanbaum.mosaic.core.CoreHolder
import io.github.stephanbaum.mosaic.ffi.AndroidCaller

/**
 * Who is asking a Credential Manager request: the package, its current signer, and, for a browser the platform
 * trusts, the origin of the page. The core checks all of it again (`is_trusted_browser`, the exact-origin rules).
 */
class ProviderCaller(
    private val packageName: String,
    private val signers: List<ByteArray>,
    /** `CallingAppInfo.getOrigin(privileged_browsers_json())`, null for any app that is not an allowlisted browser. */
    val platformOrigin: String?,
) {
    /** For a passkey request: the core takes the origin from [platformOrigin] and the package from here. */
    fun forPasskeys(): AndroidCaller = AndroidCaller(packageName, signers, null, null)

    /**
     * For a password request: a browser's page becomes a web target through the origin it gave; any other app is
     * identified by its package and signer. An origin that is not a clean `scheme://host[:port]` gives null, so
     * nothing is offered.
     */
    fun forPasswords(): AndroidCaller? {
        if (platformOrigin == null) return AndroidCaller(packageName, signers, null, null)
        val web = PlatformOrigin.parse(platformOrigin) ?: return null
        return AndroidCaller(packageName, signers, web.domain, web.scheme)
    }
}

@RequiresApi(Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
object CredentialCallers {
    /**
     * Reads [info]. Null when the caller cannot be identified: no signing information, or an origin was supplied by
     * an app the allowlist does not trust (then nothing is offered at all).
     */
    @Suppress("DEPRECATION")
    suspend fun of(holder: CoreHolder, info: CallingAppInfo): ProviderCaller? {
        val signers = Signers.of(info.signingInfo)
        if (signers.isEmpty()) return null
        var origin: String? = null
        if (info.isOriginPopulated()) {
            val allowlist = holder.call { it.privilegedBrowsersJson() }
            origin =
                try {
                    info.getOrigin(allowlist)
                } catch (e: IllegalArgumentException) {
                    null
                } catch (e: IllegalStateException) {
                    null
                }
            if (origin == null) return null
        }
        return ProviderCaller(info.packageName, signers, origin)
    }
}
