package io.github.stephanbaum.mosaic.scan

import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy

/**
 * Takes the Y plane of a camera image and hands a copy to [onFrame], at most as often as [throttle] allows. The
 * image is closed at once; nothing is kept, saved or logged.
 */
class LumaAnalyzer(
    private val throttle: FrameThrottle,
    private val onFrame: (RawLuma) -> Unit,
) : ImageAnalysis.Analyzer {
    override fun analyze(image: ImageProxy) {
        try {
            if (!throttle.admit()) return
            val plane = image.planes.firstOrNull() ?: return
            val frame =
                LumaFrames.fromPlane(plane.buffer, image.width, image.height, plane.rowStride, plane.pixelStride)
                    ?: return
            onFrame(frame)
        } finally {
            image.close()
        }
    }
}
