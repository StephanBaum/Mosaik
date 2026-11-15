package io.github.stephanbaum.mosaic.security

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class QuickUnlockKeyParamsTest {
    @Test
    fun matchesCryptoSpecPerApiLevel() {
        for (api in listOf(28, 29, 30, 34, 36)) {
            val spec = QuickUnlockKeyParams.spec(api, "mosaic-quick-unlock-00", strongBox = true)
            assertEquals(256, spec.keySizeBits)
            assertEquals("GCM", spec.blockMode)
            assertEquals("NoPadding", spec.padding)
            assertTrue(spec.randomizedEncryptionRequired)
            assertTrue(spec.userAuthenticationRequired)
            assertTrue(spec.invalidatedByBiometricEnrollment)
            assertTrue(spec.unlockedDeviceRequired)
            assertTrue(spec.strongBox)
            val auth = spec.auth
            if (api >= 30) {
                assertTrue("API $api: per-use biometric parameters", auth is QuickUnlockKeyParams.Auth.Parameters)
                val parameters = auth as QuickUnlockKeyParams.Auth.Parameters
                assertEquals(0, parameters.timeoutSeconds)
                assertTrue(parameters.strongBiometricOnly)
            } else {
                assertTrue("API $api: validity duration -1", auth is QuickUnlockKeyParams.Auth.ValidityDuration)
                assertEquals(-1, (auth as QuickUnlockKeyParams.Auth.ValidityDuration).seconds)
            }
        }
        assertEquals(false, QuickUnlockKeyParams.spec(34, "a", strongBox = false).strongBox)
    }

    @Test
    fun bodyIs61BytesWithFlagsIvAndSealedSecret() {
        val iv = ByteArray(12) { it.toByte() }
        val sealed = ByteArray(48) { (100 + it).toByte() }
        val body = QuickUnlockKeyParams.encodeBody(strongBox = true, iv = iv, ctAndTag = sealed)
        assertEquals(61, body.size)
        assertEquals(1, body[0].toInt())
        val decoded = QuickUnlockKeyParams.decodeBody(body)
        assertNotNull(decoded)
        assertTrue(decoded?.strongBox == true)
        assertArrayEquals(iv, decoded?.iv)
        assertArrayEquals(sealed, decoded?.ctAndTag)
        val plain = QuickUnlockKeyParams.decodeBody(QuickUnlockKeyParams.encodeBody(false, iv, sealed))
        assertEquals(false, plain?.strongBox)
    }

    @Test
    fun decodeIsStrict() {
        val good = QuickUnlockKeyParams.encodeBody(false, ByteArray(12), ByteArray(48))
        assertNull(QuickUnlockKeyParams.decodeBody(good.copyOf(60)))
        assertNull(QuickUnlockKeyParams.decodeBody(good + byteArrayOf(0)))
        val unknownFlag = good.copyOf()
        unknownFlag[0] = 0x02
        assertNull(QuickUnlockKeyParams.decodeBody(unknownFlag))
    }
}
