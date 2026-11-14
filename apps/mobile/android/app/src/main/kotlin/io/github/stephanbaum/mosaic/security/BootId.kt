package io.github.stephanbaum.mosaic.security

import android.content.ContentResolver
import android.provider.Settings
import java.nio.ByteBuffer

/**
 * The boot ID of CRYPTO_SPEC 6.9.2 on Android: `Settings.Global.BOOT_COUNT` as a big-endian u32. A read
 * failure gives an empty value, which the core refuses, so the boot marker never matches and the master
 * password is required (fail safe).
 */
object BootId {
    fun fromBootCount(count: Int): ByteArray = ByteBuffer.allocate(Int.SIZE_BYTES).putInt(count).array()

    fun read(resolver: ContentResolver): ByteArray =
        try {
            fromBootCount(Settings.Global.getInt(resolver, Settings.Global.BOOT_COUNT))
        } catch (e: Settings.SettingNotFoundException) {
            ByteArray(0)
        }
}
