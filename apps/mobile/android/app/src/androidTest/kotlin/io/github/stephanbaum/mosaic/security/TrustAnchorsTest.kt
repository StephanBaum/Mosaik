package io.github.stephanbaum.mosaic.security

import androidx.test.ext.junit.runners.AndroidJUnit4
import java.security.KeyStore
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** The anchors given to the core are the system CAs (CRYPTO_SPEC 6.9.6): a public root is in, user CAs are out. */
@RunWith(AndroidJUnit4::class)
class TrustAnchorsTest {
    @Test
    fun containsIsrgRootX1AndNoUserCas() {
        val anchors = TrustAnchors.collect()
        assertTrue("no anchors at all", anchors.isNotEmpty())
        assertTrue(
            "ISRG Root X1 (Let's Encrypt) is missing",
            anchors.any { it.subjectX500Principal.name.contains("ISRG Root X1") },
        )

        // AndroidCAStore lists user-installed CAs with the alias prefix "user:"; none may be an anchor.
        val store = KeyStore.getInstance("AndroidCAStore")
        store.load(null)
        val derOfAnchors = anchors.map { it.encoded.toList() }.toSet()
        for (alias in store.aliases().toList().filter { it.startsWith("user:") }) {
            val user = store.getCertificate(alias).encoded.toList()
            assertFalse("user CA $alias is trusted", user in derOfAnchors)
        }
        assertTrue(TrustAnchors.collectDer().size == anchors.size)
    }
}
