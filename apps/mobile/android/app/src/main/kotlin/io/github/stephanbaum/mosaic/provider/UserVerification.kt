package io.github.stephanbaum.mosaic.provider

import android.content.Context
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.os.Build
import android.os.CancellationSignal
import androidx.annotation.RequiresApi
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/**
 * User verification for a passkey ceremony (ADR-0093): the system prompt with a strong biometric or the device
 * credential, every time. A ceremony without it is refused, so the core is told `user_verified = true` only after
 * this returned true.
 */
@RequiresApi(Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
object UserVerification {
    /** True when the user verified; false on cancel or any failure. Call it on the main thread. */
    suspend fun verify(context: Context, title: String, subtitle: String): Boolean =
        suspendCancellableCoroutine { continuation ->
            val signal = CancellationSignal()
            continuation.invokeOnCancellation { signal.cancel() }
            val prompt =
                BiometricPrompt.Builder(context)
                    .setTitle(title)
                    .setSubtitle(subtitle)
                    .setAllowedAuthenticators(
                        BiometricManager.Authenticators.BIOMETRIC_STRONG or BiometricManager.Authenticators.DEVICE_CREDENTIAL,
                    )
                    .build()
            prompt.authenticate(
                signal,
                context.mainExecutor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        if (continuation.isActive) continuation.resume(true)
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        if (continuation.isActive) continuation.resume(false)
                    }
                },
            )
        }
}
