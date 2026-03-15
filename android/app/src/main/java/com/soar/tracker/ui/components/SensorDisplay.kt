package com.soar.tracker.ui.components

import java.util.Locale
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.soar.tracker.ui.theme.Green
import com.soar.tracker.ui.theme.Red
import kotlin.math.abs
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
    val mono = FontFamily.Monospace

    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant,
        ),
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text("Sensors", style = MaterialTheme.typography.titleMedium)
            Spacer(modifier = Modifier.height(8.dp))

            // --- Grid rows ---

            // Row 1: G-Force | Vario | Pressure
            val gForce = if (accelX != null && accelY != null && accelZ != null) {
                sqrt(accelX * accelX + accelY * accelY + accelZ * accelZ)
            } else {
                null
            }
            val varioFpm = verticalSpeedMps?.let { it * 196.85 }
            val varioColor = when {
                varioFpm == null -> Color.Unspecified
                varioFpm > 50 -> Green
                varioFpm < -50 -> Red
                else -> Color.Unspecified
            }

            GridRow {
                GridCell(
                    label = "G-Force",
                    value = gForce?.let { String.format(Locale.US, "%.1f G", it) } ?: "--",
                    mono = mono,
                )
                GridCell(
                    label = "Vario",
                    value = varioFpm?.let {
                        val sign = if (it >= 0) "+" else ""
                        String.format(Locale.US, "%s%.0f fpm", sign, it)
                    } ?: "--",
                    mono = mono,
                    valueColor = varioColor,
                )
                GridCell(
                    label = "Pressure",
                    value = pressureHpa?.let {
                        val inHg = it * 0.02953
                        String.format(Locale.US, "%.2f inHg", inHg)
                    } ?: "--",
                    mono = mono,
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Row 2: GNSS Alt | AGL | Position
            val altFt = altitudeMeters?.let { it * 3.28084 }
            GridRow {
                GridCell(
                    label = "GNSS Alt",
                    value = altFt?.let { String.format(Locale.US, "%.0f ft", it) } ?: "--",
                    mono = mono,
                )
                GridCell(
                    label = "AGL",
                    value = altitudeAglFeet?.let { String.format(Locale.US, "%d ft", it) } ?: "--",
                    mono = mono,
                )
                GridCell(
                    label = "Position",
                    value = if (latitude != null && longitude != null) {
                        val latDir = if (latitude >= 0) "N" else "S"
                        val lonDir = if (longitude >= 0) "E" else "W"
                        String.format(
                            Locale.US, "%.4f\u00B0 %s\n%.4f\u00B0 %s",
                            abs(latitude), latDir, abs(longitude), lonDir,
                        )
                    } else {
                        "--"
                    },
                    mono = mono,
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            // Row 3: Hdg (Mag) | Hdg (True) | Track (GPS)
            val trueHeading = if (magneticHeadingDegrees != null && magneticDeclinationDegrees != null) {
                var hdg = magneticHeadingDegrees + magneticDeclinationDegrees
                if (hdg < 0) hdg += 360.0
                if (hdg >= 360) hdg -= 360.0
                hdg
            } else {
                null
            }

            GridRow {
                GridCell(
                    label = "Hdg (Mag)",
                    value = magneticHeadingDegrees?.let {
                        String.format(Locale.US, "%.0f\u00B0", it)
                    } ?: "--",
                    mono = mono,
                )
                GridCell(
                    label = "Hdg (True)",
                    value = trueHeading?.let {
                        String.format(Locale.US, "%.0f\u00B0", it)
                    } ?: "--",
                    mono = mono,
                )
                GridCell(
                    label = "Track (GPS)",
                    value = gpsBearingDegrees?.let {
                        String.format(Locale.US, "%.0f\u00B0", it)
                    } ?: "--",
                    mono = mono,
                )
            }

            Spacer(modifier = Modifier.height(12.dp))
            HorizontalDivider()
            Spacer(modifier = Modifier.height(12.dp))

            // Altimeter section
            val altimeterFt = if (pressureHpa != null && groundPressureHpa != null && groundPressureHpa > 0) {
                145366.45 * (1 - Math.pow(pressureHpa / groundPressureHpa, 0.190284))
            } else {
                null
            }

            GridRow(verticalAlignment = Alignment.CenterVertically) {
                GridCell(
                    label = "Altimeter",
                    value = altimeterFt?.let { String.format(Locale.US, "%.0f ft", it) } ?: "--",
                    mono = mono,
                )
                GridCell(
                    label = "Alt Setting",
                    value = groundPressureHpa?.let {
                        val inHg = it * 0.02953
                        String.format(Locale.US, "%.2f inHg", inHg)
                    } ?: "Not set",
                    mono = mono,
                )
                Column(
                    modifier = Modifier.weight(1f),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
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
}

@Composable
private fun GridRow(
    verticalAlignment: Alignment.Vertical = Alignment.Top,
    content: @Composable RowScope.() -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = verticalAlignment,
        content = content,
    )
}

@Composable
private fun RowScope.GridCell(
    label: String,
    value: String,
    mono: FontFamily,
    valueColor: Color = Color.Unspecified,
) {
    Column(
        modifier = Modifier.weight(1f),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = value,
            style = MaterialTheme.typography.titleMedium,
            fontFamily = mono,
            color = valueColor,
            textAlign = TextAlign.Center,
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
}
