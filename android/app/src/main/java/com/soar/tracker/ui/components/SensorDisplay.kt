package com.soar.tracker.ui.components

import java.util.Locale
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlin.math.sqrt

@Composable
fun SensorDisplay(
    accelX: Double?,
    accelY: Double?,
    accelZ: Double?,
    pressureHpa: Double?,
    verticalSpeedMps: Double?,
    modifier: Modifier = Modifier,
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text("Sensors", style = MaterialTheme.typography.titleMedium)
            Spacer(modifier = Modifier.height(8.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                // G-Force
                val gForce = if (accelX != null && accelY != null && accelZ != null) {
                    sqrt(accelX * accelX + accelY * accelY + accelZ * accelZ) / 9.81
                } else {
                    null
                }

                SensorValue(
                    label = "G-Force",
                    value = gForce?.let { String.format(Locale.US,"%.1f G", it) } ?: "--",
                )

                // Vario (vertical speed in fpm)
                val varioFpm = verticalSpeedMps?.let { it * 196.85 } // m/s to fpm
                SensorValue(
                    label = "Vario",
                    value = varioFpm?.let {
                        val sign = if (it >= 0) "+" else ""
                        String.format(Locale.US,"%s%.0f fpm", sign, it)
                    } ?: "--",
                )

                // Pressure
                SensorValue(
                    label = "Pressure",
                    value = pressureHpa?.let { String.format(Locale.US,"%.1f hPa", it) } ?: "--",
                )
            }
        }
    }
}

@Composable
private fun SensorValue(label: String, value: String) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = value,
            style = MaterialTheme.typography.titleMedium,
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}
