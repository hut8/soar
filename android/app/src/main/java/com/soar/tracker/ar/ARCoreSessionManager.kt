package com.soar.tracker.ar

import android.app.Activity
import android.opengl.GLES20
import android.opengl.GLSurfaceView
import com.google.ar.core.ArCoreApk
import com.google.ar.core.Config
import com.google.ar.core.Pose
import com.google.ar.core.Session
import com.google.ar.core.TrackingState
import com.google.ar.core.exceptions.CameraNotAvailableException
import com.google.ar.core.exceptions.UnavailableException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import javax.microedition.khronos.egl.EGLConfig
import javax.microedition.khronos.opengles.GL10
import kotlin.math.asin
import kotlin.math.atan
import kotlin.math.atan2

/**
 * Manages an ARCore [Session] and extracts heading/pitch from the camera pose each frame.
 *
 * ARCore's visual-inertial odometry gives frame-accurate pose tracking, so the
 * overlay markers stay perfectly synchronized with the camera image. The heading is
 * calibrated to magnetic north using a one-time offset from the sensor-based compass.
 */
class ARCoreSessionManager(private val activity: Activity) : GLSurfaceView.Renderer {

    private var session: Session? = null
    private val backgroundRenderer = BackgroundRenderer()

    // North calibration: offset = magneticNorth - arcoreYaw (averaged over N samples)
    private var northOffsetDegrees: Float? = null
    private val calibrationSamples = mutableListOf<Float>() // (magneticHeading - arcoreYaw)
    @Volatile
    private var pendingMagneticHeading: Float? = null

    private val _headingDegrees = MutableStateFlow<Float?>(null)
    val headingDegrees: StateFlow<Float?> = _headingDegrees

    private val _pitchDegrees = MutableStateFlow<Float?>(null)
    val pitchDegrees: StateFlow<Float?> = _pitchDegrees

    private val _fovHDegrees = MutableStateFlow<Float?>(null)
    val fovHDegrees: StateFlow<Float?> = _fovHDegrees

    private val _fovVDegrees = MutableStateFlow<Float?>(null)
    val fovVDegrees: StateFlow<Float?> = _fovVDegrees

    private val _isAvailable = MutableStateFlow(false)
    val isAvailable: StateFlow<Boolean> = _isAvailable

    /**
     * Provide a magnetic heading sample for north calibration.
     * Should be called whenever the sensor collector has a new heading value.
     * Only the first [CALIBRATION_SAMPLE_COUNT] samples are used.
     */
    fun calibrateNorth(magneticHeadingDegrees: Float) {
        if (northOffsetDegrees != null) return // Already calibrated
        pendingMagneticHeading = magneticHeadingDegrees
    }

    fun createSession(): Boolean {
        return try {
            val availability = ArCoreApk.getInstance().checkAvailability(activity)
            if (!availability.isSupported) {
                _isAvailable.value = false
                return false
            }

            val session = Session(activity)
            val config = Config(session).apply {
                updateMode = Config.UpdateMode.LATEST_CAMERA_IMAGE
                planeFindingMode = Config.PlaneFindingMode.DISABLED
                lightEstimationMode = Config.LightEstimationMode.DISABLED
                depthMode = Config.DepthMode.DISABLED
                focusMode = Config.FocusMode.AUTO
            }
            session.configure(config)
            this.session = session
            _isAvailable.value = true
            true
        } catch (_: UnavailableException) {
            _isAvailable.value = false
            false
        }
    }

    fun resume() {
        try {
            session?.resume()
        } catch (_: CameraNotAvailableException) {
            // Camera unavailable — will retry on next resume
        }
    }

    fun pause() {
        session?.pause()
    }

    fun destroy() {
        session?.close()
        session = null
        _isAvailable.value = false
        _headingDegrees.value = null
        _pitchDegrees.value = null
        _fovHDegrees.value = null
        _fovVDegrees.value = null
        northOffsetDegrees = null
        calibrationSamples.clear()
    }

    // --- GLSurfaceView.Renderer ---

    override fun onSurfaceCreated(gl: GL10?, config: EGLConfig?) {
        GLES20.glClearColor(0f, 0f, 0f, 1f)
        backgroundRenderer.createOnGlThread()
        session?.setCameraTextureName(backgroundRenderer.textureId)
    }

    @Suppress("DEPRECATION") // defaultDisplay is fine for our use
    override fun onSurfaceChanged(gl: GL10?, width: Int, height: Int) {
        GLES20.glViewport(0, 0, width, height)
        session?.setDisplayGeometry(activity.windowManager.defaultDisplay.rotation, width, height)
    }

    override fun onDrawFrame(gl: GL10?) {
        GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT or GLES20.GL_DEPTH_BUFFER_BIT)

        val session = this.session ?: return
        val frame = try {
            session.update()
        } catch (_: CameraNotAvailableException) {
            return
        }

        // Draw camera background
        backgroundRenderer.draw(frame)

        val camera = frame.camera
        if (camera.trackingState != TrackingState.TRACKING) return

        // Extract heading and pitch from the display-oriented pose
        val pose = camera.displayOrientedPose
        val (arcoreYawDeg, pitchDeg) = extractYawPitch(pose)

        // Collect north calibration samples
        if (northOffsetDegrees == null) {
            val magHeading = pendingMagneticHeading
            if (magHeading != null) {
                var diff = magHeading - arcoreYawDeg
                // Normalize to -180..180 for averaging
                if (diff > 180f) diff -= 360f
                if (diff < -180f) diff += 360f
                calibrationSamples.add(diff)

                if (calibrationSamples.size >= CALIBRATION_SAMPLE_COUNT) {
                    northOffsetDegrees = calibrationSamples.average().toFloat()
                }
            }
        }

        val offset = northOffsetDegrees
        if (offset != null) {
            var heading = (arcoreYawDeg + offset) % 360f
            if (heading < 0) heading += 360f
            _headingDegrees.value = heading
        }

        _pitchDegrees.value = pitchDeg

        // Extract actual camera FOV from projection matrix
        val projMatrix = FloatArray(16)
        camera.getProjectionMatrix(projMatrix, 0, 0.1f, 100f)
        // projMatrix[0] = 1/tan(fovX/2), projMatrix[5] = 1/tan(fovY/2)
        if (projMatrix[0] != 0f) {
            _fovHDegrees.value = Math.toDegrees(2.0 * atan(1.0 / projMatrix[0])).toFloat()
        }
        if (projMatrix[5] != 0f) {
            _fovVDegrees.value = Math.toDegrees(2.0 * atan(1.0 / projMatrix[5])).toFloat()
        }
    }

    /**
     * Extract yaw and pitch from an ARCore [Pose] quaternion.
     *
     * ARCore coordinate system: Y up, -Z is the camera forward direction.
     * The displayOrientedPose accounts for screen rotation.
     */
    private fun extractYawPitch(pose: Pose): Pair<Float, Float> {
        val qx = pose.qx()
        val qy = pose.qy()
        val qz = pose.qz()
        val qw = pose.qw()

        // Yaw: rotation around Y axis (gravity direction)
        val sinYaw = 2f * (qw * qy + qx * qz)
        val cosYaw = 1f - 2f * (qy * qy + qx * qx)
        val yawRad = atan2(sinYaw, cosYaw)
        // Negate because ARCore yaw increases counter-clockwise
        var yawDeg = -Math.toDegrees(yawRad.toDouble()).toFloat()
        if (yawDeg < 0) yawDeg += 360f

        // Pitch: rotation around X axis
        val sinPitch = 2f * (qw * qx - qy * qz)
        val pitchDeg = Math.toDegrees(asin(sinPitch.coerceIn(-1f, 1f)).toDouble()).toFloat()

        return Pair(yawDeg, pitchDeg)
    }

    companion object {
        private const val CALIBRATION_SAMPLE_COUNT = 5
    }
}
