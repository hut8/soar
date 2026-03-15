package com.soar.tracker.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.soar.tracker.data.api.TrackerAircraftInfo
import com.soar.tracker.data.api.TrackerFlightInfo

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

            if (flight != null) {
                Spacer(modifier = Modifier.height(12.dp))
                Text("Flight", style = MaterialTheme.typography.titleMedium)
                Spacer(modifier = Modifier.height(4.dp))

                Row {
                    Text("State: ", style = MaterialTheme.typography.bodyMedium)
                    Text(
                        text = flight.state.replaceFirstChar { it.uppercase() },
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }

                flight.departureAirport?.let { airport ->
                    Row {
                        Text("From: ", style = MaterialTheme.typography.bodyMedium)
                        Text(airport, style = MaterialTheme.typography.bodyMedium)
                    }
                }

                flight.durationSeconds?.let { seconds ->
                    val minutes = seconds / 60
                    val hours = minutes / 60
                    val remainingMinutes = minutes % 60
                    Row {
                        Text("Duration: ", style = MaterialTheme.typography.bodyMedium)
                        Text(
                            text = if (hours > 0) "${hours}h ${remainingMinutes}m" else "${minutes}m",
                            style = MaterialTheme.typography.bodyMedium,
                        )
                    }
                }
            }
        }
    }
}
