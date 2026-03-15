package com.soar.tracker.ui.components

import java.util.Locale
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import com.soar.tracker.data.api.TrackerAircraftInfo
import com.soar.tracker.data.api.TrackerFlightInfo
import com.soar.tracker.ui.theme.Green
import com.soar.tracker.ui.theme.Orange
import com.soar.tracker.ui.theme.Red

@Composable
fun FlightStatusCard(
    aircraft: TrackerAircraftInfo?,
    flight: TrackerFlightInfo?,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            // Aircraft section
            Text("Aircraft", style = MaterialTheme.typography.titleMedium)
            Spacer(modifier = Modifier.height(8.dp))

            if (aircraft != null) {
                val label = buildString {
                    aircraft.registration?.let { append(it) }
                    if (aircraft.competitionNumber.isNotBlank()) {
                        if (isNotEmpty()) append(" ")
                        append("(${aircraft.competitionNumber})")
                    }
                }
                Text(
                    text = label.ifBlank { aircraft.aircraftModel },
                    style = MaterialTheme.typography.bodyLarge,
                )
                Text(
                    text = aircraft.aircraftModel,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = "Distance: ${aircraft.distanceMeters.toInt()}m",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else {
                Text(
                    text = "No aircraft matched",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            // Flight section
            if (flight != null) {
                Spacer(modifier = Modifier.height(12.dp))
                HorizontalDivider()
                Spacer(modifier = Modifier.height(12.dp))

                // Flight state with colored indicator
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("Flight", style = MaterialTheme.typography.titleMedium)
                    Spacer(modifier = Modifier.width(8.dp))
                    val stateColor = when (flight.state.lowercase()) {
                        "active" -> Green
                        "stale" -> Orange
                        else -> Red
                    }
                    Box(
                        modifier = Modifier
                            .size(8.dp)
                            .clip(CircleShape)
                            .background(stateColor),
                    )
                    Text(
                        text = " ${flight.state.replaceFirstChar { it.uppercase() }}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Spacer(modifier = Modifier.height(8.dp))

                // Takeoff time
                flight.takeoffTime?.let { time ->
                    // Format ISO timestamp to a readable time
                    val displayTime = try {
                        val instant = java.time.Instant.parse(time)
                        val local = java.time.LocalDateTime.ofInstant(
                            instant, java.time.ZoneId.systemDefault(),
                        )
                        val formatter = java.time.format.DateTimeFormatter.ofPattern("h:mm a")
                        "Takeoff: ${local.format(formatter)}"
                    } catch (_: Exception) {
                        "Takeoff: $time"
                    }
                    Text(text = displayTime, style = MaterialTheme.typography.bodyMedium)
                }

                // Duration
                flight.durationSeconds?.let { seconds ->
                    val hours = seconds / 3600
                    val minutes = (seconds % 3600) / 60
                    val secs = seconds % 60
                    val durationText = if (hours > 0) {
                        String.format(Locale.US, "%d:%02d:%02d", hours, minutes, secs)
                    } else {
                        String.format(Locale.US, "%d:%02d", minutes, secs)
                    }
                    Text(
                        text = "Duration: $durationText",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }

                // Departure airport
                flight.departureAirport?.let { airport ->
                    Text(
                        text = "From: $airport",
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }

                // Tow information
                if (flight.towedByRegistration != null || flight.towedByAircraftModel != null) {
                    Spacer(modifier = Modifier.height(8.dp))
                    val towLabel = buildString {
                        append("Towed by ")
                        flight.towedByRegistration?.let { append(it) }
                        flight.towedByAircraftModel?.let {
                            if (flight.towedByRegistration != null) append(" ")
                            append("($it)")
                        }
                    }
                    Text(text = towLabel, style = MaterialTheme.typography.bodyMedium)

                    // Release info
                    if (flight.towReleaseAltitudeMslFt != null) {
                        Text(
                            text = "Released at ${flight.towReleaseAltitudeMslFt} ft MSL",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}
