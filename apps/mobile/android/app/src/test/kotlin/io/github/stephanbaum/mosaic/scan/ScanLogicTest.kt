package io.github.stephanbaum.mosaic.scan

import java.nio.ByteBuffer
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LumaFramesTest {
    private fun plane(size: Int): ByteBuffer = ByteBuffer.wrap(ByteArray(size) { (it % 251).toByte() })

    @Test
    fun tightPlaneIsCopiedWhole() {
        val buffer = plane(12)
        val frame = LumaFrames.fromPlane(buffer, width = 4, height = 3, rowStride = 4, pixelStride = 1)
        assertNotNull(frame)
        assertEquals(4, frame!!.width)
        assertEquals(3, frame.height)
        assertEquals(4, frame.stride)
        assertArrayEquals(ByteArray(12) { (it % 251).toByte() }, frame.pixels)
    }

    @Test
    fun paddedRowsKeepTheirStride() {
        val frame = LumaFrames.fromPlane(plane(24), width = 5, height = 3, rowStride = 8, pixelStride = 1)!!
        assertEquals(8, frame.stride)
        assertEquals(24, frame.pixels.size)
    }

    @Test
    fun lastRowMayBeShort() {
        // 2 full rows of stride 8 and a last row of width 5: 8 * 2 + 5 = 21 bytes.
        val frame = LumaFrames.fromPlane(plane(21), width = 5, height = 3, rowStride = 8, pixelStride = 1)!!
        assertEquals(21, frame.pixels.size)
    }

    @Test
    fun aBufferShorterThanTheLastRowIsRefused() {
        assertNull(LumaFrames.fromPlane(plane(20), width = 5, height = 3, rowStride = 8, pixelStride = 1))
    }

    @Test
    fun aPlaneThatIsNotGreyLevelsIsRefused() {
        assertNull(LumaFrames.fromPlane(plane(100), width = 5, height = 3, rowStride = 8, pixelStride = 2))
        assertNull(LumaFrames.fromPlane(plane(100), width = 0, height = 3, rowStride = 8, pixelStride = 1))
        assertNull(LumaFrames.fromPlane(plane(100), width = 5, height = 0, rowStride = 8, pixelStride = 1))
        assertNull(LumaFrames.fromPlane(plane(100), width = 9, height = 3, rowStride = 8, pixelStride = 1))
    }

    @Test
    fun anAbsurdSizeIsRefused() {
        assertNull(LumaFrames.fromPlane(plane(16), width = 100_000, height = 100_000, rowStride = 100_000, pixelStride = 1))
    }

    @Test
    fun theBuffersPositionIsLeftAlone() {
        val buffer = plane(12)
        buffer.position(5)
        val frame = LumaFrames.fromPlane(buffer, width = 4, height = 3, rowStride = 4, pixelStride = 1)!!
        assertEquals(5, buffer.position())
        assertEquals(12, frame.pixels.size)
    }

    @Test
    fun wipeZeroesThePixels() {
        val frame = LumaFrames.fromPlane(plane(12), width = 4, height = 3, rowStride = 4, pixelStride = 1)!!
        frame.wipe()
        assertTrue(frame.pixels.all { it == 0.toByte() })
    }
}

class FrameThrottleTest {
    private var now = 1_000L

    @Test
    fun letsOneFrameInPerInterval() {
        val throttle = FrameThrottle(200) { now }
        assertTrue(throttle.admit())
        now += 100
        assertFalse(throttle.admit())
        now += 100
        assertTrue(throttle.admit())
        now += 199
        assertFalse(throttle.admit())
    }
}

class MissTrackerTest {
    private var now = 0L

    @Test
    fun hintsAfterTheTimeWithoutACode() {
        val tracker = MissTracker(5_000) { now }
        assertFalse(tracker.miss())
        now += 4_999
        assertFalse(tracker.miss())
        now += 1
        assertTrue(tracker.miss())
    }

    @Test
    fun aCodeOrAnErrorStartsTheCountAgain() {
        val tracker = MissTracker(5_000) { now }
        tracker.miss()
        now += 6_000
        assertTrue(tracker.miss())
        tracker.reset()
        assertFalse(tracker.miss())
        now += 4_000
        assertFalse(tracker.miss())
    }
}

class CameraAccessTest {
    @Test
    fun explainsBeforeAskingAndPointsToTypingAfterARefusal() {
        assertEquals(CameraAccess.Ask, cameraAccess(granted = false, requested = false))
        assertEquals(CameraAccess.Denied, cameraAccess(granted = false, requested = true))
        assertEquals(CameraAccess.Granted, cameraAccess(granted = true, requested = false))
        assertEquals(CameraAccess.Granted, cameraAccess(granted = true, requested = true))
    }
}
