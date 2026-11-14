package io.github.stephanbaum.mosaic.autofill

/** The package names of the allowlisted browsers, read from the core's `getOrigin` list (ADR-0092). */
object BrowserPackages {
    private val PACKAGE_NAME = Regex(""""package_name"\s*:\s*"([^"]+)"""")

    /** Every `package_name` of [json]. The core owns the list; this only reads the names out of it. */
    fun parse(json: String): Set<String> = PACKAGE_NAME.findAll(json).map { it.groupValues[1] }.toSet()
}
