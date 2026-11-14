package io.github.stephanbaum.mosaic.security

import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Ignore
import org.junit.Test
import org.w3c.dom.Document
import org.w3c.dom.Element

/**
 * RT-android red team (Android, round 4): the lock policy's provider grace, the task affinity of the provider
 * activities, and the manifest's backup, cleartext and overlay defenses. Every test states the attack; a hole found
 * here is `@Ignore("RT-android-<n>: ...")` until its fix, and the other tests pin defenses that held.
 */
class RedTeamAndroidTest {
    // ---- lock policy ------------------------------------------------------------------------

    private var now = 1_000_000L
    private var locks = 0
    private var unlocked = true

    private fun policy() =
        LockPolicy(
            clock = { now },
            settings = { LockSettings() },
            hasSession = { unlocked },
            joinPending = { false },
            lock = {
                locks += 1
                unlocked = false
            },
        )

    /**
     * Attack (RT-android-2): the user unlocks Mosaic inside another app's autofill ("Unlock Mosaic", list mode) or
     * inside the Credential Manager picker (unlock mode) and then does not pick an entry (dismisses the picker, or no
     * login matched). The provider activity finishes with its flow still open, so the policy grants the 60 s grace
     * (ADR-0094, R131). Someone who now holds the phone opens Mosaic from the launcher within those 60 s:
     * `onForeground` finds the grace deadline not reached, keeps the session, and the vault opens with no unlock,
     * although the background lock is "at once" (`LockSettings.DEFAULT_BACKGROUND_SECONDS = 0`) and cannot be turned
     * off. The grace was meant for the step between two provider activities, not for the main activity. Defense
     * wanted: a foreground return by the main activity during a grace locks (the policy learns which kind of activity
     * returned); only a provider activity keeps the grace.
     */
    @Test
    @Ignore(
        "RT-android-2: the provider grace covers the main activity too: a launcher start within 60 s of a provider " +
            "unlock opens the vault without an unlock (LockPolicy.onForeground keeps the session while the grace runs)",
    )
    fun theMainActivityReturningDuringAProviderGraceMeetsTheLockScreen() {
        val policy = policy()
        policy.openProviderFlow("autofill")
        policy.onBackground()
        now += 5_000
        policy.onForeground()
        assertEquals(1, locks)
    }

    // ---- manifest -------------------------------------------------------------------------

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
     * Attack (RT-android-3, task hijacking of the StrandHogg family): a malicious app declares an activity with
     * `taskAffinity="io.github.stephanbaum.mosaic"`. `LinkActivity` is started with `FLAG_ACTIVITY_NEW_TASK |
     * FLAG_ACTIVITY_CLEAR_TASK` (FillResponseFactory.noLoginDataset) and keeps the package's default affinity, so it
     * lands on the attacker's task, or the attacker's activity (with `allowTaskReparenting`) is pulled into Mosaic's
     * task: the Back key from the real unlock screen lands on a fake one. Only `MainActivity` opts out with
     * `taskAffinity=""`. Defense wanted: every activity sets `taskAffinity=""` (and the link entry does not clear a
     * task).
     */
    @Test
    @Ignore(
        "RT-android-3: only MainActivity sets taskAffinity=\"\"; FillAuthActivity, LinkActivity (started with " +
            "NEW_TASK|CLEAR_TASK) and CredentialActivity keep the package affinity",
    )
    fun everyActivityOptsOutOfTheSharedTaskAffinity() {
        val doc = xml("src/main/AndroidManifest.xml")
        for (activity in elements(doc, "activity")) {
            val name = activity.android("name")
            assertTrue("$name needs android:taskAffinity=\"\"", activity.hasAttributeNS(androidNs, "taskAffinity"))
            assertEquals(name, "", activity.android("taskAffinity"))
        }
    }

    /**
     * Held: nothing leaves the phone through a backup or a device transfer, no cleartext on the Java stack, system CAs
     * only (no user CAs, no debug overrides), and the overlay-hiding permission is declared.
     */
    @Test
    fun backupCleartextTrustAndOverlayDefensesHold() {
        val manifest = xml("src/main/AndroidManifest.xml")
        val application = elements(manifest, "application").single()
        assertEquals("false", application.android("allowBackup"))
        assertEquals("false", application.android("fullBackupContent"))
        assertEquals("false", application.android("usesCleartextTraffic"))
        assertEquals("@xml/data_extraction_rules", application.android("dataExtractionRules"))
        assertEquals("@xml/network_security_config", application.android("networkSecurityConfig"))
        val permissions = elements(manifest, "uses-permission").map { it.android("name") }
        assertTrue(permissions.contains("android.permission.HIDE_OVERLAY_WINDOWS"))
        assertTrue(permissions.none { it.contains("SYSTEM_ALERT_WINDOW") || it.contains("QUERY_ALL_PACKAGES") })

        val rules = xml("src/main/res/xml/data_extraction_rules.xml")
        for (section in listOf("cloud-backup", "device-transfer")) {
            val node = elements(rules, section).single()
            val excludes = node.getElementsByTagName("exclude")
            val excluded =
                (0 until excludes.length)
                    .map { excludes.item(it) as Element }
                    .map { it.getAttribute("domain") to it.getAttribute("path") }
            assertEquals(
                section,
                setOf("root", "file", "database", "sharedpref", "external").map { it to "." }.toSet(),
                excluded.toSet(),
            )
            assertEquals(section, 0, node.getElementsByTagName("include").length)
        }

        val network = xml("src/main/res/xml/network_security_config.xml")
        assertEquals("false", elements(network, "base-config").single().getAttribute("cleartextTrafficPermitted"))
        assertEquals(listOf("system"), elements(network, "certificates").map { it.getAttribute("src") })
        assertEquals(0, network.getElementsByTagName("debug-overrides").length)
        assertEquals(0, network.getElementsByTagName("domain-config").length)
    }
}
