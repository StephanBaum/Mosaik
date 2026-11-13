package io.github.stephanbaum.mosaic.security

import android.app.KeyguardManager
import android.content.Context
import android.content.pm.PackageManager
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.os.Build
import android.os.CancellationSignal
import javax.crypto.Cipher
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/** The texts of the system prompt (from the catalog; the caller localizes them). */
class PromptText(val title: String, val subtitle: String, val negativeButton: String)

/**
 * The biometric step of quick unlock: the framework `BiometricPrompt` with a `CryptoObject`, Class 3
 * (`BIOMETRIC_STRONG`) from API 30; on API 28 to 29 the platform offers only strong sensors for a key that
 * needs a biometric per use (docs/MOBILE.md, ADR-0090). The framework class is used instead of
 * `androidx.biometric`, so the app adds no dependency for it (deviation recorded in the commit).
 */
@Suppress("NULLABILITY_MISMATCH_BASED_ON_JAVA_ANNOTATIONS")
class BiometricGate {
    sealed interface Result {
        /** The user authenticated; the cipher is now usable once. */
        class Authenticated(val cipher: Cipher) : Result

        /** The user cancelled or chose the master password. */
        data object Cancelled : Result

        /** The sensor failed or locked out; [code] is the framework error code. */
        class Error(val code: Int) : Result
    }

    /** Whether this phone can offer quick unlock at all: a secure lock screen and a strong biometric. */
    @Suppress("DEPRECATION")
    fun isAvailable(context: Context): Boolean {
        val keyguard = context.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
        if (!keyguard.isDeviceSecure) return false
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val manager = context.getSystemService(BiometricManager::class.java) ?: return false
            return manager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG) ==
                BiometricManager.BIOMETRIC_SUCCESS
        }
        return context.packageManager.hasSystemFeature(PackageManager.FEATURE_FINGERPRINT)
    }

    /** Shows the prompt for [cipher]; call it on the main thread. */
    suspend fun authenticate(context: Context, cipher: Cipher, text: PromptText): Result =
        suspendCancellableCoroutine { continuation ->
            val signal = CancellationSignal()
            continuation.invokeOnCancellation { signal.cancel() }
            val executor = context.mainExecutor
            val builder =
                BiometricPrompt.Builder(context)
                    .setTitle(text.title)
                    .setSubtitle(text.subtitle)
                    .setNegativeButton(text.negativeButton, executor) { _, _ ->
                        if (continuation.isActive) continuation.resume(Result.Cancelled)
                    }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                builder.setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
            }
            val callback =
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        if (!continuation.isActive) return
                        val authenticated = result.cryptoObject?.cipher
                        continuation.resume(
                            if (authenticated != null) Result.Authenticated(authenticated) else Result.Error(CODE_NO_CIPHER),
                        )
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        if (!continuation.isActive) return
                        val cancelled = errorCode == ERROR_CANCELED || errorCode == ERROR_USER_CANCELED
                        continuation.resume(if (cancelled) Result.Cancelled else Result.Error(errorCode))
                    }
                }
            builder.build().authenticate(BiometricPrompt.CryptoObject(cipher), signal, executor, callback)
        }

    private companion object {
        // BiometricPrompt.BIOMETRIC_ERROR_CANCELED and BIOMETRIC_ERROR_USER_CANCELED.
        const val ERROR_CANCELED = 5
        const val ERROR_USER_CANCELED = 10
        const val CODE_NO_CIPHER = -1
    }
}
