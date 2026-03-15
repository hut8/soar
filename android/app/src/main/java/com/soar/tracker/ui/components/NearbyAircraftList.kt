package com.soar.tracker.ui.components

import java.util.Locale
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.unit.dp
import com.soar.tracker.data.api.NearbyAircraftInfo
import com.soar.tracker.ui.util.calculateBearing

@Composable
fun NearbyAircraftList(
    aircraft: List<NearbyAircraftInfo>,
    userLatitude: Double?,
    userLongitude: Double?,
    userHeadingDegrees: Double? = null,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = "Nearby Aircraft (${aircraft.size})",
                style = MaterialTheme.typography.titleMedium,
            )
            Spacer(modifier = Modifier.height(8.dp))

            if (aircraft.isEmpty()) {
                Text(
                    text = "No aircraft nearby",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else {
                LazyColumn(
                    modifier = Modifier.height((aircraft.size.coerceAtMost(5) * 56).dp),
                ) {
                    items(aircraft) { ac ->
                        NearbyAircraftRow(ac, userLatitude, userLongitude, userHeadingDegrees)
                        HorizontalDivider()
                    }
                }
            }
        }
    }
}

@Composable
private fun NearbyAircraftRow(
    aircraft: NearbyAircraftInfo,
    userLatitude: Double?,
    userLongitude: Double?,
    userHeadingDegrees: Double?,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = aircraft.registration ?: aircraft.aircraftModel,
                style = MaterialTheme.typography.bodyMedium,
            )
            Text(
                text = aircraft.aircraftModel,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        // Bearing arrow
        if (userLatitude != null && userLongitude != null) {
            val absoluteBearing = calculateBearing(
                userLatitude, userLongitude,
                aircraft.latitude, aircraft.longitude,
            ).toFloat()
            // Make arrow relative to device heading
            val bearing = if (userHeadingDegrees != null) {
                absoluteBearing - userHeadingDegrees.toFloat()
            } else {
                absoluteBearing
            }
            val arrowColor = MaterialTheme.colorScheme.primary
            Canvas(
                modifier = Modifier.size(24.dp),
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

        Column(horizontalAlignment = Alignment.End) {
            val distNm = aircraft.distanceMeters / 1852.0
            Text(
                text = String.format(Locale.US, "%.1f NM", distNm),
                style = MaterialTheme.typography.bodyMedium,
            )
            aircraft.altitudeFeet?.let {
                Text(
                    text = String.format(Locale.US, "%.0f ft", it),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
