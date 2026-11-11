package io.github.stephanbaum.mosaic.core

import io.github.stephanbaum.mosaic.ffi.CoreConfig
import io.github.stephanbaum.mosaic.ffi.CoreContext
import io.github.stephanbaum.mosaic.ffi.ItemFilter
import io.github.stephanbaum.mosaic.ffi.MobileException
import io.github.stephanbaum.mosaic.ffi.MosaicCore
import io.github.stephanbaum.mosaic.ffi.Platform
import io.github.stephanbaum.mosaic.ffi.SecureStorage
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.ffi.TrashMode
import io.github.stephanbaum.mosaic.ffi.coreVersion
import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** Loads the host library of the Rust core through JNA (system property set in core/build.gradle.kts). */
class CoreSmokeTest {
    private class MemoryStorage : SecureStorage {
        private val values = HashMap<String, ByteArray>()

        override fun get(name: String): ByteArray? = values[name]

        override fun put(name: String, value: ByteArray) {
            values[name] = value
        }

        override fun delete(name: String) {
            values.remove(name)
        }
    }

    @Test
    fun versionMatchesRust() {
        assertEquals(System.getProperty("mosaic.expectedVersion"), coreVersion())
    }

    @Test
    fun lockedCoreRefusesItems() {
        val dir = Files.createTempDirectory("mosaic-smoke")
        try {
            val core =
                MosaicCore.open(
                    CoreConfig(
                        storeDir = dir.toString(),
                        platform = Platform.ANDROID,
                        context = CoreContext.APP,
                        bootId = ByteArray(0),
                        trustAnchorsDer = emptyList(),
                    ),
                    MemoryStorage(),
                )
            assertTrue(core.state() is SessionState.NoAccount)
            val filter = ItemFilter(vaultId = null, trash = TrashMode.EXCLUDE, favoritesOnly = false, kind = null)
            val failure = runCatching { core.items(filter) }.exceptionOrNull()
            // A phone without an account has nothing to list; either way the core must refuse, not answer.
            assertTrue(
                "expected noAccount or locked, got $failure",
                failure is MobileException.NoAccount || failure is MobileException.Locked,
            )
            core.destroy()
        } finally {
            dir.toFile().deleteRecursively()
        }
    }
}
