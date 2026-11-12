package io.github.stephanbaum.mosaic.security

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import io.github.stephanbaum.mosaic.ffi.SecureStorage
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.nio.ByteBuffer
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.GeneralSecurityException
import java.security.KeyStore
import java.security.MessageDigest
import java.security.ProviderException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** The platform storage failed. The core maps any exception from a storage call to `storageUnavailable`. */
class SecureStorageFailure : Exception()

/**
 * The secure storage of the Secret Key and the session token (CRYPTO_SPEC 6.9.1, ADR-0090). Rust decides the
 * names and the lifetimes; this class only puts, gets, deletes and tests for presence.
 *
 * One Keystore key (`mosaic-secure-store`): AES-256-GCM, no user authentication, device must be unlocked,
 * StrongBox when available. The key is created only on a phone with a secure lock screen and never without the
 * unlocked-device flag (ADR-0090 amendment "device lock required"): without a lock screen a write fails with
 * [SecureStorageFailure], and the screens show "set a screen lock first" before that happens. Each value is the
 * file `<directory>/<hex(SHA-256(name))>.bin`: `0x01 || iv[12] || ct || tag[16]`, with the AAD of [aadFor]. A
 * file with another version, a wrong length or a failing tag is refused, never treated as absent. Writes are
 * atomic.
 */
class AndroidSecureStorage(
    private val directory: File,
    private val deviceSecure: () -> Boolean,
    private val keyAlias: String = KEY_ALIAS,
) : SecureStorage {
    private val lock = Any()
    private var cachedKey: SecretKey? = null

    override fun get(name: String): ByteArray? =
        synchronized(lock) {
            val file = fileFor(name)
            if (!file.exists()) {
                null
            } else {
                try {
                    decrypt(name, file.readBytes())
                } catch (e: GeneralSecurityException) {
                    throw SecureStorageFailure()
                } catch (e: IOException) {
                    throw SecureStorageFailure()
                } catch (e: ProviderException) {
                    throw SecureStorageFailure()
                }
            }
        }

    /** Presence only: a corrupt or wrong-version file counts as present, and nothing is decrypted. */
    override fun contains(name: String): Boolean = synchronized(lock) { fileFor(name).exists() }

    override fun put(name: String, value: ByteArray) {
        synchronized(lock) {
            var temp: File? = null
            try {
                check(directory.isDirectory || directory.mkdirs()) { "no secure directory" }
                val data = encrypt(name, value)
                val written = File.createTempFile("write", ".part", directory)
                temp = written
                FileOutputStream(written).use { out ->
                    out.write(data)
                    out.flush()
                    out.fd.sync()
                }
                Files.move(
                    written.toPath(),
                    fileFor(name).toPath(),
                    StandardCopyOption.ATOMIC_MOVE,
                    StandardCopyOption.REPLACE_EXISTING,
                )
                temp = null
            } catch (e: GeneralSecurityException) {
                throw SecureStorageFailure()
            } catch (e: IOException) {
                throw SecureStorageFailure()
            } catch (e: IllegalStateException) {
                throw SecureStorageFailure()
            } catch (e: ProviderException) {
                throw SecureStorageFailure()
            } finally {
                temp?.delete()
            }
        }
    }

    override fun delete(name: String) {
        synchronized(lock) {
            try {
                Files.deleteIfExists(fileFor(name).toPath())
            } catch (e: IOException) {
                throw SecureStorageFailure()
            }
        }
    }

    /** The file of [name]; public so the instrumented test can move and corrupt files. */
    fun fileFor(name: String): File = File(directory, sha256Hex(name) + ".bin")

    private fun encrypt(name: String, value: ByteArray): ByteArray {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key())
        cipher.updateAAD(aadFor(name))
        val iv = cipher.iv
        val sealed = cipher.doFinal(value)
        return byteArrayOf(VERSION) + iv + sealed
    }

    private fun decrypt(name: String, raw: ByteArray): ByteArray {
        if (raw.size < 1 + IV_LEN + TAG_LEN || raw[0] != VERSION) throw SecureStorageFailure()
        val iv = raw.copyOfRange(1, 1 + IV_LEN)
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(TAG_BITS, iv))
        cipher.updateAAD(aadFor(name))
        return cipher.doFinal(raw, 1 + IV_LEN, raw.size - 1 - IV_LEN)
    }

    private fun key(): SecretKey {
        cachedKey?.let { return it }
        val store = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        val existing = store.getKey(keyAlias, null)
        val key = if (existing is SecretKey) existing else generateKey()
        cachedKey = key
        return key
    }

    /** StrongBox first, then the TEE. Never without the unlocked-device flag, and never without a lock screen. */
    private fun generateKey(): SecretKey {
        if (!deviceSecure()) throw SecureStorageFailure()
        var last: Exception? = null
        for (strongBox in listOf(true, false)) {
            try {
                return generate(strongBox)
            } catch (e: ProviderException) {
                last = e
            } catch (e: GeneralSecurityException) {
                last = e
            }
        }
        throw last ?: SecureStorageFailure()
    }

    private fun generate(strongBox: Boolean): SecretKey {
        val spec =
            KeyGenParameterSpec.Builder(keyAlias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setKeySize(KEY_BITS)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .setUnlockedDeviceRequired(true)
                .setIsStrongBoxBacked(strongBox)
                .build()
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        generator.init(spec)
        return generator.generateKey()
    }

    companion object {
        const val KEY_ALIAS = "mosaic-secure-store"
        const val VERSION: Byte = 0x01
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val KEY_BITS = 256
        private const val IV_LEN = 12
        private const val TAG_LEN = 16
        private const val TAG_BITS = 128

        /** `ctx("mobile/secure-store")` of the core: `"mosaic/v1/" || label` (CRYPTO_SPEC 3). */
        private val CONTEXT = "mosaic/v1/mobile/secure-store".toByteArray(Charsets.US_ASCII)

        /**
         * The AAD of CRYPTO_SPEC 6.9.1: `0x01 || ctx(label) || 0x00 || u32_be(len(name)) || name`, the same bytes as
         * `secure_store_aad(name)` in `mosaic-mobile`. The binding does not export that function, so the layout is
         * built here and pinned by SecureStorageInstrumentedTest.
         */
        fun aadFor(name: String): ByteArray {
            val nameBytes = name.toByteArray(Charsets.UTF_8)
            return ByteBuffer.allocate(1 + CONTEXT.size + 1 + Int.SIZE_BYTES + nameBytes.size)
                .put(VERSION)
                .put(CONTEXT)
                .put(0.toByte())
                .putInt(nameBytes.size)
                .put(nameBytes)
                .array()
        }

        fun sha256Hex(name: String): String =
            MessageDigest.getInstance("SHA-256")
                .digest(name.toByteArray(Charsets.UTF_8))
                .joinToString("") { "%02x".format(it.toInt() and 0xFF) }
    }
}
