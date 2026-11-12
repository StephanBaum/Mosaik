package io.github.stephanbaum.mosaic.ui.link

import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import android.content.pm.ResolveInfo
import android.content.pm.SigningInfo
import android.os.Build
import io.github.stephanbaum.mosaic.autofill.Signers
import io.github.stephanbaum.mosaic.ffi.AndroidCaller
import java.security.MessageDigest

/** An installed app the user can link a login to. */
class AppChoice(val label: String, val packageName: String)

/** What the link screen shows about the chosen app (ADR-0092): who it is, where it came from, which signer. */
class AppDetails(
    val label: String,
    val packageName: String,
    val installer: String?,
    val certificateHex: String,
    val caller: AndroidCaller,
)

/** The first 8 bytes of a certificate fingerprint as `AA:BB:...` (what the user compares). */
fun shortFingerprint(sha256: ByteArray): String =
    sha256.take(8).joinToString(":") { "%02X".format(it.toInt() and 0xFF) }

/** SHA-256 of a signing certificate. */
fun certificateDigest(certificate: ByteArray): ByteArray = MessageDigest.getInstance("SHA-256").digest(certificate)

/**
 * The current signers as the core wants them (CRYPTO_SPEC / ADR-0092): all of `apkContentsSigners` when there are
 * several, otherwise the last entry of the certificate history.
 */
private fun currentSigners(signing: SigningInfo?): List<ByteArray> = Signers.of(signing)

@Suppress("DEPRECATION")
private fun installerOf(pm: PackageManager, packageName: String): String? =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        pm.getInstallSourceInfo(packageName).installingPackageName
    } else {
        pm.getInstallerPackageName(packageName)
    }

/** The details of an installed package, or null when it is not visible or not installed. */
@Suppress("DEPRECATION")
fun appDetails(context: Context, packageName: String): AppDetails? {
    val pm = context.packageManager
    val flags = PackageManager.GET_SIGNING_CERTIFICATES
    val info =
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                pm.getPackageInfo(packageName, PackageManager.PackageInfoFlags.of(flags.toLong()))
            } else {
                pm.getPackageInfo(packageName, flags)
            }
        } catch (e: PackageManager.NameNotFoundException) {
            return null
        }
    val signing: SigningInfo? = info.signingInfo
    val signers = currentSigners(signing)
    val applicationInfo: ApplicationInfo? = info.applicationInfo
    val label = applicationInfo?.loadLabel(pm)?.toString() ?: packageName
    val first = signers.firstOrNull()
    return AppDetails(
        label = label,
        packageName = packageName,
        installer = installerOf(pm, packageName),
        certificateHex = if (first == null) "" else shortFingerprint(first),
        caller = AndroidCaller(packageName, signers, null, null),
    )
}

/** The launchable apps of the phone except this one (needs the `queries` entry of the manifest), by label. */
@Suppress("DEPRECATION")
fun launchableApps(context: Context): List<AppChoice> {
    val pm = context.packageManager
    val intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
    val found: List<ResolveInfo> =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            pm.queryIntentActivities(intent, PackageManager.ResolveInfoFlags.of(0L))
        } else {
            pm.queryIntentActivities(intent, 0)
        }
    return found
        .map { AppChoice(it.loadLabel(pm).toString(), it.activityInfo.packageName) }
        .filter { it.packageName != context.packageName }
        .distinctBy { it.packageName }
        .sortedBy { it.label.lowercase() }
}
