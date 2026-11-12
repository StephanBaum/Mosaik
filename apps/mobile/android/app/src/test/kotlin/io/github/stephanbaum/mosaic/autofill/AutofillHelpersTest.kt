package io.github.stephanbaum.mosaic.autofill

import io.github.stephanbaum.mosaic.provider.ProviderCaller
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AutofillHelpersTest {
    private fun cert(vararg bytes: Int) = ByteArray(bytes.size) { bytes[it].toByte() }

    private fun node(domain: String?, focused: Boolean = false) =
        RawNode(
            autofillId = null,
            focused = focused,
            visible = true,
            webDomain = domain,
            webScheme = null,
            inputType = 0,
            autofillHints = emptyList(),
            htmlTag = null,
            htmlAttrs = emptyList(),
            idEntry = null,
            hint = null,
        )

    // ---- Signers --------------------------------------------------------------------------------

    @Test
    fun oneSignerUsesOnlyTheNewestCertificateOfTheHistory() {
        val chosen = Signers.choose(false, listOf(cert(9)), listOf(cert(1), cert(2), cert(3)))
        assertEquals(1, chosen.size)
        assertArrayEquals(cert(3), chosen[0])
    }

    @Test
    fun severalSignersAreHandedOverAsTheyAreSoTheCoreCanRefuseThem() {
        val chosen = Signers.choose(true, listOf(cert(1), cert(2)), listOf(cert(7)))
        assertEquals(2, chosen.size)
        assertArrayEquals(cert(1), chosen[0])
        assertArrayEquals(cert(2), chosen[1])
    }

    @Test
    fun noHistoryAndOneSignerMeansNoSigner() {
        assertTrue(Signers.choose(false, listOf(cert(1)), emptyList()).isEmpty())
    }

    @Test
    fun theDigestIsSha256() {
        // SHA-256 of the empty input.
        val expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        assertEquals(expected, Signers.digest(ByteArray(0)).joinToString("") { "%02x".format(it.toInt() and 0xFF) })
    }

    // ---- PlatformOrigin -------------------------------------------------------------------------

    @Test
    fun aCleanOriginBecomesDomainAndScheme() {
        val parts = PlatformOrigin.parse("https://github.com")
        assertNotNull(parts)
        assertEquals("github.com", parts!!.domain)
        assertEquals("https", parts.scheme)
    }

    @Test
    fun anExplicitPortStaysPartOfTheDomain() {
        val parts = PlatformOrigin.parse("https://example.com:8443")
        assertEquals("example.com:8443", parts!!.domain)
    }

    @Test
    fun anythingButACleanOriginIsRefused() {
        assertNull(PlatformOrigin.parse(null))
        assertNull(PlatformOrigin.parse("https://github.com/login"))
        assertNull(PlatformOrigin.parse("https://github.com?x=1"))
        assertNull(PlatformOrigin.parse("https://user@github.com"))
        assertNull(PlatformOrigin.parse("ftp://github.com"))
        assertNull(PlatformOrigin.parse("android:apk-key-hash:AAAA"))
        assertNull(PlatformOrigin.parse("not an origin"))
    }

    // ---- web root -------------------------------------------------------------------------------

    @Test
    fun theBrowsersOwnScreenBeforeThePageIsDropped() {
        val trimmed = StructureReader.trimToWebRoot(listOf(node(null), node(null), node("a.example"), node("a.example")))
        assertEquals(2, trimmed.size)
        assertEquals("a.example", trimmed[0].webDomain)
    }

    @Test
    fun aNativeScreenStaysWhole() {
        assertEquals(3, StructureReader.trimToWebRoot(listOf(node(null), node(null), node(null))).size)
    }

    @Test
    fun aTreeThatStartsOnThePageIsNotChanged() {
        val nodes = listOf(node("a.example"), node("b.example", focused = true))
        assertEquals(2, StructureReader.trimToWebRoot(nodes).size)
    }

    // ---- provider caller ------------------------------------------------------------------------

    @Test
    fun aBrowsersOriginBecomesAWebCallerForPasswords() {
        val caller = ProviderCaller("com.example.browser", listOf(cert(1)), "https://github.com")
        val android = caller.forPasswords()
        assertEquals("github.com", android!!.webDomain)
        assertEquals("https", android.webScheme)
    }

    @Test
    fun anAppWithoutAnOriginHasNoWebDomain() {
        val android = ProviderCaller("com.example.app", listOf(cert(1)), null).forPasswords()
        assertNull(android!!.webDomain)
        assertNull(android.webScheme)
    }

    @Test
    fun anOriginThatIsNotCleanOffersNoPasswords() {
        assertNull(ProviderCaller("com.example.browser", listOf(cert(1)), "https://github.com/x").forPasswords())
    }

    @Test
    fun passkeyCallersNeverCarryAWebDomain() {
        val android = ProviderCaller("com.example.browser", listOf(cert(1)), "https://github.com").forPasskeys()
        assertNull(android.webDomain)
        assertEquals(1, android.currentSigners.size)
    }
}
