package io.github.stephanbaum.mosaic.provider

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.annotation.RequiresApi
import androidx.credentials.provider.AuthenticationAction
import androidx.credentials.provider.BeginCreateCredentialRequest
import androidx.credentials.provider.BeginCreateCredentialResponse
import androidx.credentials.provider.BeginCreatePublicKeyCredentialRequest
import androidx.credentials.provider.BeginGetCredentialRequest
import androidx.credentials.provider.BeginGetCredentialResponse
import androidx.credentials.provider.BeginGetPasswordOption
import androidx.credentials.provider.BeginGetPublicKeyCredentialOption
import androidx.credentials.provider.CreateEntry
import androidx.credentials.provider.PasswordCredentialEntry
import androidx.credentials.provider.PublicKeyCredentialEntry
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.AndroidPasskeyRequest
import io.github.stephanbaum.mosaic.ffi.ItemKey
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import java.security.SecureRandom
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** The extras of the credential activity's pending intents. They name the chosen entry and never carry a value. */
object CredentialIntents {
    const val MODE_UNLOCK = "unlock"
    const val MODE_PASSWORD = "password"
    const val MODE_PASSKEY = "passkey"
    const val MODE_CREATE = "create"

    const val EXTRA_MODE = "io.github.stephanbaum.mosaic.provider.MODE"
    const val EXTRA_ITEM_VAULT = "io.github.stephanbaum.mosaic.provider.ITEM_VAULT"
    const val EXTRA_ITEM_ID = "io.github.stephanbaum.mosaic.provider.ITEM_ID"

    private val random = SecureRandom()

    /**
     * Mutable (the platform adds the request), with a random request code per entry (an app cannot guess it to
     * create the pending intent first) and `FLAG_CANCEL_CURRENT` (M10-G2 L5). The component is explicit.
     */
    fun pending(context: Context, mode: String, key: ItemKey?): PendingIntent {
        val intent = Intent(context, CredentialActivity::class.java).putExtra(EXTRA_MODE, mode)
        if (key != null) intent.putExtra(EXTRA_ITEM_VAULT, key.vaultId).putExtra(EXTRA_ITEM_ID, key.itemId)
        return PendingIntent.getActivity(
            context,
            random.nextInt(),
            intent,
            PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_CANCEL_CURRENT,
        )
    }
}

/**
 * Builds what the provider answers a Credential Manager request with (ADR-0092 and ADR-0093). Locked: one
 * authentication action ("Unlock Mosaic"). Unlocked: an entry per login that the core matches and per passkey whose
 * relying party equals the request's exactly. An entry holds a title and a user name and a pending intent; the value
 * or the signature is made only by [CredentialActivity] after the user chose it.
 */
@RequiresApi(Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
class CredentialEntries(private val context: Context, private val app: MosaicApp) {
    private val resources = context.resources

    suspend fun begin(request: BeginGetCredentialRequest): BeginGetCredentialResponse {
        val builder = BeginGetCredentialResponse.Builder()
        val info = request.callingAppInfo ?: return builder.build()
        // A late answer must not hand out entries of a session that is past its deadline (M10-G2 L4).
        withContext(Dispatchers.Main) { app.lockPolicy.tick() }
        val caller = CredentialCallers.of(app.holder, info) ?: return builder.build()
        if (app.holder.state() !is SessionState.Unlocked) {
            val unlock = CredentialIntents.pending(context, CredentialIntents.MODE_UNLOCK, null)
            return builder.addAuthenticationAction(AuthenticationAction(L10n.credentialUnlockEntry(resources), unlock)).build()
        }
        for (option in request.beginGetCredentialOptions) {
            when (option) {
                is BeginGetPasswordOption -> addPasswords(builder, option, caller)
                is BeginGetPublicKeyCredentialOption -> addPasskeys(builder, option, caller)
                else -> Unit
            }
        }
        return builder.build()
    }

    /** A passkey create request: one entry; the activity validates the request after the unlock. */
    fun beginCreate(request: BeginCreateCredentialRequest): BeginCreateCredentialResponse {
        val builder = BeginCreateCredentialResponse.Builder()
        if (request !is BeginCreatePublicKeyCredentialRequest) return builder.build()
        val pending = CredentialIntents.pending(context, CredentialIntents.MODE_CREATE, null)
        val entry =
            CreateEntry.Builder(L10n.appName(resources), pending)
                .setDescription(L10n.credentialCreateEntry(resources))
                .build()
        return builder.addCreateEntry(entry).build()
    }

    private suspend fun addPasswords(
        builder: BeginGetCredentialResponse.Builder,
        option: BeginGetPasswordOption,
        caller: ProviderCaller,
    ) {
        val androidCaller = caller.forPasswords() ?: return
        try {
            val target = app.holder.call { it.fillTargetAndroid(androidCaller) }
            val found = app.holder.call { it.fillCandidates(target) }
            for (candidate in found.candidates) {
                val pending = CredentialIntents.pending(context, CredentialIntents.MODE_PASSWORD, candidate.key)
                builder.addCredentialEntry(
                    PasswordCredentialEntry.Builder(context, candidate.username ?: candidate.title, pending, option)
                        .setDisplayName(candidate.title)
                        .build(),
                )
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            // No target, no match, a locked core: nothing is offered.
        }
    }

    private suspend fun addPasskeys(
        builder: BeginGetCredentialResponse.Builder,
        option: BeginGetPublicKeyCredentialOption,
        caller: ProviderCaller,
    ) {
        try {
            val request = AndroidPasskeyRequest(option.requestJson, caller.forPasskeys(), caller.platformOrigin, option.clientDataHash)
            val session = app.holder.call { it.passkeyAndroidGetBegin(request) }
            for (candidate in session.candidates) {
                val pending = CredentialIntents.pending(context, CredentialIntents.MODE_PASSKEY, candidate.key)
                builder.addCredentialEntry(
                    PublicKeyCredentialEntry.Builder(context, candidate.userName.ifBlank { candidate.rpId }, pending, option)
                        .setDisplayName(candidate.displayName.ifBlank { candidate.rpId })
                        .build(),
                )
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            // The request is refused (rp ID, app not verified, no network): nothing is offered.
        }
    }
}
