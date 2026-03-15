package com.soar.tracker.data.api

import com.squareup.moshi.Json
import com.squareup.moshi.JsonClass

// --- Auth ---

@JsonClass(generateAdapter = true)
data class LoginRequest(
    val email: String,
    val password: String,
)

@JsonClass(generateAdapter = true)
data class LoginResponse(
    val token: String,
    val user: UserInfo,
)

@JsonClass(generateAdapter = true)
data class UserInfo(
    val id: String,
    @Json(name = "firstName") val firstName: String?,
    @Json(name = "lastName") val lastName: String?,
    val email: String?,
)

@JsonClass(generateAdapter = true)
data class PasswordResetRequest(
    val email: String,
)

// --- Tracker ---

@JsonClass(generateAdapter = true)
data class TrackerFixRequest(
    val latitude: Double,
    val longitude: Double,
    val heading: Double? = null,
    @Json(name = "altitudeMeters") val altitudeMeters: Double? = null,
    @Json(name = "speedMps") val speedMps: Double? = null,
    @Json(name = "accuracyMeters") val accuracyMeters: Double? = null,
    @Json(name = "pressureHpa") val pressureHpa: Double? = null,
    @Json(name = "accelX") val accelX: Double? = null,
    @Json(name = "accelY") val accelY: Double? = null,
    @Json(name = "accelZ") val accelZ: Double? = null,
    @Json(name = "verticalSpeedMps") val verticalSpeedMps: Double? = null,
)

@JsonClass(generateAdapter = true)
data class TrackerFixResponse(
    @Json(name = "fixId") val fixId: String,
    val timestamp: String,
    @Json(name = "matchedAircraft") val matchedAircraft: TrackerAircraftInfo?,
    val flight: TrackerFlightInfo?,
    @Json(name = "nearbyAircraft") val nearbyAircraft: List<NearbyAircraftInfo>,
    @Json(name = "groundElevationMeters") val groundElevationMeters: Double?,
    @Json(name = "altitudeAglFeet") val altitudeAglFeet: Int?,
)

@JsonClass(generateAdapter = true)
data class TrackerAircraftInfo(
    val id: String,
    val registration: String?,
    @Json(name = "aircraftModel") val aircraftModel: String,
    @Json(name = "competitionNumber") val competitionNumber: String,
    @Json(name = "distanceMeters") val distanceMeters: Double,
)

@JsonClass(generateAdapter = true)
data class TrackerFlightInfo(
    val id: String,
    val state: String,
    @Json(name = "takeoffTime") val takeoffTime: String?,
    @Json(name = "departureAirport") val departureAirport: String?,
    @Json(name = "durationSeconds") val durationSeconds: Long?,
)

@JsonClass(generateAdapter = true)
data class NearbyAircraftInfo(
    val id: String,
    val registration: String?,
    @Json(name = "aircraftModel") val aircraftModel: String,
    val latitude: Double,
    val longitude: Double,
    @Json(name = "altitudeFeet") val altitudeFeet: Double?,
    @Json(name = "groundSpeedKnots") val groundSpeedKnots: Double?,
    @Json(name = "distanceMeters") val distanceMeters: Double,
    @Json(name = "lastFixAt") val lastFixAt: String?,
)
