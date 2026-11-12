package io.github.stephanbaum.mosaic.session

import android.content.Context
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.UserNotAuthenticatedException
import io.github.stephanbaum.mosaic.core.CoreHolder
import io.github.stephanbaum.mosaic.core.SecretBytes
import io.github.stephanbaum.mosaic.ffi.HandoffInfo
import io.github.stephanbaum.mosaic.ffi.HandoffStatus
import io.github.stephanbaum.mosaic.ffi.JoinHostedRequest
import io.github.stephanbaum.mosaic.ffi.JoinOutcome
import io.github.stephanbaum.mosaic.ffi.MobileSettings
import io.github.stephanbaum.mosaic.ffi.QuickUnlockAvailability
import io.github.stephanbaum.mosaic.ffi.QuickUnlockPlatform
import io.github.stephanbaum.mosaic.ffi.QuickUnlockSetup
import io.github.stephanbaum.mosaic.ffi.ServerInfo
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.security.BiometricGate
import io.github.stephanbaum.mosaic.security.ErrorCodes
import io.github.stephanbaum.mosaic.security.MemoryCheck
import io.github.stephanbaum.mosaic.security.PromptText
import io.github.stephanbaum.mosaic.security.QuickUnlockKeyMissingException
import io.github.stephanbaum.mosaic.security.QuickUnlockKeyParams
import io.github.stephanbaum.mosaic.security.QuickUnlockKeys
import io.github.stephanbaum.mosaic.security.SecureStorageFailure
import java.security.GeneralSecurityException
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.CancellationException

/** The result of a session operation: a value, a core error code (`error.<code>`), or a cancel by the user. */
sealed interface Outcome<out T> {
    class Ok<T>(val value: T) : Outcome<T>

    class Failed(val code: String) : Outcome<Nothing>

    data object Cancelled : Outcome<Nothing>
}

/**
 * Whether a join is waiting for its second factor. While it is, the lock policy must not call `lock()`,
 * because that drops the pending join (docs/MOBILE.md section 2: a TOTP app switch must survive).
 */
class JoinTracker {
    private val pending = AtomicBoolean(false)

    val isPending: Boolean get() = pending.get()

    fun set(value: Boolean) {
        pending.set(value)
    }
}

private class NotEnoughMemory : Exception()

private class QuickUnlockFailure(val code: String) : Exception()

private class QuickUnlockCancelled : Exception()

/**
 * The session operations of the onboarding and unlock screens: every call into the core goes through
 * [CoreHolder] (off the main thread), secrets travel as [SecretBytes] and are wiped when the call is over,
 * and quick unlock follows CRYPTO_SPEC 6.2 / 6.9 (Rust decides, this class drives the Keystore and the prompt).
 */
class SessionController(
    private val holder: CoreHolder,
    private val keys: QuickUnlockKeys,
    private val gate: BiometricGate,
    private val availableMemory: () -> Long,
    private val joinTracker: JoinTracker,
    private val onSettings: (MobileSettings) -> Unit,
    private val onSessionChanged: () -> Unit,
) {
    // ---- state ----------------------------------------------------------------------------------

    /** The session state; a closed store is reopened first. */
    suspend fun currentState(): SessionState =
        holder.call { core ->
            val state = core.state()
            if (state is SessionState.StoreClosed) core.reopenStore() else state
        }

    /** Deletes the Keystore key of an expired quick-unlock enrolment (the core has already dropped the wrapping). */
    fun deleteKey(alias: String) {
        keys.delete(alias)
    }

    /** Tells the screens to read the state again. */
    fun notifyChanged() {
        onSessionChanged()
    }

    // ---- onboarding -----------------------------------------------------------------------------

    suspend fun checkServer(url: String): Outcome<ServerInfo> = guarded { holder.call { it.preloginCheck(url) } }

    /**
     * The first join step. A new phone has no stored profile yet, so no memory preflight is possible here.
     * [secretKey] is kept by the onboarding flow across attempts, so a wrong password does not ask for it again:
     * each attempt works on a copy that is wiped afterwards.
     */
    suspend fun join(
        serverUrl: String,
        email: String,
        deviceName: String,
        password: SecretBytes,
        secretKey: ByteArray,
    ): Outcome<JoinOutcome> =
        guarded {
            val key = SecretBytes.of(secretKey.copyOf())
            try {
                val outcome =
                    holder.call { core ->
                        password.consume { pw ->
                            key.consume { sk ->
                                core.joinHosted(
                                    JoinHostedRequest(
                                        serverUrl = serverUrl,
                                        email = email,
                                        password = pw,
                                        secretKey = sk,
                                        deviceName = deviceName.trim().ifEmpty { null },
                                    ),
                                )
                            }
                        }
                    }
                when (outcome) {
                    is JoinOutcome.Unlocked -> {
                        joinTracker.set(false)
                        afterUnlocked()
                    }
                    is JoinOutcome.SecondFactor -> joinTracker.set(true)
                }
                outcome
            } finally {
                password.wipe()
                key.wipe()
            }
        }

    suspend fun joinTotp(code: SecretBytes): Outcome<Unit> {
        val result =
            guarded {
                try {
                    holder.call { core -> code.consume { core.joinTotp(it) } }
                } finally {
                    code.wipe()
                }
                joinTracker.set(false)
                afterUnlocked()
            }
        if (result is Outcome.Failed && result.code !in KEEP_JOIN_ON) joinTracker.set(false)
        return result
    }

    suspend fun webauthnBegin(): Outcome<HandoffInfo> =
        guarded {
            val info = holder.call { it.joinWebauthnBegin() }
            joinTracker.set(true)
            info
        }

    /** One poll of the security-key handoff; a finished handoff unlocks. */
    suspend fun webauthnStatus(): Outcome<HandoffStatus> =
        guarded {
            val status = holder.call { it.joinWebauthnStatus() }
            when (status) {
                is HandoffStatus.Done -> {
                    joinTracker.set(false)
                    afterUnlocked()
                }
                is HandoffStatus.Failed, is HandoffStatus.Expired -> joinTracker.set(false)
                is HandoffStatus.Waiting -> Unit
            }
            status
        }

    suspend fun cancelJoin() {
        joinTracker.set(false)
        holder.call { it.joinCancel() }
    }

    // ---- unlock ---------------------------------------------------------------------------------

    /**
     * Unlocks with the master password (and the typed Secret Key when none is stored). With [enrollQuick] the
     * same typed bytes start the quick-unlock enrolment inside the same core call (docs/MOBILE.md, section 3.10).
     * An enrolment that does not work never fails the unlock. The value is whether quick unlock is now on.
     */
    suspend fun unlockPassword(
        context: Context,
        password: SecretBytes,
        secretKey: SecretBytes?,
        enrollQuick: Boolean,
        text: PromptText,
    ): Outcome<Boolean> =
        guarded {
            var setup: QuickUnlockSetup? = null
            try {
                preflight()
                setup =
                    holder.call { core ->
                        password.consume { pw ->
                            if (secretKey != null) {
                                secretKey.consume { sk -> core.unlockPassword(pw, sk) }
                            } else {
                                core.unlockPassword(pw, null)
                            }
                            if (enrollQuick) runCatching { core.quickUnlockEnrollBegin(pw) }.getOrNull() else null
                        }
                    }
            } finally {
                password.wipe()
                secretKey?.wipe()
            }
            // The screens switch to the vault only after the optional enrolment, so its prompt is not cancelled
            // by the screen that started it going away.
            loadSettings()
            val pendingSetup = setup
            try {
                pendingSetup != null && runCatching { finishEnroll(context, pendingSetup, text) }.isSuccess
            } finally {
                onSessionChanged()
            }
        }

    /** Unlocks with the Keystore key and a biometric. */
    suspend fun unlockQuick(
        context: Context,
        available: QuickUnlockAvailability.Available,
        text: PromptText,
    ): Outcome<Unit> =
        guarded {
            val body = QuickUnlockKeyParams.decodeBody(available.body)
            if (body == null) {
                dropQuickUnlock(available.keyAlias)
                throw QuickUnlockFailure("quickUnlockInvalidated")
            }
            val cipher =
                try {
                    keys.prepareDecrypt(available.keyAlias, body.iv)
                } catch (e: KeyPermanentlyInvalidatedException) {
                    dropQuickUnlock(available.keyAlias)
                    throw QuickUnlockFailure("quickUnlockInvalidated")
                } catch (e: QuickUnlockKeyMissingException) {
                    dropQuickUnlock(available.keyAlias)
                    throw QuickUnlockFailure("quickUnlockInvalidated")
                }
            val authenticated =
                when (val result = gate.authenticate(context, cipher, text)) {
                    is BiometricGate.Result.Authenticated -> result.cipher
                    BiometricGate.Result.Cancelled -> throw QuickUnlockCancelled()
                    is BiometricGate.Result.Error -> throw QuickUnlockFailure("passwordRequired")
                }
            authenticated.updateAAD(available.aad)
            val w =
                try {
                    authenticated.doFinal(body.ctAndTag)
                } catch (e: UserNotAuthenticatedException) {
                    throw QuickUnlockFailure("passwordRequired")
                } catch (e: GeneralSecurityException) {
                    dropQuickUnlock(available.keyAlias)
                    throw QuickUnlockFailure("quickUnlockInvalidated")
                }
            try {
                holder.call { it.unlockQuick(w) }
            } finally {
                w.fill(0)
            }
            afterUnlocked()
        }

    // ---- quick unlock enrolment -----------------------------------------------------------------

    /** Turns quick unlock on while unlocked: a re-authentication with [password], then the Keystore key. */
    suspend fun enrollQuickUnlock(context: Context, password: SecretBytes, text: PromptText): Outcome<Unit> =
        guarded {
            val setup =
                try {
                    preflight()
                    holder.call { core -> password.consume { core.quickUnlockEnrollBegin(it) } }
                } finally {
                    password.wipe()
                }
            finishEnroll(context, setup, text)
        }

    /** Turns quick unlock off: the core drops the wrapping and names the key, which goes too. */
    suspend fun removeQuickUnlock(): Outcome<Unit> =
        guarded {
            val alias = holder.call { it.quickUnlockRemove() }
            if (alias != null) keys.delete(alias)
            Unit
        }

    /** Creates the key, encrypts `W` behind a biometric, and hands the public body to the core. */
    private suspend fun finishEnroll(context: Context, setup: QuickUnlockSetup, text: PromptText) {
        val alias = setup.keyAlias
        try {
            val prepared = keys.prepareEnroll(alias)
            val authenticated =
                when (val result = gate.authenticate(context, prepared.cipher, text)) {
                    is BiometricGate.Result.Authenticated -> result.cipher
                    BiometricGate.Result.Cancelled -> throw QuickUnlockCancelled()
                    is BiometricGate.Result.Error -> throw QuickUnlockFailure("passwordRequired")
                }
            authenticated.updateAAD(setup.aad)
            val w = setup.w.exposeBytes()
            val sealed =
                try {
                    authenticated.doFinal(w)
                } finally {
                    w.fill(0)
                    setup.w.wipe()
                }
            val body = QuickUnlockKeyParams.encodeBody(prepared.strongBox, authenticated.iv, sealed)
            val platform = QuickUnlockPlatform.AndroidKeystore(prepared.strongBox)
            val enrolled = holder.call { it.quickUnlockEnrollFinish(platform, body) }
            enrolled.replacedKeyAlias?.let { keys.delete(it) }
        } catch (e: Exception) {
            setup.w.wipe()
            keys.delete(alias)
            throw e
        }
    }

    // ---- helpers --------------------------------------------------------------------------------

    private suspend fun preflight() {
        val required = holder.call { it.requiredUnlockMemoryBytes() }
        if (!MemoryCheck.isEnough(required, availableMemory())) throw NotEnoughMemory()
    }

    /** The core drops the wrapping and names the key; the key goes too, also when the core has no alias. */
    private suspend fun dropQuickUnlock(alias: String) {
        val dropped = runCatching { holder.call { it.quickUnlockPlatformFailed() } }.getOrNull()
        keys.delete(dropped ?: alias)
        if (dropped != null && dropped != alias) keys.delete(alias)
    }

    private suspend fun loadSettings() {
        val settings = runCatching { holder.call { it.settings() } }.getOrNull()
        if (settings != null) onSettings(settings)
    }

    private suspend fun afterUnlocked() {
        loadSettings()
        onSessionChanged()
    }

    private suspend fun <T> guarded(block: suspend () -> T): Outcome<T> =
        try {
            Outcome.Ok(block())
        } catch (e: CancellationException) {
            throw e
        } catch (e: QuickUnlockCancelled) {
            Outcome.Cancelled
        } catch (e: QuickUnlockFailure) {
            Outcome.Failed(e.code)
        } catch (e: NotEnoughMemory) {
            Outcome.Failed("notEnoughMemory")
        } catch (e: SecureStorageFailure) {
            Outcome.Failed("storageUnavailable")
        } catch (e: Exception) {
            Outcome.Failed(ErrorCodes.of(e))
        }

    private companion object {
        /** Errors after which the second factor can be tried again, so the join stays pending. */
        val KEEP_JOIN_ON = setOf("syncTwoFactorInvalid", "syncRateLimited", "busy", "emptyPassword")
    }
}
