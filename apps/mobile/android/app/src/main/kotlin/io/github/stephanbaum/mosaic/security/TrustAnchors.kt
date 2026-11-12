package io.github.stephanbaum.mosaic.security

import java.security.KeyStore
import java.security.cert.X509Certificate
import javax.net.ssl.TrustManagerFactory
import javax.net.ssl.X509TrustManager

/**
 * The TLS trust anchors handed to the Rust core (CRYPTO_SPEC 6.9.6, ADR-0091). They come from the platform's
 * default trust manager, which honours `res/xml/network_security_config.xml` (system CAs only, no user CAs).
 * `AndroidCAStore` is deliberately not used as the source.
 */
object TrustAnchors {
    fun collect(): List<X509Certificate> {
        val factory = TrustManagerFactory.getInstance(TrustManagerFactory.getDefaultAlgorithm())
        factory.init(null as KeyStore?)
        return factory.trustManagers
            .filterIsInstance<X509TrustManager>()
            .flatMap { it.acceptedIssuers.asList() }
    }

    /** The anchors as DER, the form of `CoreConfig.trustAnchorsDer`. */
    fun collectDer(): List<ByteArray> = collect().map { it.encoded }
}
