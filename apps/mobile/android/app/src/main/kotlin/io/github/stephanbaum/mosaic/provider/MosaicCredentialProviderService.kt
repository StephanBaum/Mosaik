package io.github.stephanbaum.mosaic.provider

import android.os.Build
import android.os.CancellationSignal
import android.os.OutcomeReceiver
import androidx.annotation.RequiresApi
import androidx.credentials.exceptions.ClearCredentialException
import androidx.credentials.exceptions.CreateCredentialException
import androidx.credentials.exceptions.CreateCredentialUnknownException
import androidx.credentials.exceptions.GetCredentialException
import androidx.credentials.exceptions.GetCredentialUnknownException
import androidx.credentials.provider.BeginCreateCredentialRequest
import androidx.credentials.provider.BeginCreateCredentialResponse
import androidx.credentials.provider.BeginGetCredentialRequest
import androidx.credentials.provider.BeginGetCredentialResponse
import androidx.credentials.provider.CredentialProviderService
import androidx.credentials.provider.ProviderClearCredentialStateRequest
import io.github.stephanbaum.mosaic.MosaicApp
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/**
 * The Credential Manager provider (API 34+): passwords and passkeys (ADR-0092, ADR-0093). The service only lists
 * entries; it never returns a value. A value, a signature or a new passkey is made by [CredentialActivity] after
 * the user chose an entry, and every passkey ceremony needs user verification there.
 */
@RequiresApi(Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
class MosaicCredentialProviderService : CredentialProviderService() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    override fun onBeginGetCredentialRequest(
        request: BeginGetCredentialRequest,
        cancellationSignal: CancellationSignal,
        callback: OutcomeReceiver<BeginGetCredentialResponse, GetCredentialException>,
    ) {
        val job =
            scope.launch {
                val response =
                    try {
                        CredentialEntries(this@MosaicCredentialProviderService, application as MosaicApp).begin(request)
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        callback.onError(GetCredentialUnknownException())
                        return@launch
                    }
                callback.onResult(response)
            }
        cancellationSignal.setOnCancelListener { job.cancel() }
    }

    override fun onBeginCreateCredentialRequest(
        request: BeginCreateCredentialRequest,
        cancellationSignal: CancellationSignal,
        callback: OutcomeReceiver<BeginCreateCredentialResponse, CreateCredentialException>,
    ) {
        try {
            callback.onResult(CredentialEntries(this, application as MosaicApp).beginCreate(request))
        } catch (e: Exception) {
            callback.onError(CreateCredentialUnknownException())
        }
    }

    override fun onClearCredentialStateRequest(
        request: ProviderClearCredentialStateRequest,
        cancellationSignal: CancellationSignal,
        callback: OutcomeReceiver<Void?, ClearCredentialException>,
    ) {
        // Mosaic keeps no sign-in state for apps: there is nothing to clear.
        callback.onResult(null)
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }
}
