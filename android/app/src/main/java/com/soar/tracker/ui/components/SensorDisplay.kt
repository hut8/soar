package com.soar.tracker.ui.components

import java.util.Locale
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
    latitude: Double?,
    longitude: Double?,
    altitudeMeters: Double?,
    altitudeAglFeet: Int?,
    gpsBearingDegrees: Double?,
    magneticHeadingDegrees: Double?,
    magneticDeclinationDegrees: Double?,
    groundPressureHpa: Double?,
    onSetAltimeter: () -> Unit,
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

            // Row 1: G-Force, Vario, Pressure
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                val gForce = if (accelX != null && accelY != null && accelZ != null) {
                    sqrt(accelX * accelX + accelY * accelY + accelZ * accelZ)
                } else {
                    null
                }

                SensorValue(
                    label = "G-Force",
                    value = gForce?.let { String.format(Locale.US, "%.1f G", it) } ?: "--",
                )

                val varioFpm = verticalSpeedMps?.let { it * 196.85 }
                SensorValue(
                    label = "Vario",
                    value = varioFpm?.let {
                        val sign = if (it >= 0) "+" else ""
                        String.format(Locale.US, "%s%.0f fpm", sign, it)
                    } ?: "--",
                )

                // Pressure in both hPa and inHg
                val pressureText = if (pressureHpa != null) {
                    val inHg = pressureHpa * 0.02953
                    String.format(Locale.US, "%.1f hPa\n%.2f inHg", pressureHpa, inHg)
                } else {
                    "--"
                }
                SensorValue(
                    label = "Pressure",
                    value = pressureText,
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            // Row 2: GNSS Alt, AGL, Position
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                val altFt = altitudeMeters?.let { it * 3.28084 }
                SensorValue(
                    label = "GNSS Alt",
                    value = altFt?.let { String.format(Locale.US, "%.0f ft", it) } ?: "--",
                )

                SensorValue(
                    label = "AGL",
                    value = altitudeAglFeet?.let { String.format(Locale.US, "%d ft", it) } ?: "--",
                )

                SensorValue(
                    label = "Position",
                    value = if (latitude != null && longitude != null) {
                        String.format(Locale.US, "%.4f\n%.4f", latitude, longitude)
                    } else {
                        "--"
                    },
                )
            }

            // Row 3: Heading Magnetic (compass), Heading True (computed), Declination
            Spacer(modifier = Modifier.height(12.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                SensorValue(
                    label = "Hdg (Mag)",
                    value = magneticHeadingDegrees?.let {
                        String.format(Locale.US, "%.0f\u00B0", it)
                    } ?: "--",
                )

                // True heading = magnetic heading + declination
                val trueHeading = if (magneticHeadingDegrees != null && magneticDeclinationDegrees != null) {
                    var hdg = magneticHeadingDegrees + magneticDeclinationDegrees
                    if (hdg < 0) hdg += 360.0
                    if (hdg >= 360) hdg -= 360.0
                    hdg
                } else {
                    null
                }
                SensorValue(
                    label = "Hdg (True)",
                    value = trueHeading?.let {
                        String.format(Locale.US, "%.0f\u00B0", it)
                    } ?: "--",
                )

                SensorValue(
                    label = "Track (GPS)",
                    value = gpsBearingDegrees?.let {
                        String.format(Locale.US, "%.0f\u00B0", it)
                    } ?: "--",
                )
            }

            // Row 4: Declination
            Spacer(modifier = Modifier.height(12.dp))

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                SensorValue(
                    label = "Declination",
                    value = magneticDeclinationDegrees?.let {
                        val sign = if (it >= 0) "E" else "W"
                        String.format(Locale.US, "%.1f\u00B0 %s", Math.abs(it), sign)
                    } ?: "--",
                )
            }

            Spacer(modifier = Modifier.height(12.dp))
            HorizontalDivider()
            Spacer(modifier = Modifier.height(12.dp))

            // Altimeter section
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                // Altimeter reading
                val altimeterFt = if (pressureHpa != null && groundPressureHpa != null && groundPressureHpa > 0) {
                    145366.45 * (1 - Math.pow(pressureHpa / groundPressureHpa, 0.190284))
                } else {
                    null
                }
                SensorValue(
                    label = "Altimeter",
                    value = altimeterFt?.let { String.format(Locale.US, "%.0f ft", it) } ?: "--",
                )

                // Altimeter setting (the reference pressure)
                SensorValue(
                    label = "Altimeter Setting",
                    value = groundPressureHpa?.let {
                        val inHg = it * 0.02953
                        String.format(Locale.US, "%.2f inHg\n%.1f hPa", inHg, it)
                    } ?: "Not set",
                )

                // Set button
                TextButton(
                    onClick = onSetAltimeter,
                    enabled = pressureHpa != null,
                    contentPadding = ButtonDefaults.TextButtonContentPadding,
                ) {
                    Text("Set")
                }
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
