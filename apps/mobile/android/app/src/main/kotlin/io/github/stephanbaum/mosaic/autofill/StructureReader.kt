package io.github.stephanbaum.mosaic.autofill

import android.app.assist.AssistStructure
import android.view.View
import android.view.autofill.AutofillId
import io.github.stephanbaum.mosaic.ffi.Attr
import io.github.stephanbaum.mosaic.ffi.FieldNode

/** One field of the view tree as read from the platform: plain values, no logic. [autofillId] answers a fill. */
class RawNode(
    val autofillId: AutofillId?,
    val focused: Boolean,
    val visible: Boolean,
    val webDomain: String?,
    val webScheme: String?,
    val inputType: Int,
    val autofillHints: List<String>,
    val htmlTag: String?,
    val htmlAttrs: List<Pair<String, String>>,
    val idEntry: String?,
    val hint: String?,
    /**
     * The domain of the first node in tree order that reports one, container or not, set by the walk on every field
     * it reads; null before that node. A container that carries the top document's domain takes no value, so without
     * this a cross-origin frame placed before the page's own fields would become the page root (M10-G3-1).
     */
    val pageDomain: String? = null,
    /** The scheme that same node reports. */
    val pageScheme: String? = null,
)

/** The first domain a walk meets, shared by every window of one structure. */
class PageRoot {
    var domain: String? = null
        private set
    var scheme: String? = null
        private set
    private var found = false

    /** Records [domain] and [scheme] when this is the first node that reports a domain. */
    fun see(domain: String?, scheme: String?) {
        if (found || domain == null) return
        found = true
        this.domain = domain
        this.scheme = scheme
    }
}

/**
 * One node of a view tree, as far as the walk needs it. [AssistNode] adapts the platform's `ViewNode`; the JVM tests
 * use a plain tree.
 */
interface ViewSource {
    /** The node itself is visible (not its ancestors). */
    val visible: Boolean

    /** The `webDomain` the node reports itself (a browser reports it on a container, not on each field). */
    val webDomain: String?
    val webScheme: String?
    val childCount: Int

    fun child(index: Int): ViewSource

    /**
     * The field this node is, or null for a node that takes no autofill value. [domain] and [scheme] are the node's
     * own values, or the nearest ancestor's when it has none (M10-G2 H1).
     */
    fun raw(shown: Boolean, domain: String?, scheme: String?): RawNode?
}

private fun RawNode.withRoot(root: PageRoot) =
    RawNode(
        autofillId, focused, visible, webDomain, webScheme, inputType, autofillHints, htmlTag, htmlAttrs, idEntry, hint,
        root.domain, root.scheme,
    )

/** What the reader found: the caller package and the fields, in tree order, with the IDs to answer them. */
class ParsedStructure(
    val callerPackage: String,
    val nodes: List<RawNode>,
) {
    private val walkedRoot: RawNode? get() = nodes.firstOrNull { it.pageDomain != null }
    private val firstWithDomain: RawNode? get() = nodes.firstOrNull { it.webDomain != null }

    /**
     * The web domain of the page: the first node in tree order that reports one, a container or a field (the core
     * roots a web fill at the target's own host, so a foreign frame is never the page), null for a native screen.
     * Only an allowlisted browser's value counts: the core decides that, see `fillTargetAndroid`.
     */
    val webDomain: String? get() = walkedRoot?.pageDomain ?: firstWithDomain?.webDomain

    /** The scheme of that same node, if it reports one. */
    val webScheme: String? get() = if (walkedRoot != null) walkedRoot?.pageScheme else firstWithDomain?.webScheme

    /** The node list in the core's record, the position in [nodes] being the node ID. */
    fun fieldNodes(): List<FieldNode> =
        nodes.mapIndexed { index, node ->
            FieldNode(
                id = index.toUInt(),
                focused = node.focused,
                visible = node.visible,
                webDomain = node.webDomain,
                inputType = node.inputType.toUInt(),
                autofillHints = node.autofillHints,
                htmlTag = node.htmlTag,
                htmlAttrs = node.htmlAttrs.map { Attr(it.first, it.second) },
                idEntry = node.idEntry,
                hint = node.hint,
            )
        }

    /** The autofill ID of node [id] (a position of [nodes]), or null when [id] is out of range. */
    fun idOf(id: UInt?): AutofillId? = if (id == null) null else nodes.getOrNull(id.toInt())?.autofillId
}

/**
 * Reads an `AssistStructure` into [ParsedStructure]. The reader only describes, and drops nothing: whether a field is
 * filled, and for whom, is decided by the core (`select_fill_fields`, `fill_target_android`). In particular the core,
 * not the reader, cuts a browser's own screen off a page, and only for a page: an app with a `WebView` keeps its
 * native fields (M10-G2 round 2, N1).
 */
@Suppress("NULLABILITY_MISMATCH_BASED_ON_JAVA_ANNOTATIONS")
object StructureReader {
    /** A page with more fields than this is cut off (fail closed): a hostile tree cannot make the walk unbounded. */
    const val MAX_NODES = 512

    private const val MAX_DEPTH = 64

    fun read(structure: AssistStructure): ParsedStructure {
        val out = ArrayList<RawNode>()
        val root = PageRoot()
        for (index in 0 until structure.windowNodeCount) {
            walk(AssistNode(structure.getWindowNodeAt(index).rootViewNode), true, null, null, 0, out, root)
        }
        return ParsedStructure(
            callerPackage = structure.activityComponent?.packageName.orEmpty(),
            nodes = out,
        )
    }

    /**
     * Walks the tree in order. A browser (Chrome, a WebView) reports the `webDomain` on a container node and not on
     * the fields below it, so the nearest ancestor's value is passed down and a field uses its own value first
     * (M10-G2 H1). Without it a page's fields would look like a native screen of the browser app.
     */
    fun walk(
        node: ViewSource,
        parentShown: Boolean,
        inheritedDomain: String?,
        inheritedScheme: String?,
        depth: Int,
        out: MutableList<RawNode>,
        root: PageRoot = PageRoot(),
    ) {
        if (out.size >= MAX_NODES || depth > MAX_DEPTH) return
        val shown = parentShown && node.visible
        // The domain and the scheme come as a pair from one node: a scheme of another ancestor would pair an origin
        // the page never had (M10-G2 round 2, N5).
        val ownDomain = node.webDomain
        val domain = ownDomain ?: inheritedDomain
        val scheme = if (ownDomain != null) node.webScheme else inheritedScheme
        // The page root is the first node that reports a domain, even one that takes no value (M10-G3-1).
        root.see(ownDomain, node.webScheme)
        node.raw(shown, domain, scheme)?.let { out.add(it.withRoot(root)) }
        for (child in 0 until node.childCount) {
            walk(node.child(child), shown, domain, scheme, depth + 1, out, root)
        }
    }
}

/** [ViewSource] over a platform `ViewNode`: plain reads, no logic. */
private class AssistNode(private val node: AssistStructure.ViewNode) : ViewSource {
    override val visible: Boolean get() = node.visibility == View.VISIBLE
    override val webDomain: String? get() = node.webDomain
    override val webScheme: String? get() = node.webScheme
    override val childCount: Int get() = node.childCount

    override fun child(index: Int): ViewSource = AssistNode(node.getChildAt(index))

    override fun raw(shown: Boolean, domain: String?, scheme: String?): RawNode? {
        val id = node.autofillId
        if (id == null || node.autofillType == View.AUTOFILL_TYPE_NONE) return null
        val html = node.htmlInfo
        val attrs = html?.attributes
        return RawNode(
            autofillId = id,
            focused = node.isFocused,
            visible = shown && node.width > 0 && node.height > 0,
            webDomain = domain,
            webScheme = scheme,
            inputType = node.inputType,
            autofillHints = node.autofillHints?.toList().orEmpty(),
            htmlTag = html?.tag,
            htmlAttrs = attrs?.map { Pair(it.first.orEmpty(), it.second.orEmpty()) }.orEmpty(),
            idEntry = node.idEntry,
            hint = node.hint,
        )
    }
}
