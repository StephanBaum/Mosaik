package io.github.stephanbaum.mosaic.ui

import io.github.stephanbaum.mosaic.ui.link.certificateDigest
import io.github.stephanbaum.mosaic.ui.link.shortFingerprint
import io.github.stephanbaum.mosaic.ui.vault.appUrlName
import io.github.stephanbaum.mosaic.ui.vault.groupCode
import org.junit.Assert.assertEquals
import org.junit.Test

/** The pure helpers of the vault screens. */
class VaultHelpersTest {
    @Test
    fun codesAreGroupedForReading() {
        assertEquals("123 456", groupCode("123456"))
        assertEquals("1234 5678", groupCode("12345678"))
        assertEquals("12345", groupCode("12345"))
    }

    @Test
    fun appUrlsShowThePackage() {
        assertEquals("com.example.app", appUrlName("androidapp://com.example.app?cert=AAAA"))
        assertEquals("com.example.app", appUrlName("androidapp://com.example.app"))
    }

    @Test
    fun fingerprintShowsTheFirstEightBytes() {
        val digest = certificateDigest(byteArrayOf())
        // SHA-256 of the empty input starts with e3 b0 c4 42 98 fc 1c 14.
        assertEquals("E3:B0:C4:42:98:FC:1C:14", shortFingerprint(digest))
    }
}
