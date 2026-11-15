package io.github.stephanbaum.mosaic.autofill

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LinkFlowTest {
    @Test
    fun aStrongBiometricConfirmsWithThePrompt() =
        assertEquals(LinkConfirm.Biometric, LinkFlow.confirmMode(strongBiometricAvailable = true))

    @Test
    fun withoutOneTheScreenConfirms() =
        assertEquals(LinkConfirm.Screen, LinkFlow.confirmMode(strongBiometricAvailable = false))

    @Test
    fun onlyTheBiometricPathFills() {
        assertTrue(LinkFlow.fillsAfterLink(LinkConfirm.Biometric))
        assertFalse(LinkFlow.fillsAfterLink(LinkConfirm.Screen))
    }

    @Test
    fun onlyThePlayStoreAndTheGalaxyStoreAreTrusted() {
        assertTrue(LinkFlow.isTrustedInstaller("com.android.vending"))
        assertTrue(LinkFlow.isTrustedInstaller("com.sec.android.app.samsungapps"))
        assertFalse(LinkFlow.isTrustedInstaller(null))
        assertFalse(LinkFlow.isTrustedInstaller(""))
        assertFalse(LinkFlow.isTrustedInstaller("com.android.shell"))
        assertFalse(LinkFlow.isTrustedInstaller("com.android.packageinstaller"))
        assertFalse(LinkFlow.isTrustedInstaller("com.google.android.packageinstaller"))
        // Byte for byte: no case folding, no prefix or look-alike rule.
        assertFalse(LinkFlow.isTrustedInstaller("COM.ANDROID.VENDING"))
        assertFalse(LinkFlow.isTrustedInstaller("com.android.vending.fake"))
        assertFalse(LinkFlow.isTrustedInstaller("com.android.vending "))
    }

    private val play = "com.android.vending"
    private val galaxy = "com.sec.android.app.samsungapps"

    @Test
    fun theInstallSourceNeedsTheInstallerAndTheInitiatorTrusted() {
        assertTrue(LinkFlow.installSourceTrusted(30, play, play))
        assertTrue(LinkFlow.installSourceTrusted(34, galaxy, galaxy))
        // A privileged installer (adb) can name a store as the installer; the initiator gives it away.
        assertFalse(LinkFlow.installSourceTrusted(34, play, "com.android.shell"))
        assertFalse(LinkFlow.installSourceTrusted(34, "com.android.shell", play))
    }

    @Test
    fun beforeApi30NothingIsTrusted() {
        assertFalse(LinkFlow.installSourceTrusted(29, play, play))
        assertFalse(LinkFlow.installSourceTrusted(28, play, null))
    }

    @Test
    fun missingInstallSourceFieldsAreNotTrusted() {
        assertFalse(LinkFlow.installSourceTrusted(34, null, null))
        assertFalse(LinkFlow.installSourceTrusted(34, play, null))
        assertFalse(LinkFlow.installSourceTrusted(34, null, play))
        assertFalse(LinkFlow.installSourceTrusted(34, "", ""))
    }

    @Test
    fun anImportedLoginNeedsATrustedInstallSource() {
        assertTrue(LinkFlow.mayOffer(imported = true, installTrusted = true))
        assertFalse(LinkFlow.mayOffer(imported = true, installTrusted = false))
    }

    @Test
    fun aVerifiedLoginIsNotGatedByTheInstallSource() {
        assertTrue(LinkFlow.mayOffer(imported = false, installTrusted = false))
        assertTrue(LinkFlow.mayOffer(imported = false, installTrusted = true))
    }

    @Test
    fun linkEntriesAreCappedAtThree() {
        val entries = LinkFlow.selectEntries(listOf("a", "b", "c", "d", "e"), { false }, installTrusted = false)
        assertEquals(listOf("a", "b", "c"), entries)
    }

    @Test
    fun untrustedInstallSourceDropsOnlyImportedEntries() {
        val all = listOf("verified1", "imported1", "verified2", "imported2")
        val kept = LinkFlow.selectEntries(all, { it.startsWith("imported") }, installTrusted = false)
        assertEquals(listOf("verified1", "verified2"), kept)
        // Trusted: nothing is dropped, only the cap of three applies.
        assertEquals(all.take(3), LinkFlow.selectEntries(all, { it.startsWith("imported") }, installTrusted = true))
    }

    @Test
    fun atMostThreeEntries() = assertEquals(3, LinkFlow.MAX_ENTRIES)
}
