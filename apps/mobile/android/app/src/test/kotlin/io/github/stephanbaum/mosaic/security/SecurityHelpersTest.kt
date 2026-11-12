package io.github.stephanbaum.mosaic.security

import io.github.stephanbaum.mosaic.ui.l10n.L10n
import org.junit.Assert.assertArrayEquals
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
        val name = "secret-key/0123"
        val expected =
            byteArrayOf(0x01) +
                "mosaic/v1/mobile/secure-store".toByteArray(Charsets.US_ASCII) +
                byteArrayOf(0x00, 0x00, 0x00, 0x00, 0x0F) +
                name.toByteArray(Charsets.UTF_8)
        assertArrayEquals(expected, AndroidSecureStorage.aadFor(name))
    }

    @Test
    fun fileNamesAreTheSha256OfTheName() {
        assertEquals(
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            AndroidSecureStorage.sha256Hex("abc"),
        )
    }
}
