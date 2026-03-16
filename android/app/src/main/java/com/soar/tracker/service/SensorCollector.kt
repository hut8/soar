package com.soar.tracker.service

import android.content.Context
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class SensorCollector(context: Context) : SensorEventListener {
    private val sensorManager = context.getSystemService(Context.SENSOR_SERVICE) as SensorManager
    private val accelerometer = sensorManager.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)
    private val barometer = sensorManager.getDefaultSensor(Sensor.TYPE_PRESSURE)
    private val rotationVector = sensorManager.getDefaultSensor(Sensor.TYPE_ROTATION_VECTOR)

    @Volatile
    var accelX: Float = 0f
        private set

    @Volatile
    var accelY: Float = 0f
        private set

    @Volatile
    var accelZ: Float = 0f
        private set

    @Volatile
    var pressureHpa: Float? = null
        private set

    /** Magnetic heading in degrees (0-360, 0=north). Derived from rotation vector sensor. */
    @Volatile
    var magneticHeadingDegrees: Float? = null
        private set

    /**
     * Live heading StateFlow that updates on every sensor event (~20ms at SENSOR_DELAY_GAME).
     * UI components that need real-time heading (e.g. nearby-aircraft arrows) should collect
     * this flow instead of waiting for the GPS-paced SensorData snapshot.
     */
    private val _liveHeadingDegrees = MutableStateFlow<Float?>(null)
    val liveHeadingDegrees: StateFlow<Float?> = _liveHeadingDegrees

    private val rotationMatrix = FloatArray(9)
    private val orientation = FloatArray(3)

    fun start() {
        accelerometer?.let {
            sensorManager.registerListener(this, it, SensorManager.SENSOR_DELAY_GAME)
        }
        barometer?.let {
            sensorManager.registerListener(this, it, SensorManager.SENSOR_DELAY_NORMAL)
        }
        rotationVector?.let {
            sensorManager.registerListener(this, it, SensorManager.SENSOR_DELAY_GAME)
        }
    }

    fun stop() {
        sensorManager.unregisterListener(this)
        _liveHeadingDegrees.value = null
    }

    override fun onSensorChanged(event: SensorEvent) {
        when (event.sensor.type) {
            Sensor.TYPE_ACCELEROMETER -> {
                accelX = event.values[0]
                accelY = event.values[1]
                accelZ = event.values[2]
            }
            Sensor.TYPE_PRESSURE -> {
                pressureHpa = event.values[0]
            }
            Sensor.TYPE_ROTATION_VECTOR -> {
                SensorManager.getRotationMatrixFromVector(rotationMatrix, event.values)
                SensorManager.getOrientation(rotationMatrix, orientation)
                // orientation[0] is azimuth in radians (-π to π), convert to degrees (0-360)
                var azimuth = Math.toDegrees(orientation[0].toDouble()).toFloat()
                if (azimuth < 0) azimuth += 360f
                magneticHeadingDegrees = azimuth
                _liveHeadingDegrees.value = azimuth
            }
        }
    }

    override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) {
        // Not used
    }
}
