package io.github.stephanbaum.mosaic.provider

import android.content.Intent
import android.os.Build
import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.annotation.RequiresApi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.credentials.CreatePublicKeyCredentialRequest
import androidx.credentials.CreatePublicKeyCredentialResponse
import androidx.credentials.GetCredentialResponse
import androidx.credentials.GetPublicKeyCredentialOption
import androidx.credentials.PasswordCredential
import androidx.credentials.PublicKeyCredential
import androidx.credentials.exceptions.CreateCredentialUnknownException
import androidx.credentials.exceptions.GetCredentialUnknownException
import androidx.credentials.provider.PendingIntentHandler
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.AndroidPasskeyRequest
import io.github.stephanbaum.mosaic.ffi.ItemKey
import io.github.stephanbaum.mosaic.ffi.PasskeyCreateSession
import io.github.stephanbaum.mosaic.security.ErrorCodes
import io.github.stephanbaum.mosaic.security.SecureActivity
import io.github.stephanbaum.mosaic.ui.MosaicTheme
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.ChoiceRow
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.FieldLabel
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.SecondaryButton
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.components.Working
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The activity behind the entries of [MosaicCredentialProviderService] (ADR-0092, ADR-0093). The platform starts it
 * through the entry's pending intent, in one of four modes:
 *
 *  - **unlock**: unlock, then answer the begin request with the entries.
 *  - **password**: look the target up again, check that the chosen login is still a candidate, return its password.
 *  - **passkey**: the core validates the request (rp ID, asset links for a non-browser app) and finds the passkey;
 *    the user verifies (strong biometric or device credential); the core signs, persisting the sign count first.
 *  - **create**: the core validates the request, the user confirms and verifies, the core stores the new passkey
 *    **before** the response is returned.
 *
 * The provider flow stays open after the unlock step and ends with the chosen entry, so a session can survive the
 * step between them for at most the provider grace (ADR-0094).
 */
@RequiresApi(Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
class CredentialActivity : SecureActivity() {
    private class Failure(val code: String) : Exception()

    private class VerifyFailed : Exception()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val app = application as MosaicApp
        val mode = intent.getStringExtra(CredentialIntents.EXTRA_MODE)
        if (mode == null) {
            setResult(RESULT_CANCELED)
            finish()
            return
        }
        setContent {
            MosaicTheme {
                ProviderGate(app, ProviderFlows.CREDENTIAL) {
                    if (mode == CredentialIntents.MODE_CREATE) CreateRunner(app) else GetRunner(app, mode)
                }
            }
        }
    }

    // ---- get ----------------------------------------------------------------------------------

    @Composable
    private fun GetRunner(app: MosaicApp, mode: String) {
        var failure by remember { mutableStateOf<String?>(null) }
        LaunchedEffect(Unit) {
            try {
                val data =
                    when (mode) {
                        CredentialIntents.MODE_UNLOCK -> unlockList(app)
                        CredentialIntents.MODE_PASSWORD -> getPassword(app)
                        CredentialIntents.MODE_PASSKEY -> getPasskey(app)
                        else -> throw Failure("invalidInput")
                    }
                setResult(RESULT_OK, data)
                if (mode != CredentialIntents.MODE_UNLOCK) app.lockPolicy.closeProviderFlow(ProviderFlows.CREDENTIAL)
                finish()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Failure) {
                failure = e.code
            } catch (e: VerifyFailed) {
                failure = VERIFY_FAILED
            } catch (e: Exception) {
                failure = ErrorCodes.of(e)
            }
        }
        val code = failure
        if (code == null) {
            ScreenFrame { }
        } else {
            FailureScreen(code) {
                val data = Intent()
                PendingIntentHandler.setGetCredentialException(data, GetCredentialUnknownException())
                closeWith(app, data)
            }
        }
    }

    private suspend fun unlockList(app: MosaicApp): Intent {
        val request = PendingIntentHandler.retrieveBeginGetCredentialRequest(intent) ?: throw Failure("invalidInput")
        val response = CredentialEntries(this, app).begin(request)
        val data = Intent()
        PendingIntentHandler.setBeginGetCredentialResponse(data, response)
        return data
    }

    private suspend fun getPassword(app: MosaicApp): Intent {
        val request = PendingIntentHandler.retrieveProviderGetCredentialRequest(intent) ?: throw Failure("invalidInput")
        val key = chosenKey()
        val caller = CredentialCallers.of(app.holder, request.callingAppInfo) ?: throw Failure("noMatch")
        val androidCaller = caller.forPasswords() ?: throw Failure("noMatch")
        // The target is made again and the choice is checked against fresh candidates: the pending intent names an
        // entry, and the core decides whether it may still fill this request.
        val target = app.holder.call { it.fillTargetAndroid(androidCaller) }
        val values =
            app.holder.call { core ->
                val found = core.fillCandidates(target)
                try {
                    if (found.candidates.none { it.key == key }) throw Failure("noMatch")
                    core.fillValues(found.sessionId, key)
                } finally {
                    core.fillRelease(found.sessionId)
                }
            }
        val secret = values.password ?: throw Failure("notFound")
        // The platform's PasswordCredential takes a String, which cannot be wiped; the core's copy and the bytes are.
        val bytes = secret.exposeBytes()
        val password =
            try {
                String(bytes, Charsets.UTF_8)
            } finally {
                bytes.fill(0)
                secret.wipe()
            }
        val data = Intent()
        PendingIntentHandler.setGetCredentialResponse(
            data,
            GetCredentialResponse(PasswordCredential(values.username.orEmpty(), password)),
        )
        return data
    }

    private suspend fun getPasskey(app: MosaicApp): Intent {
        val request = PendingIntentHandler.retrieveProviderGetCredentialRequest(intent) ?: throw Failure("invalidInput")
        val option =
            request.credentialOptions.filterIsInstance<GetPublicKeyCredentialOption>().firstOrNull()
                ?: throw Failure("invalidInput")
        val key = chosenKey()
        val caller = CredentialCallers.of(app.holder, request.callingAppInfo) ?: throw Failure("noMatch")
        val passkeyRequest = AndroidPasskeyRequest(option.requestJson, caller.forPasskeys(), caller.platformOrigin, option.clientDataHash)
        val session = app.holder.call { it.passkeyAndroidGetBegin(passkeyRequest) }
        val json =
            try {
                if (session.candidates.none { it.key == key }) throw Failure("passkeyNotFound")
                val verified =
                    UserVerification.verify(
                        this,
                        L10n.credentialVerifyTitle(resources),
                        L10n.credentialVerifySignIn(resources, session.rpId),
                    )
                if (!verified) throw VerifyFailed()
                app.holder.call { it.passkeyAndroidGetFinish(session.sessionId, key, true) }
            } finally {
                // A finished session is gone already; every other exit forgets it here (M10-G3).
                withContext(NonCancellable) { app.holder.call { it.passkeyRelease(session.sessionId) } }
            }
        val data = Intent()
        PendingIntentHandler.setGetCredentialResponse(data, GetCredentialResponse(PublicKeyCredential(json)))
        return data
    }

    private fun chosenKey(): ItemKey {
        val vault = intent.getStringExtra(CredentialIntents.EXTRA_ITEM_VAULT)
        val item = intent.getStringExtra(CredentialIntents.EXTRA_ITEM_ID)
        if (vault == null || item == null) throw Failure("invalidInput")
        return ItemKey(vault, item)
    }

    // ---- create -------------------------------------------------------------------------------

    @Composable
    private fun CreateRunner(app: MosaicApp) {
        val resources = LocalContext.current.resources
        val scope = rememberCoroutineScope()
        var session by remember { mutableStateOf<PasskeyCreateSession?>(null) }
        var vaultId by remember { mutableStateOf<String?>(null) }
        var failure by remember { mutableStateOf<String?>(null) }
        var busy by remember { mutableStateOf(false) }

        LaunchedEffect(Unit) {
            try {
                val begun = beginCreate(app)
                session = begun
                vaultId = begun.vaults.firstOrNull { it.available }?.vaultId
            } catch (e: CancellationException) {
                throw e
            } catch (e: Failure) {
                failure = e.code
            } catch (e: Exception) {
                failure = ErrorCodes.of(e)
            }
        }

        val code = failure
        val current = session
        if (code != null) {
            FailureScreen(code) {
                val data = Intent()
                PendingIntentHandler.setCreateCredentialException(data, CreateCredentialUnknownException())
                closeWith(app, data)
            }
        } else if (current == null) {
            ScreenFrame { Working(L10n.commonWorking(resources)) }
        } else {
            ScreenFrame {
                TitleText(L10n.credentialCreateTitle(resources))
                BodyText(L10n.credentialCreateBody(resources, current.rpId, current.userName))
                val choices = current.vaults.filter { it.available }
                if (choices.size > 1) {
                    FieldLabel(L10n.credentialCreateVault(resources))
                    for (vault in choices) {
                        ChoiceRow(
                            selected = vaultId == vault.vaultId,
                            label = vault.name ?: L10n.credentialVaultDefault(resources),
                            onSelect = { vaultId = vault.vaultId },
                        )
                    }
                }
                if (busy) Working(L10n.commonWorking(resources))
                PrimaryButton(
                    L10n.credentialCreateConfirm(resources),
                    enabled = !busy && vaultId != null,
                    onClick = {
                        val chosen = vaultId
                        if (chosen != null) {
                            busy = true
                            scope.launch {
                                try {
                                    closeWith(app, finishCreate(app, current, chosen))
                                } catch (e: CancellationException) {
                                    throw e
                                } catch (e: VerifyFailed) {
                                    failure = VERIFY_FAILED
                                } catch (e: Exception) {
                                    failure = ErrorCodes.of(e)
                                } finally {
                                    busy = false
                                }
                            }
                        }
                    },
                )
                SecondaryButton(L10n.commonCancel(resources), enabled = !busy, onClick = { closeWith(app, null) })
            }
        }
    }

    private suspend fun beginCreate(app: MosaicApp): PasskeyCreateSession {
        val request = PendingIntentHandler.retrieveProviderCreateCredentialRequest(intent) ?: throw Failure("invalidInput")
        val create = request.callingRequest as? CreatePublicKeyCredentialRequest ?: throw Failure("invalidInput")
        val caller = CredentialCallers.of(app.holder, request.callingAppInfo) ?: throw Failure("noMatch")
        val passkeyRequest = AndroidPasskeyRequest(create.requestJson, caller.forPasskeys(), caller.platformOrigin, create.clientDataHash)
        return app.holder.call { it.passkeyAndroidCreateBegin(passkeyRequest) }
    }

    private suspend fun finishCreate(app: MosaicApp, session: PasskeyCreateSession, vaultId: String): Intent {
        val verified =
            UserVerification.verify(
                this,
                L10n.credentialVerifyTitle(resources),
                L10n.credentialVerifyCreate(resources, session.rpId),
            )
        if (!verified) throw VerifyFailed()
        // The core stores the passkey before it returns the response (ADR-0093).
        val json = app.holder.call { it.passkeyAndroidCreateFinish(session.sessionId, vaultId, true) }
        val data = Intent()
        PendingIntentHandler.setCreateCredentialResponse(data, CreatePublicKeyCredentialResponse(json))
        return data
    }

    // ---- shared -------------------------------------------------------------------------------

    @Composable
    private fun FailureScreen(code: String, onClose: () -> Unit) {
        val resources = LocalContext.current.resources
        ScreenFrame {
            TitleText(L10n.appName(resources))
            ErrorText(if (code == VERIFY_FAILED) L10n.credentialVerifyFailed(resources) else errorText(resources, code))
            PrimaryButton(L10n.commonClose(resources), onClick = onClose)
        }
    }

    /** Ends the ceremony: with a result, or cancelled when [data] is null. Ends the provider flow too. */
    private fun closeWith(app: MosaicApp, data: Intent?) {
        if (data == null) setResult(RESULT_CANCELED) else setResult(RESULT_OK, data)
        app.lockPolicy.closeProviderFlow(ProviderFlows.CREDENTIAL)
        finish()
    }

    private companion object {
        /** Not a core code: the user cancelled or failed the verification prompt. */
        const val VERIFY_FAILED = "verifyFailed"
    }
}
