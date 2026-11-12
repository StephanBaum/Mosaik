package io.github.stephanbaum.mosaic.security

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.security.KeyStore
import java.util.UUID
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/** The secure storage on a real Keystore: round trip, delete, and the binding of a file to its name (CRYPTO_SPEC 6.9.1). */
@RunWith(AndroidJUnit4::class)
class SecureStorageInstrumentedTest {
    private val alias = "mosaic-secure-store-test-" + UUID.randomUUID()
    private lateinit var directory: File
    private lateinit var storage: AndroidSecureStorage

    @Before
    fun setUp() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        directory = File(context.noBackupFilesDir, "secure-test-" + UUID.randomUUID())
        storage = AndroidSecureStorage(directory, alias)
    }

    @After
    fun tearDown() {
        directory.deleteRecursively()
        val store = KeyStore.getInstance("AndroidKeyStore")
        store.load(null)
        if (store.containsAlias(alias)) store.deleteEntry(alias)
    }

    private fun assertRefused(name: String) {
        try {
            storage.get(name)
            fail("a file that fails its tag must be refused, not read")
        } catch (e: SecureStorageFailure) {
            // expected: the core turns this into storageUnavailable
        }
    }

    @Test
    fun roundTripDeleteAndAadBinding() {
        val name = "secret-key/00112233445566778899aabbccddeeff"
        val secret = ByteArray(16) { (it * 7).toByte() }

        // Missing is absent, and contains does not decrypt anything.
        assertNull(storage.get(name))
        assertFalse(storage.contains(name))

        storage.put(name, secret)
        assertTrue(storage.contains(name))
        assertArrayEquals(secret, storage.get(name))
        val file = storage.fileFor(name)
        val raw = file.readBytes()
        assertEquals("version byte", 0x01, raw[0].toInt())
        assertEquals("version, iv, ciphertext, tag", 1 + 12 + secret.size + 16, raw.size)

        // Each put draws a new IV and replaces the value.
        val replacement = ByteArray(32) { 9 }
        storage.put(name, replacement)
        assertArrayEquals(replacement, storage.get(name))
        assertFalse("a new IV for every write", raw.copyOfRange(1, 13).contentEquals(file.readBytes().copyOfRange(1, 13)))

        // AAD binding: the same bytes under another name must not open.
        val other = "sync-token/00112233445566778899aabbccddeeff"
        storage.fileFor(other).writeBytes(file.readBytes())
        assertTrue("a moved file counts as present", storage.contains(other))
        assertRefused(other)

        // A wrong version byte is refused, never treated as absent.
        val tampered = file.readBytes()
        tampered[0] = 0x02
        file.writeBytes(tampered)
        assertTrue(storage.contains(name))
        assertRefused(name)

        // A flipped ciphertext bit fails the tag.
        val flipped = storage.fileFor("flip").also { storage.put("flip", secret) }.readBytes()
        flipped[flipped.size - 1] = (flipped[flipped.size - 1].toInt() xor 0x01).toByte()
        storage.fileFor("flip").writeBytes(flipped)
        assertRefused("flip")

        // Delete removes the value, and deleting a missing value is not an error.
        storage.delete(name)
        assertFalse(storage.contains(name))
        assertNull(storage.get(name))
        storage.delete(name)
    }
}
