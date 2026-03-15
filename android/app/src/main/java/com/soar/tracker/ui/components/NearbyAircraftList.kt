package com.soar.tracker.ui.components

import java.util.Locale
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.unit.dp
import com.soar.tracker.data.api.NearbyAircraftInfo

@Composable
fun NearbyAircraftList(
    aircraft: List<NearbyAircraftInfo>,
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
                        NearbyAircraftRow(ac)
                        HorizontalDivider()
                    }
                }
            }
        }
    }
}

@Composable
private fun NearbyAircraftRow(aircraft: NearbyAircraftInfo) {
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

        Column(horizontalAlignment = Alignment.End) {
            val distNm = aircraft.distanceMeters / 1852.0
            Text(
                text = String.format(Locale.US,"%.1f NM", distNm),
                style = MaterialTheme.typography.bodyMedium,
            )
            aircraft.altitudeFeet?.let {
                Text(
                    text = String.format(Locale.US,"%.0f ft", it),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
