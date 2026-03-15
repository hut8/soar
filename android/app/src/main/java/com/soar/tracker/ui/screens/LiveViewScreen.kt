package com.soar.tracker.ui.screens

import java.util.Locale
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.unit.dp
import com.soar.tracker.data.api.NearbyAircraftInfo
import com.soar.tracker.service.TrackingService

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LiveViewScreen(modifier: Modifier = Modifier) {
    val latestResponse by TrackingService.latestResponse.collectAsState()
    val isTracking by TrackingService.isTracking.collectAsState()
    val sensorData by TrackingService.lastSensorData.collectAsState()
    val aircraft = latestResponse?.nearbyAircraft ?: emptyList()

    Scaffold(
        modifier = modifier,
        topBar = {
            TopAppBar(title = { Text("Nearby Aircraft") })
        },
    ) { padding ->
        if (!isTracking) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(32.dp),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(
                    text = "Start tracking to see nearby aircraft",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else if (aircraft.isEmpty()) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(32.dp),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(
                    text = "No aircraft nearby",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(horizontal = 12.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                item { Spacer(modifier = Modifier.height(4.dp)) }
                items(aircraft) { ac ->
                    AircraftCard(ac, sensorData?.latitude, sensorData?.longitude)
                }
                item { Spacer(modifier = Modifier.height(8.dp)) }
            }
        }
    }
}

@Composable
private fun AircraftCard(
    aircraft: NearbyAircraftInfo,
    userLatitude: Double?,
    userLongitude: Double?,
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            // Header: Registration + model + bearing arrow + distance
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = aircraft.registration ?: "Unknown",
                        style = MaterialTheme.typography.titleSmall,
                    )
                    Text(
                        text = aircraft.aircraftModel,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }

                // Bearing arrow
                if (userLatitude != null && userLongitude != null) {
                    val bearing = calculateBearing(
                        userLatitude, userLongitude,
                        aircraft.latitude, aircraft.longitude,
                    ).toFloat()
                    val arrowColor = MaterialTheme.colorScheme.primary
                    Canvas(
                        modifier = Modifier
                            .size(24.dp)
                            .padding(end = 8.dp),
                    ) {
                        rotate(bearing) {
                            val path = Path().apply {
                                moveTo(size.width / 2f, 0f)
                                lineTo(size.width * 0.8f, size.height * 0.7f)
                                lineTo(size.width / 2f, size.height * 0.5f)
                                lineTo(size.width * 0.2f, size.height * 0.7f)
                                close()
                            }
                            drawPath(path, arrowColor)
                        }
                    }
                }

                // Distance
                val distNm = aircraft.distanceMeters / 1852.0
                Text(
                    text = String.format(Locale.US, "%.1f NM", distNm),
                    style = MaterialTheme.typography.titleSmall,
                )
            }

            Spacer(modifier = Modifier.height(8.dp))
            HorizontalDivider()
            Spacer(modifier = Modifier.height(8.dp))

            // Details row: Altitude, Speed, Climb, Heading
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly,
            ) {
                // Altitude
                DetailValue(
                    label = "Alt",
                    value = aircraft.altitudeFeet?.let {
                        String.format(Locale.US, "%.0f ft", it)
                    } ?: "--",
                )

                // Ground speed
                DetailValue(
                    label = "GS",
                    value = aircraft.groundSpeedKnots?.let {
                        String.format(Locale.US, "%.0f kt", it)
                    } ?: "--",
                )

                // Climb rate with arrow
                DetailValue(
                    label = "Climb",
                    value = aircraft.climbFpm?.let {
                        val arrow = when {
                            it > 200 -> "\u25B2"
                            it < -200 -> "\u25BC"
                            else -> ""
                        }
                        String.format(Locale.US, "%s%.0f fpm", arrow, it)
                    } ?: "--",
                )

                // Track heading
                DetailValue(
                    label = "Track",
                    value = aircraft.trackDegrees?.let {
                        String.format(Locale.US, "%.0f\u00B0", it)
                    } ?: "--",
                )
            }

            // Staleness
            aircraft.lastFixAt?.let { time ->
                Spacer(modifier = Modifier.height(4.dp))
                val agoText = try {
                    val instant = java.time.Instant.parse(time)
                    val seconds = java.time.Duration.between(instant, java.time.Instant.now()).seconds
                    when {
                        seconds < 60 -> "${seconds}s ago"
                        seconds < 3600 -> "${seconds / 60}m ago"
                        else -> "${seconds / 3600}h ago"
                    }
                } catch (_: Exception) {
                    ""
                }
                if (agoText.isNotEmpty()) {
                    Text(
                        text = agoText,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

@Composable
private fun DetailValue(label: String, value: String) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

private fun calculateBearing(lat1: Double, lon1: Double, lat2: Double, lon2: Double): Double {
    val lat1Rad = Math.toRadians(lat1)
    val lat2Rad = Math.toRadians(lat2)
    val dLon = Math.toRadians(lon2 - lon1)
    val y = Math.sin(dLon) * Math.cos(lat2Rad)
    val x = Math.cos(lat1Rad) * Math.sin(lat2Rad) -
        Math.sin(lat1Rad) * Math.cos(lat2Rad) * Math.cos(dLon)
    val bearing = Math.toDegrees(Math.atan2(y, x))
    return (bearing + 360) % 360
}
