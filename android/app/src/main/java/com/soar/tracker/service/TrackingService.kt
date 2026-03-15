package com.soar.tracker.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Looper
import androidx.lifecycle.LifecycleService
import androidx.lifecycle.lifecycleScope
import com.google.android.gms.location.FusedLocationProviderClient
import com.google.android.gms.location.LocationCallback
import com.google.android.gms.location.LocationRequest
import com.google.android.gms.location.LocationResult
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
import com.soar.tracker.MainActivity
import com.soar.tracker.R
import com.soar.tracker.SoarTrackerApp
import com.soar.tracker.data.api.TrackerFixRequest
import com.soar.tracker.data.api.TrackerFixResponse
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

data class SensorData(
    val accelX: Double,
    val accelY: Double,
    val accelZ: Double,
    val pressureHpa: Double?,
    val verticalSpeedMps: Double?,
    val latitude: Double?,
    val longitude: Double?,
    val altitudeMeters: Double?,
)

class TrackingService : LifecycleService() {
    private lateinit var fusedLocationClient: FusedLocationProviderClient
    private lateinit var sensorCollector: SensorCollector
    private var locationCallback: LocationCallback? = null
    private var pendingSubmit: kotlinx.coroutines.Job? = null

    // Vertical speed calculation state
    private val altitudeHistory = mutableListOf<Pair<Long, Double>>() // (timestamp_ms, altitude_m)

    companion object {
        const val CHANNEL_ID = "soar_tracking"
        const val NOTIFICATION_ID = 1
        const val ACTION_STOP = "com.soar.tracker.STOP"

        private val _latestResponse = MutableStateFlow<TrackerFixResponse?>(null)
        val latestResponse: StateFlow<TrackerFixResponse?> = _latestResponse

        private val _isTracking = MutableStateFlow(false)
        val isTracking: StateFlow<Boolean> = _isTracking

        private val _lastError = MutableStateFlow<String?>(null)
        val lastError: StateFlow<String?> = _lastError

        private val _lastUpdateTime = MutableStateFlow<Long>(0)
        val lastUpdateTime: StateFlow<Long> = _lastUpdateTime

        private val _lastSensorData = MutableStateFlow<SensorData?>(null)
        val lastSensorData: StateFlow<SensorData?> = _lastSensorData

        private val _groundPressureHpa = MutableStateFlow<Double?>(null)
        val groundPressureHpa: StateFlow<Double?> = _groundPressureHpa

        fun start(context: Context) {
            val intent = Intent(context, TrackingService::class.java)
            context.startForegroundService(intent)
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, TrackingService::class.java))
        }

        fun setGroundPressure(pressureHpa: Double) {
            _groundPressureHpa.value = pressureHpa
        }
    }

    override fun onCreate() {
        super.onCreate()
        fusedLocationClient = LocationServices.getFusedLocationProviderClient(this)
        sensorCollector = SensorCollector(this)
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        super.onStartCommand(intent, flags, startId)

        if (intent?.action == ACTION_STOP) {
            stopTracking()
            stopSelf()
            return START_NOT_STICKY
        }

        startForeground(NOTIFICATION_ID, createNotification())
        startTracking()
        return START_STICKY
    }

    override fun onDestroy() {
        stopTracking()
        super.onDestroy()
    }

    @Suppress("MissingPermission") // Permission checked before starting service
    private fun startTracking() {
        if (_isTracking.value) return

        sensorCollector.start()
        _isTracking.value = true
        _lastError.value = null

        val locationRequest = LocationRequest.Builder(
            Priority.PRIORITY_HIGH_ACCURACY,
            3_000L, // 3 seconds
        ).setMinUpdateIntervalMillis(2_000L).build()

        locationCallback = object : LocationCallback() {
            override fun onLocationResult(result: LocationResult) {
                val location = result.lastLocation ?: return

                val hasAltitude = location.hasAltitude()
                val verticalSpeed = if (hasAltitude) calculateVerticalSpeed(location.altitude) else null

                // Android accelerometer reports m/s², convert to g-force
                val ax = sensorCollector.accelX.toDouble() / 9.81
                val ay = sensorCollector.accelY.toDouble() / 9.81
                val az = sensorCollector.accelZ.toDouble() / 9.81
                val pressure = sensorCollector.pressureHpa?.toDouble()

                val request = TrackerFixRequest(
                    latitude = location.latitude,
                    longitude = location.longitude,
                    heading = if (location.hasBearing()) location.bearing.toDouble() else null,
                    altitudeMeters = if (hasAltitude) location.altitude else null,
                    speedMps = if (location.hasSpeed()) location.speed.toDouble() else null,
                    accuracyMeters = if (location.hasAccuracy()) location.accuracy.toDouble() else null,
                    pressureHpa = pressure,
                    accelX = ax,
                    accelY = ay,
                    accelZ = az,
                    verticalSpeedMps = verticalSpeed,
                )

                _lastSensorData.value = SensorData(
                    accelX = ax,
                    accelY = ay,
                    accelZ = az,
                    pressureHpa = pressure,
                    verticalSpeedMps = verticalSpeed,
                    latitude = location.latitude,
                    longitude = location.longitude,
                    altitudeMeters = if (hasAltitude) location.altitude else null,
                )

                submitFix(request)
            }
        }

        fusedLocationClient.requestLocationUpdates(
            locationRequest,
            locationCallback!!,
            Looper.getMainLooper(),
        )
    }

    private fun stopTracking() {
        locationCallback?.let { fusedLocationClient.removeLocationUpdates(it) }
        locationCallback = null
        sensorCollector.stop()
        _isTracking.value = false
        altitudeHistory.clear()
    }

    private fun submitFix(request: TrackerFixRequest) {
        val app = application as SoarTrackerApp
        val repo = app.trackerRepository ?: return

        // Cancel any in-flight request to avoid backlog on slow networks
        pendingSubmit?.cancel()
        pendingSubmit = lifecycleScope.launch {
            val result = repo.submitFix(request)
            result.onSuccess { response ->
                _latestResponse.value = response
                _lastError.value = null
                _lastUpdateTime.value = System.currentTimeMillis()
                // Capture ground pressure when AGL ≈ 0
                val agl = response.altitudeAglFeet
                val pressure = _lastSensorData.value?.pressureHpa
                if (agl != null && agl in -50..50 && pressure != null) {
                    _groundPressureHpa.value = pressure
                }
            }.onFailure { e ->
                _lastError.value = e.message ?: "Request failed"
            }
        }
    }

    private fun calculateVerticalSpeed(altitude: Double): Double? {
        val now = System.currentTimeMillis()
        altitudeHistory.add(now to altitude)

        // Keep last 5 samples
        while (altitudeHistory.size > 5) {
            altitudeHistory.removeAt(0)
        }

        if (altitudeHistory.size < 2) return null

        val first = altitudeHistory.first()
        val last = altitudeHistory.last()
        val dt = (last.first - first.first) / 1000.0 // seconds

        if (dt < 0.5) return null

        return (last.second - first.second) / dt
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            getString(R.string.tracking_notification_channel),
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = "Flight tracking notifications"
            setShowBadge(false)
        }
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(channel)
    }

    private fun createNotification(): Notification {
        val intent = Intent(this, MainActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        return Notification.Builder(this, CHANNEL_ID)
            .setContentTitle(getString(R.string.tracking_notification_title))
            .setContentText(getString(R.string.tracking_notification_text))
            .setSmallIcon(android.R.drawable.ic_menu_mylocation)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()
    }
}
