package io.github.stephanbaum.mosaic.autofill

import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test
import org.w3c.dom.Document
import org.w3c.dom.Element

/**
 * M10-G3 red team (Android): the structure reader, the platform origin of a privileged browser, and what the
 * manifest exposes to other apps. A hole found here was `@Ignore("M10-G3-<n>: ...")` until its fix; the other
 * tests pin defenses that held.
 */
class RedTeamG3Test {
    /** A plain view tree: [field] nodes take a value, the others are containers. */
    private class Tree(
        override val webDomain: String?,
        override val webScheme: String? = null,
        private val field: Boolean = false,
        private val kids: List<Tree> = emptyList(),
        override val visible: Boolean = true,
    ) : ViewSource {
        override val childCount: Int get() = kids.size

        override fun child(index: Int): ViewSource = kids[index]

        override fun raw(shown: Boolean, domain: String?, scheme: String?): RawNode? =
            if (!field) null else RawNode(null, false, shown, domain, scheme, 0, emptyList(), null, emptyList(), null, null)
    }

    private fun walked(root: ViewSource): List<RawNode> {
        val out = ArrayList<RawNode>()
        StructureReader.walk(root, true, null, null, 0, out)
        return out
    }

    // ---- structure reader -------------------------------------------------------------------

    /**
     * Attack (M10-G3-1): a hostile page puts a cross-origin frame (the bank's real login page) before its own
     * fields. The top document's domain sits on a container that takes no value, so the reader drops it and the page
     * root becomes the first fillable node: the bank frame. The bank login is then offered and filled into a
     * cross-origin frame (brief section 4). Defense wanted: the page root is the first node in tree order that
     * reports a domain, container or not (and the core roots the field selection at the target, see the Rust test
     * `g3_1_web_fields_are_rooted_at_the_target_host`).
     */
    @Test
    fun aCrossOriginFrameBeforeThePageFieldsIsNotThePageRoot() {
        val page =
            Tree(
                "evil.example",
                "https",
                kids =
                    listOf(
                        Tree("bank.example", "https", kids = listOf(Tree(null, field = true), Tree(null, field = true))),
                        Tree(null, field = true),
                    ),
            )
        val parsed = ParsedStructure("org.mozilla.firefox", walked(page))
        assertEquals("evil.example", parsed.webDomain)
    }

    /** Attack: a page with more fields than the walk takes. Defense: the walk stops at the cap. */
    @Test
    fun aHugeTreeIsCutAtTheCap() {
        val wide = Tree("bank.example", kids = List(StructureReader.MAX_NODES + 100) { Tree(null, field = true) })
        assertEquals(StructureReader.MAX_NODES, walked(wide).size)
    }

    /**
     * Attack: a foreign frame nested deeper than the depth bound, hoping its fields surface without their domain.
     * Defense: the walk drops the whole subtree; a field never loses the domain of a container above it.
     */
    @Test
    fun aDeepForeignFrameIsDroppedWholeAndNeverLosesItsDomain() {
        var deep = Tree("ads.example", kids = listOf(Tree(null, field = true)))
        repeat(66) { deep = Tree(null, kids = listOf(deep)) }
        val page = Tree("bank.example", kids = listOf(Tree(null, field = true), deep))
        val nodes = walked(page)
        assertEquals(1, nodes.size)
        assertEquals("bank.example", nodes[0].webDomain)
    }

    /** Attack: a field inside a hidden container. Defense: it is reported as not visible, so the core skips it. */
    @Test
    fun aFieldUnderAHiddenContainerIsNotVisible() {
        val page = Tree("bank.example", kids = listOf(Tree(null, visible = false, kids = listOf(Tree(null, field = true)))))
        val nodes = walked(page)
        assertEquals(1, nodes.size)
        assertFalse(nodes[0].visible)
    }

    // ---- platform origin (Credential Manager passwords) ---------------------------------------

    /**
     * Attack: an origin from `getOrigin` that a lenient reader would take for another host (userinfo, an encoded
     * slash before `@`, a backslash, a fragment, a path, an empty query, an opaque or scheme-less form, an encoded or
     * underscored host). Defense: anything but a clean `scheme://host[:port]` gives null, so nothing is offered.
     */
    @Test
    fun platformOriginTricksNeverNameAnotherHost() {
        val backslash = "https://evil.example" + '\\' + "@bank.example"
        for (origin in
            listOf(
                "https://bank.example@evil.example",
                "https://evil.example%2f@bank.example",
                backslash,
                "https://evil.example#@bank.example",
                "https://evil.example/..",
                "https://bank.example:443/",
                "https://bank.example?",
                "https://bank.example#",
                "javascript://bank.example",
                "https:bank.example",
                "//bank.example",
                "https://%62ank.example",
                "https://bank_example.com",
                "https://bücher.example",
            )) {
            assertNull(origin, PlatformOrigin.parse(origin))
        }
        // What passes is the host as given; the core lowercases it and refuses what is not a canonical origin
        // (`g3_web_domain_tricks_from_a_trusted_browser_never_name_another_host` in crates/origin).
        val upper = PlatformOrigin.parse("HTTPS://BANK.Example")
        assertEquals("https", upper!!.scheme)
        assertEquals("BANK.Example", upper.domain)
        assertEquals("[::1]:8443", PlatformOrigin.parse("https://[::1]:8443")!!.domain)
        assertEquals("bank.example.", PlatformOrigin.parse("https://bank.example.")!!.domain)
        assertEquals("bank.example:99999", PlatformOrigin.parse("https://bank.example:99999")!!.domain)
    }

    // ---- manifest exposure ------------------------------------------------------------------

    private val androidNs = "http://schemas.android.com/apk/res/android"

    private fun moduleFile(path: String): File =
        listOf(File(path), File("app/$path"), File("apps/mobile/android/app/$path")).firstOrNull { it.exists() }
            ?: throw AssertionError("$path not found from ${File(".").absolutePath}")

    private fun xml(path: String): Document {
        val factory = DocumentBuilderFactory.newInstance()
        factory.isNamespaceAware = true
        return factory.newDocumentBuilder().parse(moduleFile(path))
    }

    private fun elements(doc: Document, tag: String): List<Element> {
        val list = doc.getElementsByTagName(tag)
        return (0 until list.length).map { list.item(it) as Element }
    }

    private fun Element.android(name: String): String = getAttributeNS(androidNs, name)

    /**
     * Attack: another app starts a provider activity directly (to get values without the platform's tap, to redeem a
     * forged link, to tamper with a fill result), binds a service, or sends the launcher a deep link. Defense: the
     * provider activities are not exported and have no intent filter, the services need the system's bind
     * permission, and the only exported activity takes MAIN/LAUNCHER and nothing else.
     */
    @Test
    fun theProviderActivitiesArePrivateAndTheServicesNeedTheSystemsPermission() {
        val doc = xml("src/main/AndroidManifest.xml")
        val activities = elements(doc, "activity").associateBy { it.android("name") }
        for (name in listOf(".autofill.FillAuthActivity", ".autofill.LinkActivity", ".provider.CredentialActivity")) {
            val activity = activities[name] ?: throw AssertionError("$name is not declared")
            assertEquals(name, "false", activity.android("exported"))
            assertEquals(name, 0, activity.getElementsByTagName("intent-filter").length)
        }
        val exported = activities.values.filter { it.android("exported") != "false" }
        assertEquals(listOf(".ui.MainActivity"), exported.map { it.android("name") })
        val launcher = exported.single()
        assertEquals(
            listOf("android.intent.action.MAIN"),
            (0 until launcher.getElementsByTagName("action").length).map {
                (launcher.getElementsByTagName("action").item(it) as Element).android("name")
            },
        )
        assertEquals(0, launcher.getElementsByTagName("data").length)
        val services = elements(doc, "service").associateBy { it.android("name") }
        assertEquals(
            "android.permission.BIND_AUTOFILL_SERVICE",
            services[".autofill.MosaicAutofillService"]?.android("permission"),
        )
        assertEquals(
            "android.permission.BIND_CREDENTIAL_PROVIDER_SERVICE",
            services[".provider.MosaicCredentialProviderService"]?.android("permission"),
        )
        for (service in services.values) {
            if (service.android("exported") != "false") {
                assertFalse(service.android("name"), service.android("permission").isEmpty())
            }
        }
        for (tag in listOf("provider", "receiver")) {
            for (component in elements(doc, tag)) {
                assertEquals(component.android("name"), "false", component.android("exported"))
            }
        }
    }

    /**
     * Attack: values without a dataset tap through compatibility mode (an accessibility read of a URL bar) or inline
     * suggestions (which a keyboard renders and can tap). Defense: the service declares neither.
     */
    @Test
    fun theAutofillServiceDeclaresNoCompatibilityModeAndNoInlineSuggestions() {
        val doc = xml("src/main/res/xml/autofill_service.xml")
        assertEquals(0, doc.getElementsByTagName("compatibility-package").length)
        for (service in elements(doc, "autofill-service")) {
            assertFalse(service.android("supportsInlineSuggestions") == "true")
        }
    }
}
