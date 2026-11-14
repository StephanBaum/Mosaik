package io.github.stephanbaum.mosaic.session

import io.github.stephanbaum.mosaic.core.CoreHolder
import io.github.stephanbaum.mosaic.core.SecretBytes
import io.github.stephanbaum.mosaic.ffi.AndroidCaller
import io.github.stephanbaum.mosaic.ffi.AssetLinksStatus
import io.github.stephanbaum.mosaic.ffi.GeneratedSecret
import io.github.stephanbaum.mosaic.ffi.GeneratorOptions
import io.github.stephanbaum.mosaic.ffi.ItemDetail
import io.github.stephanbaum.mosaic.ffi.ItemFilter
import io.github.stephanbaum.mosaic.ffi.ItemKey
import io.github.stephanbaum.mosaic.ffi.ItemSummary
import io.github.stephanbaum.mosaic.ffi.LoginDraft
import io.github.stephanbaum.mosaic.ffi.LumaFrame
import io.github.stephanbaum.mosaic.ffi.MobileSettings
import io.github.stephanbaum.mosaic.ffi.NoteDraft
import io.github.stephanbaum.mosaic.ffi.OtpCode
import io.github.stephanbaum.mosaic.ffi.OtpPreview
import io.github.stephanbaum.mosaic.ffi.SecretValue
import io.github.stephanbaum.mosaic.ffi.SettingsUpdate
import io.github.stephanbaum.mosaic.ffi.SyncOverview
import io.github.stephanbaum.mosaic.ffi.TrashMode
import io.github.stephanbaum.mosaic.ffi.VaultSummary
import io.github.stephanbaum.mosaic.security.ErrorCodes
import io.github.stephanbaum.mosaic.security.SecureStorageFailure
import kotlinx.coroutines.CancellationException

/**
 * The vault operations of the signed-in screens. Every call goes through [CoreHolder] (off the main thread);
 * secrets travel as [SecretBytes] (wiped when the call is over) or `SecretValue`, and a failure is an error
 * code of the core, never a message.
 */
class VaultController(
    private val holder: CoreHolder,
    private val onSettings: (MobileSettings) -> Unit,
) {
    suspend fun items(): Outcome<List<ItemSummary>> =
        guarded {
            holder.call {
                it.items(ItemFilter(vaultId = null, trash = TrashMode.EXCLUDE, favoritesOnly = false, kind = null))
            }
        }

    suspend fun search(query: String): Outcome<List<ItemSummary>> = guarded { holder.call { it.search(query) } }

    suspend fun vaults(): Outcome<List<VaultSummary>> = guarded { holder.call { it.vaults() } }

    suspend fun item(vaultId: String, itemId: String): Outcome<ItemDetail> =
        guarded { holder.call { it.item(vaultId, itemId) } }

    /** The caller wipes the returned value. */
    suspend fun reveal(vaultId: String, itemId: String, field: String): Outcome<SecretValue> =
        guarded { holder.call { it.reveal(vaultId, itemId, field) } }

    /** The caller wipes the code in the returned value. */
    suspend fun otpCode(vaultId: String, itemId: String): Outcome<OtpCode> =
        guarded { holder.call { it.otpCode(vaultId, itemId) } }

    /** Saves a login. [password] and [notes] are wiped here; null keeps the stored value. */
    suspend fun saveLogin(draft: LoginDraft, password: SecretBytes?, notes: SecretBytes?): Outcome<ItemKey> =
        guarded {
            try {
                holder.call { core ->
                    withSecret(password) { pw ->
                        withSecret(notes) { nt -> core.saveLogin(draft.copy(password = pw, notes = nt)) }
                    }
                }
            } finally {
                password?.wipe()
                notes?.wipe()
            }
        }

    suspend fun saveNote(draft: NoteDraft, notes: SecretBytes?): Outcome<ItemKey> =
        guarded {
            try {
                holder.call { core -> withSecret(notes) { nt -> core.saveNote(draft.copy(notes = nt)) } }
            } finally {
                notes?.wipe()
            }
        }

    suspend fun trash(vaultId: String, itemId: String): Outcome<Unit> =
        guarded { holder.call { it.trashItem(vaultId, itemId) } }

    suspend fun setFavorite(vaultId: String, itemId: String, favorite: Boolean): Outcome<Unit> =
        guarded { holder.call { it.setFavorite(vaultId, itemId, favorite) } }

    /** The caller wipes the generated value. */
    suspend fun generate(options: GeneratorOptions): Outcome<GeneratedSecret> =
        guarded { holder.call { it.generatePassword(options) } }

    suspend fun otpFromText(text: SecretBytes): Outcome<OtpPreview> =
        guarded {
            try {
                holder.call { core -> text.consume { core.otpFromText(it) } }
            } finally {
                text.wipe()
            }
        }

    /**
     * Looks for an `otpauth://` QR code in one camera frame (the scanner of the login editor). Null: no code in
     * this frame. A found code waits in the core as a pending setup, like a typed one. The frame is wiped by the
     * scanner afterwards.
     */
    suspend fun scanOtpQr(frame: LumaFrame): Outcome<OtpPreview?> = guarded { holder.call { it.scanOtpQr(frame) } }

    suspend fun assetLinksStatus(vaultId: String, itemId: String, app: AndroidCaller): Outcome<AssetLinksStatus> =
        guarded { holder.call { it.assetLinksStatus(vaultId, itemId, app) } }

    suspend fun linkApp(vaultId: String, itemId: String, app: AndroidCaller): Outcome<Unit> =
        guarded { holder.call { it.linkAndroidApp(vaultId, itemId, app) } }

    suspend fun syncOverview(): Outcome<SyncOverview> = guarded { holder.call { it.syncOverview() } }

    suspend fun syncNow(): Outcome<SyncOverview> = guarded { holder.call { it.syncNow() } }

    /** The user checked the sync findings: clears them. The caller syncs afterwards. */
    suspend fun acknowledgeSyncFindings(): Outcome<SyncOverview> =
        guarded { holder.call { it.acknowledgeSyncFindings() } }

    suspend fun settings(): Outcome<MobileSettings> =
        guarded {
            val settings = holder.call { it.settings() }
            onSettings(settings)
            settings
        }

    suspend fun updateSettings(update: SettingsUpdate): Outcome<MobileSettings> =
        guarded {
            val settings = holder.call { it.updateSettings(update) }
            onSettings(settings)
            settings
        }

    private fun <T> withSecret(secret: SecretBytes?, block: (ByteArray?) -> T): T =
        if (secret == null) block(null) else secret.consume { block(it) }

    private suspend fun <T> guarded(block: suspend () -> T): Outcome<T> =
        try {
            Outcome.Ok(block())
        } catch (e: CancellationException) {
            throw e
        } catch (e: SecureStorageFailure) {
            Outcome.Failed("storageUnavailable")
        } catch (e: Exception) {
            Outcome.Failed(ErrorCodes.of(e))
        }
}
