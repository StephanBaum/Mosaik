package io.github.stephanbaum.mosaic.security

import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * The quick-unlock key and body of CRYPTO_SPEC 6.2 (Android Keystore row, platform byte 0x04). The decisions
 * live here as plain data so a JVM test can check them per API level.
 */
object QuickUnlockKeyParams {
    const val KEY_SIZE_BITS = 256
    const val BLOCK_MODE_GCM = "GCM"
    const val PADDING_NONE = "NoPadding"
    const val TAG_BITS = 128
    const val IV_LEN = 12
    const val SECRET_LEN = 32
    const val TAG_LEN = 16

    /** `flags (1) || iv (12) || ct (32) || tag (16)`. */
    const val BODY_LEN = 1 + IV_LEN + SECRET_LEN + TAG_LEN
    private const val FLAG_STRONGBOX = 0x01

    /** How the key demands the user: per use, Class 3 biometric only. */
    sealed interface Auth {
        /** API 30+: `setUserAuthenticationParameters(0, AUTH_BIOMETRIC_STRONG)`. */
        class Parameters(val timeoutSeconds: Int, val strongBiometricOnly: Boolean) : Auth

        /** API 28 to 29: `setUserAuthenticationValidityDurationSeconds(-1)`, biometric CryptoObject only. */
        class ValidityDuration(val seconds: Int) : Auth
    }

    class Spec(
        val alias: String,
        val keySizeBits: Int,
        val blockMode: String,
        val padding: String,
        val randomizedEncryptionRequired: Boolean,
        val userAuthenticationRequired: Boolean,
        val auth: Auth,
        val invalidatedByBiometricEnrollment: Boolean,
        val unlockedDeviceRequired: Boolean,
        val strongBox: Boolean,
    )

    /** The key parameters of the spec for [apiLevel]. */
    fun spec(apiLevel: Int, alias: String, strongBox: Boolean): Spec =
        Spec(
            alias = alias,
            keySizeBits = KEY_SIZE_BITS,
            blockMode = BLOCK_MODE_GCM,
            padding = PADDING_NONE,
            randomizedEncryptionRequired = true,
            userAuthenticationRequired = true,
            auth =
                if (apiLevel >= Build.VERSION_CODES.R) {
                    Auth.Parameters(timeoutSeconds = 0, strongBiometricOnly = true)
                } else {
                    Auth.ValidityDuration(seconds = -1)
                },
            invalidatedByBiometricEnrollment = true,
            unlockedDeviceRequired = true,
            strongBox = strongBox,
        )

    class Body(val strongBox: Boolean, val iv: ByteArray, val ctAndTag: ByteArray)

    fun encodeBody(strongBox: Boolean, iv: ByteArray, ctAndTag: ByteArray): ByteArray {
        require(iv.size == IV_LEN && ctAndTag.size == SECRET_LEN + TAG_LEN) { "body parts have the wrong length" }
        val flags = if (strongBox) FLAG_STRONGBOX else 0
        return byteArrayOf(flags.toByte()) + iv + ctAndTag
    }

    /** Strict: the exact length, and no flag bit other than StrongBox. Null for anything else. */
    fun decodeBody(body: ByteArray): Body? {
        if (body.size != BODY_LEN) return null
        val flags = body[0].toInt() and 0xFF
        if (flags and FLAG_STRONGBOX.inv() != 0) return null
        return Body(
            strongBox = flags and FLAG_STRONGBOX != 0,
            iv = body.copyOfRange(1, 1 + IV_LEN),
            ctAndTag = body.copyOfRange(1 + IV_LEN, BODY_LEN),
        )
    }
}

/** The Keystore key is gone (cleared by the OS or never created): quick unlock cannot continue. */
class QuickUnlockKeyMissingException : Exception()

/** The Keystore part of quick unlock (docs/MOBILE.md, ADR-0090). Rust names the alias; this only holds the key. */
class QuickUnlockKeys {
    /** A cipher in encrypt mode on a new key, to be handed to the biometric prompt. */
    class Prepared(val cipher: Cipher, val strongBox: Boolean)

    /** Creates the key (StrongBox when available) and a cipher for it. */
    fun prepareEnroll(alias: String): Prepared {
        var strongBox = true
        val key =
            try {
                generate(alias, strongBox = true)
            } catch (e: StrongBoxUnavailableException) {
                strongBox = false
                generate(alias, strongBox = false)
            }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        return Prepared(cipher, strongBox)
    }

    /**
     * A cipher in decrypt mode for [iv]. Throws `KeyPermanentlyInvalidatedException` after a biometric
     * enrolment change, and [QuickUnlockKeyMissingException] when the key is gone.
     */
    fun prepareDecrypt(alias: String, iv: ByteArray): Cipher {
        val key = load(alias) ?: throw QuickUnlockKeyMissingException()
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(QuickUnlockKeyParams.TAG_BITS, iv))
        return cipher
    }

    /** Deletes the key, if it exists. */
    fun delete(alias: String) {
        val store = keyStore()
        if (store.containsAlias(alias)) store.deleteEntry(alias)
    }

    /** The key itself, for the instrumented test that reads its `KeyInfo`. */
    fun load(alias: String): SecretKey? = keyStore().getKey(alias, null) as? SecretKey

    @Suppress("DEPRECATION")
    private fun generate(alias: String, strongBox: Boolean): SecretKey {
        val spec = QuickUnlockKeyParams.spec(Build.VERSION.SDK_INT, alias, strongBox)
        val builder =
            KeyGenParameterSpec.Builder(spec.alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setKeySize(spec.keySizeBits)
                .setBlockModes(spec.blockMode)
                .setEncryptionPaddings(spec.padding)
                .setRandomizedEncryptionRequired(spec.randomizedEncryptionRequired)
                .setUserAuthenticationRequired(spec.userAuthenticationRequired)
                .setInvalidatedByBiometricEnrollment(spec.invalidatedByBiometricEnrollment)
                .setUnlockedDeviceRequired(spec.unlockedDeviceRequired)
                .setIsStrongBoxBacked(spec.strongBox)
        when (val auth = spec.auth) {
            is QuickUnlockKeyParams.Auth.Parameters ->
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    builder.setUserAuthenticationParameters(auth.timeoutSeconds, KeyProperties.AUTH_BIOMETRIC_STRONG)
                }
            is QuickUnlockKeyParams.Auth.ValidityDuration ->
                builder.setUserAuthenticationValidityDurationSeconds(auth.seconds)
        }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        generator.init(builder.build())
        return generator.generateKey()
    }

    private fun keyStore(): KeyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }

    private companion object {
        const val ANDROID_KEYSTORE = "AndroidKeyStore"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
    }
}
