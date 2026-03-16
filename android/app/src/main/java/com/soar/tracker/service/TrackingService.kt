package com.soar.tracker.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.location.Location
import android.os.Build
import android.os.Looper
import android.os.PowerManager
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
    /** Raw WGS84 ellipsoid height from GPS (not corrected for geoid). */
    val altitudeMeters: Double?,
    /**
     * Altitude above mean sea level (MSL) in meters.
     * On Android 34+: from [Location.getMslAltitudeMeters].
     * On older APIs: computed using the server-provided ground elevation and a cached geoid offset.
     * Falls back to raw WGS84 height if no correction is available yet.
     */
    val altitudeMslMeters: Double?,
    /** Ground speed in m/s from GPS */
    val speedMps: Double?,
    /** GPS track / direction of travel (only available when moving) */
    val gpsBearingDegrees: Double?,
    /** Magnetic compass heading from rotation vector sensor (always available) */
    val magneticHeadingDegrees: Double?,
)

class TrackingService : LifecycleService() {
    private lateinit var fusedLocationClient: FusedLocationProviderClient
    private lateinit var sensorCollector: SensorCollector
    private var locationCallback: LocationCallback? = null
    private var pendingSubmit: kotlinx.coroutines.Job? = null
    private var headingForwardJob: kotlinx.coroutines.Job? = null
    private var pitchForwardJob: kotlinx.coroutines.Job? = null
    private var rollForwardJob: kotlinx.coroutines.Job? = null
    private var wakeLock: PowerManager.WakeLock? = null

    // Vertical speed calculation state
    private val altitudeHistory = mutableListOf<Pair<Long, Double>>() // (timestamp_ms, altitude_m)

    /**
     * Cached geoid offset (WGS84 ellipsoid height minus MSL) in meters.
     * Computed from the server's ground elevation when AGL ≈ 0.
     * The geoid height is relatively stable over hundreds of kilometers,
     * so a single cached value is sufficient for a session.
     */
    private var cachedGeoidOffsetMeters: Double? = null

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

        /**
         * Live magnetic heading from the rotation vector sensor, updating at ~50 Hz.
         * Use this for UI that needs immediate heading updates (e.g. nearby-aircraft arrows)
         * instead of [lastSensorData] which only updates on GPS callbacks (~3 s).
         */
        private val _liveHeadingDegrees = MutableStateFlow<Float?>(null)
        val liveHeadingDegrees: StateFlow<Float?> = _liveHeadingDegrees

        private val _livePitchDegrees = MutableStateFlow<Float?>(null)
        val livePitchDegrees: StateFlow<Float?> = _livePitchDegrees

        private val _liveRollDegrees = MutableStateFlow<Float?>(null)
        val liveRollDegrees: StateFlow<Float?> = _liveRollDegrees

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
        wakeLock?.let { if (it.isHeld) it.release() }
        wakeLock = null
        super.onDestroy()
    }

    @Suppress("MissingPermission") // Permission checked before starting service
    private fun startTracking() {
        if (_isTracking.value) return

        // Acquire wake lock to keep CPU active during background tracking
        val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = powerManager.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "soar:tracking",
        ).apply {
            acquire(4 * 60 * 60 * 1000L) // 4 hour timeout as safety net
        }

        sensorCollector.start()
        _isTracking.value = true
        _lastError.value = null

        // Forward live heading from sensor collector to the static StateFlow
        headingForwardJob = lifecycleScope.launch {
            sensorCollector.liveHeadingDegrees.collect { heading ->
                _liveHeadingDegrees.value = heading
            }
        }
        pitchForwardJob = lifecycleScope.launch {
            sensorCollector.livePitchDegrees.collect { pitch ->
                _livePitchDegrees.value = pitch
            }
        }
        rollForwardJob = lifecycleScope.launch {
            sensorCollector.liveRollDegrees.collect { roll ->
                _liveRollDegrees.value = roll
            }
        }

        val locationRequest = LocationRequest.Builder(
            Priority.PRIORITY_HIGH_ACCURACY,
            3_000L, // 3 seconds
        ).setMinUpdateIntervalMillis(2_000L).build()

        locationCallback = object : LocationCallback() {
            override fun onLocationResult(result: LocationResult) {
                val location = result.lastLocation ?: return

                val hasAltitude = location.hasAltitude()
                val verticalSpeed = if (hasAltitude) calculateVerticalSpeed(location.altitude) else null

                // Compute MSL altitude
                val altMsl = if (hasAltitude) computeMslAltitude(location) else null

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
                    altitudeMslMeters = altMsl,
                    speedMps = if (location.hasSpeed()) location.speed.toDouble() else null,
                    gpsBearingDegrees = if (location.hasBearing()) location.bearing.toDouble() else null,
                    magneticHeadingDegrees = sensorCollector.magneticHeadingDegrees?.toDouble(),
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
        headingForwardJob?.cancel()
        headingForwardJob = null
        pitchForwardJob?.cancel()
        pitchForwardJob = null
        rollForwardJob?.cancel()
        rollForwardJob = null
        sensorCollector.stop()
        _isTracking.value = false
        _groundPressureHpa.value = null
        _liveHeadingDegrees.value = null
        _livePitchDegrees.value = null
        _liveRollDegrees.value = null
        cachedGeoidOffsetMeters = null
        altitudeHistory.clear()
        wakeLock?.let {
            if (it.isHeld) it.release()
        }
        wakeLock = null
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
                // Compute and cache geoid offset when near ground level.
                // geoidOffset = WGS84_height - MSL_height
                // When AGL ≈ 0: MSL_height ≈ groundElevation, so
                // geoidOffset ≈ WGS84_height - groundElevation
                // Use request.altitudeMeters (captured at call time) rather than
                // _lastSensorData which may have advanced during the network round-trip.
                val groundElev = response.groundElevationMeters
                val gpsAlt = request.altitudeMeters
                if (agl != null && agl in -50..50 &&
                    groundElev != null && gpsAlt != null &&
                    cachedGeoidOffsetMeters == null
                ) {
                    cachedGeoidOffsetMeters = gpsAlt - groundElev
                }
            }.onFailure { e ->
                _lastError.value = e.message?.takeIf { it.isNotBlank() } ?: "Request failed"
            }
        }
    }

    /**
     * Compute MSL altitude from a GPS [Location].
     *
     * GPS `getAltitude()` returns height above the WGS84 ellipsoid, which can differ
     * from mean-sea-level (MSL) by up to 100 m depending on location.  For example,
     * in Troy, NY the geoid is ≈29 m below the WGS84 ellipsoid, so the raw GPS
     * altitude reads ≈29 m (≈95 ft) too low.
     *
     * Strategy:
     *  1. Android 34+: use `getMslAltitudeMeters()` (built-in EGM2008 model).
     *  2. Older APIs: apply a cached geoid offset derived from the server's
     *     `groundElevationMeters` when the user was near ground level.
     *  3. Fallback: return raw WGS84 altitude (better than nothing).
     */
    private fun computeMslAltitude(location: Location): Double {
        // Android 34+ provides MSL altitude directly via built-in EGM2008 model
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            if (location.hasMslAltitude()) {
                return location.mslAltitudeMeters
            }
        }

        // Apply cached geoid offset if available
        val offset = cachedGeoidOffsetMeters
        if (offset != null) {
            return location.altitude - offset
        }

        // No correction available yet — return raw WGS84 height
        return location.altitude
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
        val openIntent = Intent(this, MainActivity::class.java)
        val openPendingIntent = PendingIntent.getActivity(
            this, 0, openIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val stopIntent = Intent(this, TrackingService::class.java).apply {
            action = ACTION_STOP
        }
        val stopPendingIntent = PendingIntent.getService(
            this, 1, stopIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        return Notification.Builder(this, CHANNEL_ID)
            .setContentTitle(getString(R.string.tracking_notification_title))
            .setContentText(getString(R.string.tracking_notification_text))
            .setSmallIcon(android.R.drawable.ic_menu_mylocation)
            .setContentIntent(openPendingIntent)
            .addAction(
                android.R.drawable.ic_media_pause,
                "Stop Tracking",
                stopPendingIntent,
            )
            .setOngoing(true)
            .build()
    }
}
