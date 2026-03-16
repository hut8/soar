package com.soar.tracker.ui.util

import com.soar.tracker.data.api.NearbyAircraftInfo
import kotlin.math.atan2

private const val METERS_TO_FEET = 3.28084
private const val NM_TO_FEET = 6076.12
private const val NM_TO_METERS = 1852.0

/**
 * Aircraft position with pre-computed bearing and elevation angle for AR projection.
 */
data class ARAircraftPosition(
    val aircraft: NearbyAircraftInfo,
    /** Bearing from user to aircraft in degrees (0-360). */
    val bearing: Double,
    /** Elevation angle from user to aircraft in degrees (positive = above horizon). */
    val elevation: Double,
    /** Distance from user in nautical miles. */
    val distanceNm: Double,
)

/**
 * Screen-space position of an aircraft marker.
 */
data class ARScreenPosition(
    val x: Float,
    val y: Float,
    val visible: Boolean,
)

/** Normalize bearing to -180..180 range for relative bearing calculations. */
fun normalizeBearing(bearing: Double): Double {
    var n = bearing
    while (n > 180) n -= 360
    while (n < -180) n += 360
    return n
}

/**
 * Calculate elevation angle from user to aircraft.
 * @param userAltitudeM User altitude in meters
 * @param aircraftAltitudeFt Aircraft altitude in feet
 * @param groundDistanceNm Horizontal distance in nautical miles
 * @return Angle in degrees above the horizon
 */
fun calculateElevationAngle(
    userAltitudeM: Double,
    aircraftAltitudeFt: Double,
    groundDistanceNm: Double,
): Double {
    val userAltitudeFt = userAltitudeM * METERS_TO_FEET
    val altDiffFt = aircraftAltitudeFt - userAltitudeFt
    val groundDistFt = groundDistanceNm * NM_TO_FEET
    if (groundDistFt == 0.0) return 0.0
    return Math.toDegrees(atan2(altDiffFt, groundDistFt))
}

/**
 * Wrap a [NearbyAircraftInfo] with bearing and elevation relative to the user.
 */
fun toARAircraftPosition(
    ac: NearbyAircraftInfo,
    userLat: Double,
    userLon: Double,
    userAltM: Double,
): ARAircraftPosition {
    val bearing = calculateBearing(userLat, userLon, ac.latitude, ac.longitude)
    val distNm = ac.distanceMeters / NM_TO_METERS
    val altFt = ac.altitudeFeet ?: 0.0
    val elevation = calculateElevationAngle(userAltM, altFt, distNm)
    return ARAircraftPosition(ac, bearing, elevation, distNm)
}

/**
 * Project an [ARAircraftPosition] to screen coordinates.
 *
 * Uses the same linear FOV projection as the web AR view:
 * relative bearing maps to X, adjusted elevation maps to Y.
 */
fun projectToScreen(
    arPos: ARAircraftPosition,
    headingDeg: Float,
    pitchDeg: Float,
    fovH: Float,
    fovV: Float,
    screenW: Float,
    screenH: Float,
): ARScreenPosition {
    val relativeBearing = normalizeBearing(arPos.bearing - headingDeg)
    val adjustedElevation = arPos.elevation - pitchDeg

    val withinH = Math.abs(relativeBearing) <= fovH / 2.0
    val withinV = Math.abs(adjustedElevation) <= fovV / 2.0
    if (!withinH || !withinV) {
        return ARScreenPosition(-1000f, -1000f, false)
    }

    val normalizedX = relativeBearing / (fovH / 2.0)
    val x = (screenW / 2.0 + normalizedX * screenW / 2.0).toFloat()

    val normalizedY = -adjustedElevation / (fovV / 2.0)
    val y = (screenH / 2.0 + normalizedY * screenH / 2.0).toFloat()

    return ARScreenPosition(x, y, true)
}

/**
 * Exponential moving average that handles the 0/360 wrap-around for angles.
 * Returns angle in 0..360 range.
 */
fun smoothAngle(current: Float, target: Float, alpha: Float): Float {
    var diff = target - current
    // Shortest path across 0/360 boundary
    if (diff > 180f) diff -= 360f
    if (diff < -180f) diff += 360f
    var result = current + alpha * diff
    if (result < 0f) result += 360f
    if (result >= 360f) result -= 360f
    return result
}

/** Simple EMA for pitch (no wrap-around needed). */
fun smoothLinear(current: Float, target: Float, alpha: Float): Float {
    return current + alpha * (target - current)
}
