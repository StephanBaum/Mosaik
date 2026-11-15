package io.github.stephanbaum.mosaic.ui

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The positional arguments of every string (`%1$s`, `%2$d`) are the same in English and German, and number
 * 1..n without gaps, so the generated typed accessors cannot pass them in the wrong order (docs/MOBILE.md,
 * section 3.15).
 */
class L10nTest {
    private val placeholder = Regex("%(\\d+)\\$([sd])")
    private val resource = Regex("<(string|plurals) name=\"([^\"]+)\"[^>]*>(.*?)</\\1>", RegexOption.DOT_MATCHES_ALL)

    /** Resource name to the set of "index:type" of its placeholders (all plural items together). */
    private fun placeholders(dir: String): Map<String, Set<String>> {
        val xml = File("src/main/res/$dir/strings.xml").readText()
        val result = HashMap<String, Set<String>>()
        for (match in resource.findAll(xml)) {
            val found = placeholder.findAll(match.groupValues[3]).map { it.groupValues[1] + ":" + it.groupValues[2] }
            result[match.groupValues[2]] = found.toSet()
        }
        return result
    }

    @Test
    fun argumentOrderMatchesCatalog() {
        val english = placeholders("values")
        val german = placeholders("values-de")
        assertTrue("no strings parsed", english.size > 100)
        assertEquals("both languages define the same strings", english.keys, german.keys)
        for ((name, args) in english) {
            assertEquals("placeholders of $name differ between EN and DE", args, german[name])
            val indexes = args.map { it.substringBefore(':').toInt() }.toSet()
            assertEquals("placeholders of $name are not 1..n", (1..indexes.size).toSet(), indexes)
        }
    }
}
