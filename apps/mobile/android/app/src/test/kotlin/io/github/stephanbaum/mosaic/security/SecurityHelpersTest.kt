package io.github.stephanbaum.mosaic.security

import io.github.stephanbaum.mosaic.ui.l10n.L10n
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SecurityHelpersTest {
    @Test
    fun bootIdIsTheBootCountAsBigEndianU32() {
        assertArrayEquals(byteArrayOf(0x01, 0x02, 0x03, 0x04), BootId.fromBootCount(0x01020304))
        assertEquals(4, BootId.fromBootCount(7).size)
    }

    @Test
    fun memoryCheckRefusesWhenTheShortfallIsKnown() {
        assertTrue(MemoryCheck.isEnough(null, 0L))
        assertTrue(MemoryCheck.isEnough(100UL, 100L))
        assertFalse(MemoryCheck.isEnough(101UL, 100L))
        assertFalse(MemoryCheck.isEnough(1UL, -1L))
    }

    @Test
    fun variantNamesBecomeErrorCodes() {
        assertEquals("wrongCredentials", ErrorCodes.fromVariantName("WrongCredentials"))
        assertEquals("internal", ErrorCodes.fromVariantName("NoSuchVariant"))
        for (code in L10n.errorCodes) {
            assertEquals(code, ErrorCodes.fromVariantName(code.replaceFirstChar { it.uppercaseChar() }))
        }
        assertEquals("internal", ErrorCodes.of(IllegalStateException()))
    }

    @Test
    fun secureStoreAadHasTheLayoutOfCryptoSpec691() {
        // The vector of the Rust test `secure_store_aad_has_the_fixed_vector` (M10-G2 L1).
        val expected =
            "016d6f736169632f76312f6d6f62696c652f7365637572652d73746f7265000000000f7365637265742d6b65792f30313233"
        val actual = AndroidSecureStorage.aadFor("secret-key/0123").joinToString("") { "%02x".format(it.toInt() and 0xFF) }
        assertEquals(expected, actual)
    }

    @Test
    fun fileNamesAreTheSha256OfTheName() {
        assertEquals(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            AndroidSecureStorage.sha256Hex("abc"),
        )
    }
}
