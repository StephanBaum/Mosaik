package io.github.stephanbaum.mosaic.scan

import android.Manifest
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.Settings
import android.util.Size
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.LifecycleOwner
import io.github.stephanbaum.mosaic.MosaicApp
import io.github.stephanbaum.mosaic.ffi.LumaFrame
import io.github.stephanbaum.mosaic.ffi.OtpPreview
import io.github.stephanbaum.mosaic.ffi.SetupCode
import io.github.stephanbaum.mosaic.session.Outcome
import io.github.stephanbaum.mosaic.ui.components.BodyText
import io.github.stephanbaum.mosaic.ui.components.ErrorText
import io.github.stephanbaum.mosaic.ui.components.HintText
import io.github.stephanbaum.mosaic.ui.components.PageColumn
import io.github.stephanbaum.mosaic.ui.components.PrimaryButton
import io.github.stephanbaum.mosaic.ui.components.QuietButton
import io.github.stephanbaum.mosaic.ui.components.ScreenFrame
import io.github.stephanbaum.mosaic.ui.components.TitleText
import io.github.stephanbaum.mosaic.ui.components.TopBar
import io.github.stephanbaum.mosaic.ui.errorText
import io.github.stephanbaum.mosaic.ui.l10n.L10n
import io.github.stephanbaum.mosaic.ui.theme.MosaicSpace
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.launch

private const val ANALYSIS_WIDTH = 1280
private const val ANALYSIS_HEIGHT = 720
private const val LOOK_EVERY_MS = 200L
private const val HINT_AFTER_MS = 5000L
private const val PREVIEW_RATIO = 3f / 4f

/**
 * The scanner: CameraX `ImageAnalysis` (the Y plane) into the core's `scan_setup_qr` or `scan_otp_qr`. There is
 * no ML Kit and no Play service. The camera permission is asked for here, when the screen opens, after a
 * sentence that says why; a refusal leaves a screen that points back to typing. Frames are copied for one scan,
 * zeroed afterwards and never stored. [layout] says where it is drawn.
 */
@Composable
fun ScanScreen(
    app: MosaicApp,
    target: ScanTarget,
    layout: ScanLayout,
    onSetupCode: (SetupCode) -> Unit,
    onOtp: (OtpPreview) -> Unit,
    onCancel: () -> Unit,
) {
    val resources = LocalContext.current.resources
    val title =
        if (target == ScanTarget.SetupCode) L10n.setupScanTitle(resources) else L10n.scanOtpTitle(resources)
    when (layout) {
        ScanLayout.Screen ->
            ScreenFrame {
                TitleText(title)
                ScanBody(app, target, onSetupCode, onOtp)
                QuietButton(L10n.commonBack(resources), onClick = onCancel)
            }
        ScanLayout.Shell ->
            PageColumn {
                TopBar(title = title, onBack = onCancel)
                ScanBody(app, target, onSetupCode, onOtp)
            }
        ScanLayout.Embedded ->
            Column(verticalArrangement = Arrangement.spacedBy(MosaicSpace.s4)) {
                ScanBody(app, target, onSetupCode, onOtp)
                QuietButton(L10n.commonCancel(resources), onClick = onCancel)
            }
    }
}

/** Where the scanner is drawn. */
enum class ScanLayout {
    /** A full onboarding screen. */
    Screen,

    /** A full page inside the signed-in shell. */
    Shell,

    /** Inside a form that must stay as it is (typed fields keep their text). */
    Embedded,
}

@Composable
private fun ScanBody(
    app: MosaicApp,
    target: ScanTarget,
    onSetupCode: (SetupCode) -> Unit,
    onOtp: (OtpPreview) -> Unit,
) {
    val context = LocalContext.current
    val resources = context.resources
    var granted by remember { mutableStateOf(hasCameraPermission(context)) }
    var requested by rememberSaveable { mutableStateOf(false) }
    var unavailable by remember { mutableStateOf(!hasCameraHardware(context)) }
    val launcher =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { allowed ->
            granted = allowed
            requested = true
        }

    // Back from the system settings after allowing the camera there.
    val owner = context.findLifecycleOwner()
    DisposableEffect(owner) {
        val observer =
            LifecycleEventObserver { _, event ->
                if (event == Lifecycle.Event.ON_RESUME) granted = hasCameraPermission(context)
            }
        owner?.lifecycle?.addObserver(observer)
        onDispose { owner?.lifecycle?.removeObserver(observer) }
    }

    Column(verticalArrangement = Arrangement.spacedBy(MosaicSpace.s4)) {
        if (unavailable) {
            BodyText(L10n.scanUnavailable(resources))
        } else {
            when (cameraAccess(granted, requested)) {
                CameraAccess.Ask -> {
                    BodyText(L10n.scanRationale(resources))
                    PrimaryButton(
                        L10n.scanAllow(resources),
                        onClick = { launcher.launch(Manifest.permission.CAMERA) },
                    )
                }
                CameraAccess.Denied -> {
                    BodyText(L10n.scanDenied(resources))
                    PrimaryButton(L10n.scanOpenSettings(resources), onClick = { openAppSettings(context) })
                }
                CameraAccess.Granted ->
                    Scanner(app, target, onSetupCode, onOtp, onUnavailable = { unavailable = true })
            }
        }
    }
}

/** The preview and the look-for-a-code loop. */
@Composable
private fun Scanner(
    app: MosaicApp,
    target: ScanTarget,
    onSetupCode: (SetupCode) -> Unit,
    onOtp: (OtpPreview) -> Unit,
    onUnavailable: () -> Unit,
) {
    val resources = LocalContext.current.resources
    val scope = rememberCoroutineScope()
    val inFlight = remember { AtomicBoolean(false) }
    val done = remember { AtomicBoolean(false) }
    val misses = remember { MissTracker(HINT_AFTER_MS, System::currentTimeMillis) }
    var errorCode by remember { mutableStateOf<String?>(null) }
    var hint by remember { mutableStateOf(false) }
    val setupCallback by rememberUpdatedState(onSetupCode)
    val otpCallback by rememberUpdatedState(onOtp)

    /** One scan of one frame, on the main thread; the core runs it off the main thread. */
    suspend fun scan(raw: RawLuma) {
        val frame = LumaFrame(raw.width.toUInt(), raw.height.toUInt(), raw.stride.toUInt(), raw.pixels)
        try {
            if (target == ScanTarget.SetupCode) {
                when (val outcome = app.session.scanSetupQr(frame)) {
                    is Outcome.Ok -> {
                        val code = outcome.value
                        if (code == null) {
                            errorCode = null
                            hint = misses.miss()
                        } else if (done.compareAndSet(false, true)) {
                            misses.reset()
                            setupCallback(code)
                        }
                    }
                    is Outcome.Failed -> {
                        misses.reset()
                        hint = false
                        errorCode = outcome.code
                    }
                    Outcome.Cancelled -> Unit
                }
            } else {
                when (val outcome = app.vault.scanOtpQr(frame)) {
                    is Outcome.Ok -> {
                        val preview = outcome.value
                        errorCode = null
                        if (preview != null && done.compareAndSet(false, true)) otpCallback(preview)
                    }
                    is Outcome.Failed -> errorCode = outcome.code
                    Outcome.Cancelled -> Unit
                }
            }
        } finally {
            raw.wipe()
        }
    }

    val onFrame: (RawLuma) -> Unit = { raw ->
        if (done.get() || !inFlight.compareAndSet(false, true)) {
            raw.wipe()
        } else {
            scope.launch {
                try {
                    scan(raw)
                } finally {
                    inFlight.set(false)
                }
            }
        }
    }

    CameraPreview(onFrame = onFrame, onUnavailable = onUnavailable)
    val failure = errorCode
    if (failure != null) {
        ErrorText(errorText(resources, failure))
    } else if (hint && target == ScanTarget.SetupCode) {
        ErrorText(L10n.scanNotSetupCode(resources))
        HintText(L10n.scanNotSetupCodeHelp(resources))
    } else {
        HintText(
            if (target == ScanTarget.SetupCode) L10n.setupScanHint(resources) else L10n.scanOtpHint(resources),
        )
    }
}

/** The camera view, bound to the screen's lifecycle: it stops when the app leaves the foreground. */
@Composable
private fun CameraPreview(onFrame: (RawLuma) -> Unit, onUnavailable: () -> Unit) {
    val context = LocalContext.current
    val owner = context.findLifecycleOwner()
    val latestFrame by rememberUpdatedState(onFrame)
    val latestUnavailable by rememberUpdatedState(onUnavailable)
    val previewView = remember { PreviewView(context) }
    previewView.contentDescription = L10n.scanPreview(context.resources)

    DisposableEffect(owner) {
        val executor = Executors.newSingleThreadExecutor()
        var provider: ProcessCameraProvider? = null
        if (owner == null) {
            latestUnavailable()
        } else {
            val pending = ProcessCameraProvider.getInstance(context)
            pending.addListener(
                {
                    try {
                        val camera = pending.get()
                        provider = camera
                        val preview = Preview.Builder().build()
                        preview.setSurfaceProvider(previewView.surfaceProvider)
                        val resolution =
                            ResolutionSelector.Builder()
                                .setResolutionStrategy(
                                    ResolutionStrategy(
                                        Size(ANALYSIS_WIDTH, ANALYSIS_HEIGHT),
                                        ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER,
                                    ),
                                )
                                .build()
                        val analysis =
                            ImageAnalysis.Builder()
                                .setResolutionSelector(resolution)
                                .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                                .build()
                        val throttle = FrameThrottle(LOOK_EVERY_MS, System::currentTimeMillis)
                        analysis.setAnalyzer(executor, LumaAnalyzer(throttle) { frame -> latestFrame(frame) })
                        camera.unbindAll()
                        camera.bindToLifecycle(owner, CameraSelector.DEFAULT_BACK_CAMERA, preview, analysis)
                    } catch (e: Exception) {
                        // No usable back camera, or the camera is busy: the screen says so and points to typing.
                        latestUnavailable()
                    }
                },
                ContextCompat.getMainExecutor(context),
            )
        }
        onDispose {
            provider?.unbindAll()
            executor.shutdown()
        }
    }

    AndroidView(
        factory = { previewView },
        modifier = Modifier.fillMaxWidth().aspectRatio(PREVIEW_RATIO),
    )
}

private fun hasCameraPermission(context: Context): Boolean =
    ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED

private fun hasCameraHardware(context: Context): Boolean =
    context.packageManager.hasSystemFeature(PackageManager.FEATURE_CAMERA_ANY)

private fun openAppSettings(context: Context) {
    val intent =
        Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null))
    try {
        context.startActivity(intent)
    } catch (e: ActivityNotFoundException) {
        // No settings screen on this device: the Back button still leads to typing the details.
    }
}

private fun Context.findLifecycleOwner(): LifecycleOwner? {
    var current: Context? = this
    while (current is ContextWrapper) {
        if (current is LifecycleOwner) return current
        current = current.baseContext
    }
    return null
}
