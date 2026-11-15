package io.github.stephanbaum.mosaic.autofill

import android.content.Context
import android.content.pm.PackageManager

/** What the platform knows about an installed app: its current signers and its label. */
object CallerReader {
    /**
     * The current signers of [packageName] (see [Signers]), or null when the package is not visible or not
     * installed. Not visible means the request fails closed: Mosaic does not fill an app it cannot identify
     * (ADR-0092, package visibility).
     */
    @Suppress("DEPRECATION")
    fun signersOf(context: Context, packageName: String): List<ByteArray>? {
        val pm = context.packageManager
        val flags = PackageManager.GET_SIGNING_CERTIFICATES
        val info =
            try {
                if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.TIRAMISU) {
                    pm.getPackageInfo(packageName, PackageManager.PackageInfoFlags.of(flags.toLong()))
                } else {
                    pm.getPackageInfo(packageName, flags)
                }
            } catch (e: PackageManager.NameNotFoundException) {
                return null
            }
        val signers = Signers.of(info.signingInfo)
        return signers.ifEmpty { null }
    }

    /** The label the user sees for [packageName], or the package name itself. */
    @Suppress("DEPRECATION")
    fun labelOf(context: Context, packageName: String): String =
        try {
            val pm = context.packageManager
            pm.getApplicationInfo(packageName, 0).loadLabel(pm).toString()
        } catch (e: PackageManager.NameNotFoundException) {
            packageName
        }
}
