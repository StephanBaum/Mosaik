package io.github.stephanbaum.mosaic

import android.app.Activity
import android.app.Application
import android.app.Application.ActivityLifecycleCallbacks
import android.app.KeyguardManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import androidx.core.content.ContextCompat
import io.github.stephanbaum.mosaic.autofill.LinkTokens
import io.github.stephanbaum.mosaic.core.CoreHolder
import io.github.stephanbaum.mosaic.ffi.CoreConfig
import io.github.stephanbaum.mosaic.ffi.CoreContext
import io.github.stephanbaum.mosaic.ffi.MobileSettings
import io.github.stephanbaum.mosaic.ffi.MosaicCore
import io.github.stephanbaum.mosaic.ffi.Platform
import io.github.stephanbaum.mosaic.ffi.SecureStorage
import io.github.stephanbaum.mosaic.security.AndroidClipboardPort
import io.github.stephanbaum.mosaic.security.AndroidSecureStorage
import io.github.stephanbaum.mosaic.security.BiometricGate
import io.github.stephanbaum.mosaic.security.BootId
import io.github.stephanbaum.mosaic.security.ClipboardGuard
import io.github.stephanbaum.mosaic.security.LockPolicy
import io.github.stephanbaum.mosaic.security.LockSettings
import io.github.stephanbaum.mosaic.security.MainLooperScheduler
import io.github.stephanbaum.mosaic.security.MemoryCheck
import io.github.stephanbaum.mosaic.security.ProviderActivity
import io.github.stephanbaum.mosaic.security.QuickUnlockKeys
import io.github.stephanbaum.mosaic.security.TrustAnchors
import io.github.stephanbaum.mosaic.security.sessionNeedsLock
import io.github.stephanbaum.mosaic.session.JoinTracker
import io.github.stephanbaum.mosaic.session.SessionController
import io.github.stephanbaum.mosaic.session.VaultController
import java.io.File
import java.util.UUID
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.update

/**
 * The process: one [CoreHolder] (the activities, and later the Autofill service and the credential provider,
 * share it), the lock policy fed by the activity lifecycle and the screen-off broadcast, and the clipboard
 * guard. A new process always starts locked, because the core does.
 */
@Suppress("NULLABILITY_MISMATCH_BASED_ON_JAVA_ANNOTATIONS")
class MosaicApp : Application() {
    lateinit var holder: CoreHolder
        private set
    lateinit var lockPolicy: LockPolicy
        private set
    lateinit var clipboard: ClipboardGuard
        private set
    lateinit var session: SessionController
        private set
    lateinit var vault: VaultController
        private set
    private lateinit var keyguard: KeyguardManager

    val joinTracker = JoinTracker()

    /** The one-time tokens of the "No login for ..." entries (M10-G2 M3). */
    val linkTokens = LinkTokens(SystemClock::elapsedRealtime)

    @Volatile
    var lockSettings = LockSettings()
        private set

    private val sessionEpochState = MutableStateFlow(0)
    private val foregroundState = MutableStateFlow(false)
    private val handler = Handler(Looper.getMainLooper())
    private var startedActivities = 0

    /** Counts up whenever the session changed outside a screen (a lock); screens re-read the state. */
    val sessionEpoch: StateFlow<Int> get() = sessionEpochState

    /** True while an activity is started. */
    val foreground: StateFlow<Boolean> get() = foregroundState

    private val ticker =
        object : Runnable {
            override fun run() {
                lockPolicy.tick()
                clipboard.tick()
                // In the background the ticker runs only while the session is open or a copied secret is on the
                // clipboard: a background delay or the provider grace (ADR-0094) must lock on time, and the clipboard
                // deadline must hold, even when no screen comes back to the front.
                if (foregroundState.value || sessionNeedsLock(holder.state()) || clipboard.hasPending) {
                    handler.postDelayed(this, TICK_MS)
                }
            }
        }

    override fun onCreate() {
        super.onCreate()
        keyguard = getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
        val storage = AndroidSecureStorage(File(noBackupFilesDir, "secure"), deviceSecure = { keyguard.isDeviceSecure })
        holder = CoreHolder { openCore(storage) }
        clipboard =
            ClipboardGuard(AndroidClipboardPort(this), MainLooperScheduler(), SystemClock::elapsedRealtime) {
                UUID.randomUUID().toString()
            }
        lockPolicy =
            LockPolicy(
                clock = SystemClock::elapsedRealtime,
                settings = { lockSettings },
                hasSession = { sessionNeedsLock(holder.state()) },
                joinPending = { joinTracker.isPending },
                lock = { lockNow() },
            )
        session =
            SessionController(
                holder = holder,
                keys = QuickUnlockKeys(),
                gate = BiometricGate(),
                availableMemory = { MemoryCheck.availableBytes(this) },
                joinTracker = joinTracker,
                onSettings = { applySettings(it) },
                onSessionChanged = {
                    sessionEpochState.update { it + 1 }
                    // An unlock can finish after the app went to the background (a lock that arrived while the core
                    // was busy ended it locked, but a late unlock may not): check the deadlines at once and restart
                    // the ticker, which stopped while the core was locked (M10-G2 M1).
                    if (!foregroundState.value) {
                        handler.post {
                            handler.removeCallbacks(ticker)
                            ticker.run()
                        }
                    }
                },
            )
        vault = VaultController(holder = holder, onSettings = { applySettings(it) })
        registerActivityLifecycleCallbacks(
            object : ActivityLifecycleCallbacks {
                override fun onActivityStarted(activity: Activity) {
                    startedActivities += 1
                    if (startedActivities == 1) {
                        foregroundState.value = true
                        lockPolicy.onForeground(activity is ProviderActivity)
                        clipboard.onForeground()
                        handler.removeCallbacks(ticker)
                        handler.postDelayed(ticker, TICK_MS)
                    }
                }

                override fun onActivityStopped(activity: Activity) {
                    startedActivities -= 1
                    // A configuration change stops and starts the activity again: that is not a background.
                    if (startedActivities == 0 && !activity.isChangingConfigurations) {
                        foregroundState.value = false
                        lockPolicy.onBackground()
                    }
                }

                override fun onActivityCreated(activity: Activity, savedInstanceState: Bundle?) = Unit

                override fun onActivityResumed(activity: Activity) = Unit

                override fun onActivityPaused(activity: Activity) = Unit

                override fun onActivitySaveInstanceState(activity: Activity, outState: Bundle) = Unit

                override fun onActivityDestroyed(activity: Activity) = Unit
            },
        )
        ContextCompat.registerReceiver(
            this,
            object : BroadcastReceiver() {
                override fun onReceive(context: Context, intent: Intent) {
                    clipboard.clearNow()
                    lockPolicy.onScreenOff()
                }
            },
            IntentFilter(Intent.ACTION_SCREEN_OFF),
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )
    }

    /** Whether the phone has a secure lock screen; the secure storage needs one (ADR-0090 amendment). */
    fun isDeviceSecure(): Boolean = keyguard.isDeviceSecure

    /** Locks the core at once (it bypasses the call queue) and tells the screens. */
    fun lockNow() {
        clipboard.clearNow()
        holder.lock()
        sessionEpochState.update { it + 1 }
    }

    /** Takes the settings the lock policy and the clipboard guard read. */
    fun applySettings(settings: MobileSettings) {
        lockSettings =
            LockSettings(
                backgroundSeconds = settings.autoLockBackgroundSeconds.toInt(),
                idleMinutes = settings.idleLockMinutes.toInt(),
                clipboardSeconds = settings.clipboardClearSeconds.toInt(),
            )
    }

    private fun openCore(storage: SecureStorage): MosaicCore {
        val storeDir = File(noBackupFilesDir, "mosaic")
        storeDir.mkdirs()
        return MosaicCore.open(
            CoreConfig(
                storeDir = storeDir.path,
                platform = Platform.ANDROID,
                context = CoreContext.APP,
                bootId = BootId.read(contentResolver),
                trustAnchorsDer = TrustAnchors.collectDer(),
            ),
            storage,
        )
    }

    private companion object {
        const val TICK_MS = 1000L
    }
}
