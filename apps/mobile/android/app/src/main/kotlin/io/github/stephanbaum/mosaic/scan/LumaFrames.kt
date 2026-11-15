package io.github.stephanbaum.mosaic.scan

import java.nio.ByteBuffer

/**
 * A camera frame of grey levels: the Y plane of a YUV image, row by row. It lives only for the one scan that
 * reads it ([wipe] zeroes it afterwards) and is never written anywhere (docs/MOBILE.md 3.14).
 */
class RawLuma(
    val width: Int,
    val height: Int,
    /** Bytes per row (the plane may pad each row). */
    val stride: Int,
    val pixels: ByteArray,
) {
    fun wipe() {
        pixels.fill(0)
    }
}

/** Builds the frame the core scans from the Y plane of a camera image. */
object LumaFrames {
    /** More pixels than any phone camera gives to an analyzer: a guard against a bad plane. */
    const val MAX_PIXELS = 16_000_000L

    /**
     * Copies the plane into a [RawLuma], or returns null when it cannot be a grey-level plane: a pixel stride
     * other than 1, a row stride shorter than the width, no pixels, too many pixels, or a buffer shorter than
     * `stride * (height - 1) + width` (the last row may be short). The buffer's own position is left alone.
     */
    fun fromPlane(plane: ByteBuffer, width: Int, height: Int, rowStride: Int, pixelStride: Int): RawLuma? {
        if (pixelStride != 1 || width <= 0 || height <= 0 || rowStride < width) return null
        if (width.toLong() * height > MAX_PIXELS) return null
        val needed = rowStride.toLong() * (height - 1) + width
        val view = plane.duplicate()
        view.rewind()
        if (view.remaining() < needed) return null
        val count = minOf(view.remaining().toLong(), rowStride.toLong() * height).toInt()
        val pixels = ByteArray(count)
        view.get(pixels)
        return RawLuma(width, height, rowStride, pixels)
    }
}
