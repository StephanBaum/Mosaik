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

    @Test
    fun anImportedLoginNeedsATrustedInstaller() {
        assertTrue(LinkFlow.mayOffer(imported = true, installer = "com.android.vending"))
        assertTrue(LinkFlow.mayOffer(imported = true, installer = "com.sec.android.app.samsungapps"))
        assertFalse(LinkFlow.mayOffer(imported = true, installer = null))
        assertFalse(LinkFlow.mayOffer(imported = true, installer = "com.android.shell"))
    }

    @Test
    fun aVerifiedLoginIsNotGatedByTheInstaller() {
        assertTrue(LinkFlow.mayOffer(imported = false, installer = null))
        assertTrue(LinkFlow.mayOffer(imported = false, installer = "com.android.shell"))
    }

    @Test
    fun atMostThreeEntries() = assertEquals(3, LinkFlow.MAX_ENTRIES)
}
