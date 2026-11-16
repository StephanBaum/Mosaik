package io.github.stephanbaum.mosaic.session

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class OAuthCallbackTest {
    private val base = "io.github.stephanbaum.mosaic.oauth:/cb?"

    @Test
    fun theExactCallbackIsAccepted() {
        assertTrue(OAuthCallback.accepts(base + "code=abc&state=xyz"))
    }

    @Test
    fun otherPathsSchemesAndShapesAreRejected() {
        assertFalse(OAuthCallback.accepts("io.github.stephanbaum.mosaic.oauth:/cb2?x"))
        assertFalse(OAuthCallback.accepts("io.github.stephanbaum.mosaic.oauth:/cb"))
        assertFalse(OAuthCallback.accepts("io.github.stephanbaum.mosaic.oauth://cb?code=1"))
        assertFalse(OAuthCallback.accepts("https://example.com/cb?code=1"))
        assertFalse(OAuthCallback.accepts("other.scheme:/cb?code=1"))
        assertFalse(OAuthCallback.accepts(""))
    }

    @Test
    fun theLengthIsCappedAt4096Characters() {
        val atLimit = base + "a".repeat(OAuthCallback.MAX_LENGTH - base.length)
        assertTrue(atLimit.length == OAuthCallback.MAX_LENGTH)
        assertTrue(OAuthCallback.accepts(atLimit))
        assertFalse(OAuthCallback.accepts(atLimit + "a"))
    }
}
