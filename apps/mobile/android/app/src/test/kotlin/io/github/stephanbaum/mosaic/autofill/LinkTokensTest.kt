package io.github.stephanbaum.mosaic.autofill

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LinkTokensTest {
    private var now = 1_000L
    private var counter = 0
    private val tokens = LinkTokens({ now }) { "t" + counter++ }

    @Test
    fun aTokenGivesItsPackageOnce() {
        val token = tokens.issue("com.example.app")
        assertEquals("com.example.app", tokens.redeem(token))
        assertNull(tokens.redeem(token))
    }

    @Test
    fun aTokenExpiresAfterTwoMinutes() {
        val token = tokens.issue("com.example.app")
        now += LinkTokens.TTL_MS - 1
        assertEquals("com.example.app", tokens.redeem(token))
        val late = tokens.issue("com.example.app")
        now += LinkTokens.TTL_MS
        assertNull(tokens.redeem(late))
    }

    @Test
    fun anUnknownOrMissingTokenGivesNothing() {
        tokens.issue("com.example.app")
        assertNull(tokens.redeem("guess"))
        assertNull(tokens.redeem(null))
        assertNull(tokens.redeem(""))
    }

    @Test
    fun eachTokenKeepsItsOwnPackage() {
        val a = tokens.issue("com.example.a")
        val b = tokens.issue("com.example.b")
        assertNotEquals(a, b)
        assertEquals("com.example.b", tokens.redeem(b))
        assertEquals("com.example.a", tokens.redeem(a))
    }

    @Test
    fun theMapIsBounded() {
        val first = tokens.issue("com.example.first")
        for (i in 0 until 40) {
            now += 1
            tokens.issue("com.example.p$i")
        }
        // The oldest entries were dropped to keep the map small.
        assertNull(tokens.redeem(first))
    }

    @Test
    fun theRealTokensAre128RandomBits() {
        val a = LinkTokens.secureToken()
        val b = LinkTokens.secureToken()
        assertEquals(32, a.length)
        assertTrue(a.all { it in "0123456789abcdef" })
        assertNotEquals(a, b)
    }
}
