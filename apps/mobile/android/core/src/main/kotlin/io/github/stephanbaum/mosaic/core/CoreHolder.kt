package io.github.stephanbaum.mosaic.core

import io.github.stephanbaum.mosaic.ffi.MosaicCore
import io.github.stephanbaum.mosaic.ffi.SessionState
import java.util.concurrent.atomic.AtomicReference
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.withContext

/**
 * Owns the one [MosaicCore] of the process (the activities, the Autofill service and the credential
 * provider share it). Every core call blocks (docs/MOBILE.md, threading), so [call] runs it on a
 * single-threaded IO dispatcher: calls are serial and never touch the main thread.
 *
 * [lock] and [state] bypass the queue: they are cheap and must preempt a long call (an Argon2id unlock).
 */
class CoreHolder(
    private val open: () -> MosaicCore,
) {
    @OptIn(ExperimentalCoroutinesApi::class)
    private val coreDispatcher: CoroutineDispatcher = Dispatchers.IO.limitedParallelism(1)

    private val core = AtomicReference<MosaicCore?>(null)

    /** Runs [block] with the core on the core dispatcher, opening the core on first use. */
    suspend fun <T> call(block: (MosaicCore) -> T): T =
        withContext(coreDispatcher) { block(openedCore()) }

    /** Locks at once, without waiting for a running call. A no-op while the core is not open. */
    fun lock() {
        core.get()?.lock()
    }

    /** The session state at once, or null while the core is not open. */
    fun state(): SessionState? = core.get()?.state()

    // Only called on the core dispatcher, so opening is never concurrent.
    private fun openedCore(): MosaicCore =
        core.get() ?: open().also { core.set(it) }
}
