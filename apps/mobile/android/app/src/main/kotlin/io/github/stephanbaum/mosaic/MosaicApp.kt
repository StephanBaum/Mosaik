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
import io.github.stephanbaum.mosaic.core.CoreHolder
import io.github.stephanbaum.mosaic.ffi.CoreConfig
import io.github.stephanbaum.mosaic.ffi.CoreContext
import io.github.stephanbaum.mosaic.ffi.MobileSettings
import io.github.stephanbaum.mosaic.ffi.MosaicCore
import io.github.stephanbaum.mosaic.ffi.Platform
import io.github.stephanbaum.mosaic.ffi.SecureStorage
import io.github.stephanbaum.mosaic.ffi.SessionState
import io.github.stephanbaum.mosaic.security.AndroidClipboardPort
import io.github.stephanbaum.mosaic.security.AndroidSecureStorage
import io.github.stephanbaum.mosaic.security.BiometricGate
import io.github.stephanbaum.mosaic.security.BootId
import io.github.stephanbaum.mosaic.security.ClipboardGuard
import io.github.stephanbaum.mosaic.security.LockPolicy
import io.github.stephanbaum.mosaic.security.LockSettings
import io.github.stephanbaum.mosaic.security.MainLooperScheduler
import io.github.stephanbaum.mosaic.security.MemoryCheck
import io.github.stephanbaum.mosaic.security.QuickUnlockKeys
import io.github.stephanbaum.mosaic.security.TrustAnchors
import io.github.stephanbaum.mosaic.session.JoinTracker
import io.github.stephanbaum.mosaic.session.SessionController
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

    val joinTracker = JoinTracker()

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
                handler.postDelayed(this, TICK_MS)
            }
        }

    override fun onCreate() {
        super.onCreate()
        val keyguard = getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager
        val storage = AndroidSecureStorage(File(noBackupFilesDir, "secure"), deviceSecure = { keyguard.isDeviceSecure })
        holder = CoreHolder { openCore(storage) }
        clipboard =
            ClipboardGuard(AndroidClipboardPort(this), MainLooperScheduler()) { UUID.randomUUID().toString() }
        lockPolicy =
            LockPolicy(
                clock = SystemClock::elapsedRealtime,
                settings = { lockSettings },
                isUnlocked = { holder.state() is SessionState.Unlocked },
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
                onSessionChanged = { sessionEpochState.update { it + 1 } },
            )
        registerActivityLifecycleCallbacks(
            object : ActivityLifecycleCallbacks {
                override fun onActivityStarted(activity: Activity) {
                    startedActivities += 1
                    if (startedActivities == 1) {
                        foregroundState.value = true
                        lockPolicy.onForeground()
                        handler.postDelayed(ticker, TICK_MS)
                    }
                }

                override fun onActivityStopped(activity: Activity) {
                    startedActivities -= 1
                    // A configuration change stops and starts the activity again: that is not a background.
                    if (startedActivities == 0 && !activity.isChangingConfigurations) {
                        foregroundState.value = false
                        handler.removeCallbacks(ticker)
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
                    lockPolicy.onScreenOff()
                }
            },
            IntentFilter(Intent.ACTION_SCREEN_OFF),
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )
    }

    /** Locks the core at once (it bypasses the call queue) and tells the screens. */
    fun lockNow() {
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
