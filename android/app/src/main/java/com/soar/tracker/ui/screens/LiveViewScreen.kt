package com.soar.tracker.ui.screens

import java.util.Locale
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.soar.tracker.data.api.NearbyAircraftInfo
import com.soar.tracker.service.TrackingService
import com.soar.tracker.ui.components.AircraftDetailRow
import com.soar.tracker.ui.theme.Green
import com.soar.tracker.ui.theme.Red
import com.soar.tracker.ui.util.calculateBearing
import com.soar.tracker.ui.util.drawAircraftIcon
import com.soar.tracker.ui.util.getAltitudeColor
import com.soar.tracker.ui.util.NM_TO_METERS
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin
import kotlin.math.sqrt

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LiveViewScreen(modifier: Modifier = Modifier) {
    val latestResponse by TrackingService.latestResponse.collectAsState()
    val isTracking by TrackingService.isTracking.collectAsState()
    val sensorData by TrackingService.lastSensorData.collectAsState()
    val liveHeading by TrackingService.liveHeadingDegrees.collectAsState()

    val aircraft = latestResponse?.nearbyAircraft ?: emptyList()
    val airports = latestResponse?.nearbyAirports ?: emptyList()
    val userLat = sensorData?.latitude
    val userLon = sensorData?.longitude
    // Prefer live heading (updates at ~50 Hz) over the GPS-paced snapshot (~3 s)
    // Nullable: null means heading is unknown, 0f means north
    val userHeading: Float? = liveHeading ?: sensorData?.magneticHeadingDegrees?.toFloat()

    var northUp by remember { mutableStateOf(true) }

    // Selected aircraft for detail popup
    var selectedAircraft by remember { mutableStateOf<NearbyAircraftInfo?>(null) }

    // Rendered aircraft positions for hit-testing taps
    val renderedPositions = remember { mutableListOf<Pair<NearbyAircraftInfo, Offset>>() }

    // Dynamic range controlled by pinch zoom (NM)
    var radarRangeNm by remember { mutableStateOf(50.0f) }
    val minRangeNm = 0.5f
    val maxRangeNm = 200f

    // Compute range rings based on current zoom level
    val rangeRingsNm = remember(radarRangeNm) {
        val r = radarRangeNm.toDouble()
        // Pick a ring spacing that gives 3-5 rings
        val spacing = when {
            r <= 1.0 -> 0.25
            r <= 2.0 -> 0.5
            r <= 5.0 -> 1.0
            r <= 10.0 -> 2.0
            r <= 25.0 -> 5.0
            r <= 50.0 -> 10.0
            r <= 100.0 -> 25.0
            else -> 50.0
        }
        val rings = mutableListOf<Double>()
        var ring = spacing
        while (ring <= r) {
            rings.add(ring)
            ring += spacing
        }
        rings
    }

    val groundPressureHpa by TrackingService.groundPressureHpa.collectAsState()

    val ringColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.4f)
    val aircraftColor = Green
    val airportColor = MaterialTheme.colorScheme.tertiary
    val userColor = MaterialTheme.colorScheme.primary
    val northLineColor = Red
    val textColor = MaterialTheme.colorScheme.onSurface

    val density = LocalDensity.current
    val labelTextSize = with(density) { 10.dp.toPx() }

    Scaffold(
        modifier = modifier,
        topBar = {
            TopAppBar(
                title = { Text("Live View") },
                actions = {
                    FilledTonalButton(
                        onClick = { northUp = !northUp },
                        modifier = Modifier.padding(end = 8.dp),
                    ) {
                        Text(if (northUp) "North Up" else "Heading Up")
                    }
                },
            )
        },
    ) { padding ->
        if (!isTracking || userLat == null || userLon == null) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = "Start tracking to see the radar view",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        } else {
            val mapRotation = if (northUp) 0f else -(userHeading ?: 0f)

            // Precompute telemetry values for the bottom panel
            // Prefer MSL-corrected altitude (accounts for geoid undulation)
            val altFt = (sensorData?.altitudeMslMeters ?: sensorData?.altitudeMeters)?.let { it * 3.28084 }
            val aglFt = latestResponse?.altitudeAglFeet
            val gsKnots = sensorData?.speedMps?.let { it * 1.94384 }
            val varioFpm = sensorData?.verticalSpeedMps?.let { it * 196.85 }
            val pressureHpa = sensorData?.pressureHpa
            val altimeterFt = if (pressureHpa != null && groundPressureHpa != null && groundPressureHpa!! > 0) {
                145366.45 * (1 - Math.pow(pressureHpa / groundPressureHpa!!, 0.190284))
            } else {
                null
            }
            val altSettingInHg = groundPressureHpa?.let { it * 0.02953 }
            val magneticDeclination = latestResponse?.magneticDeclinationDegrees
            val trueHeading: Double? = if (userHeading != null) {
                if (magneticDeclination != null) {
                    var hdg = userHeading + magneticDeclination
                    if (hdg < 0) hdg += 360.0
                    if (hdg >= 360) hdg -= 360.0
                    hdg
                } else {
                    userHeading.toDouble()
                }
            } else {
                null
            }

            // Distance and bearing to departure airport
            val flight = latestResponse?.flight
            val departureIdent = flight?.departureAirport
            val departureAirport = if (departureIdent != null) {
                airports.find { it.ident == departureIdent }
            } else {
                null
            }
            val distToDepartureNm = if (departureAirport != null) {
                val dLat = (departureAirport.latitude - userLat) * 111_320.0
                val dLon = (departureAirport.longitude - userLon) * 111_320.0 * cos(Math.toRadians(userLat))
                Math.sqrt(dLat * dLat + dLon * dLon) / NM_TO_METERS
            } else {
                null
            }
            val bearingToDeparture = if (departureAirport != null) {
                calculateBearing(userLat, userLon, departureAirport.latitude, departureAirport.longitude)
            } else {
                null
            }

            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(top = padding.calculateTopPadding()),
            ) {
                // Pre-create Paint objects outside Canvas to avoid allocation every frame
                val northPaint = remember(textColor, labelTextSize) {
                    android.graphics.Paint().apply {
                        color = textColor.toArgb()
                        textSize = labelTextSize * 1.4f
                        textAlign = android.graphics.Paint.Align.CENTER
                        isFakeBoldText = true
                        isAntiAlias = true
                    }
                }
                val airportLabelPaint = remember(airportColor, labelTextSize) {
                    android.graphics.Paint().apply {
                        color = airportColor.toArgb()
                        textSize = labelTextSize
                        textAlign = android.graphics.Paint.Align.CENTER
                        isAntiAlias = true
                    }
                }
                val aircraftLabelPaint = remember(aircraftColor, labelTextSize) {
                    android.graphics.Paint().apply {
                        color = aircraftColor.toArgb()
                        textSize = labelTextSize
                        textAlign = android.graphics.Paint.Align.CENTER
                        isAntiAlias = true
                    }
                }

                // Radar canvas with pinch-to-zoom
                Box(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth(),
                ) {
                Canvas(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(MaterialTheme.colorScheme.surface)
                        .pointerInput(Unit) {
                            detectTapGestures { tapOffset ->
                                var closest: NearbyAircraftInfo? = null
                                var closestDist = 60f
                                val positions = renderedPositions.toList()
                                for ((ac, pos) in positions) {
                                    val dx = tapOffset.x - pos.x
                                    val dy = tapOffset.y - pos.y
                                    val dist = sqrt(dx * dx + dy * dy)
                                    if (dist < closestDist) {
                                        closestDist = dist
                                        closest = ac
                                    }
                                }
                                if (closest != null) {
                                    selectedAircraft = closest
                                }
                            }
                        }
                        .pointerInput(Unit) {
                            detectTransformGestures { _, _, zoom, _ ->
                                radarRangeNm = (radarRangeNm / zoom).coerceIn(minRangeNm, maxRangeNm)
                            }
                        },
                ) {
                    val cx = size.width / 2f
                    val cy = size.height / 2f
                    val maxRadius = min(cx, cy) * 0.9f
                    val metersPerPixel = (radarRangeNm * NM_TO_METERS).toFloat() / maxRadius

                    // Draw range rings (no labels — they are illegible on mobile)
                    for (rangeNm in rangeRingsNm) {
                        val ringRadius = ((rangeNm * NM_TO_METERS) / metersPerPixel).toFloat()
                        drawCircle(
                            color = ringColor,
                            radius = ringRadius,
                            center = Offset(cx, cy),
                            style = Stroke(width = 1f),
                        )
                    }

                    // Red north reference line from center outward
                    val northAngleRad = Math.toRadians(mapRotation.toDouble() - 90.0)
                    val northLineEndX = cx + maxRadius * cos(northAngleRad).toFloat()
                    val northLineEndY = cy + maxRadius * sin(northAngleRad).toFloat()
                    drawLine(
                        color = northLineColor,
                        start = Offset(cx, cy),
                        end = Offset(northLineEndX, northLineEndY),
                        strokeWidth = 2f,
                    )

                    // North indicator label
                    val northLabelX = cx + (maxRadius + 15f) * cos(northAngleRad).toFloat()
                    val northLabelY = cy + (maxRadius + 15f) * sin(northAngleRad).toFloat()
                    drawContext.canvas.nativeCanvas.drawText(
                        "N",
                        northLabelX,
                        northLabelY + labelTextSize / 3f,
                        northPaint,
                    )

                    // Draw airports
                    for (airport in airports) {
                        val pos = latLonToScreen(
                            userLat, userLon,
                            airport.latitude, airport.longitude,
                            cx, cy, metersPerPixel, mapRotation,
                        )

                        // Airport diamond
                        val d = 6f
                        val diamondPath = Path().apply {
                            moveTo(pos.x, pos.y - d)
                            lineTo(pos.x + d, pos.y)
                            lineTo(pos.x, pos.y + d)
                            lineTo(pos.x - d, pos.y)
                            close()
                        }
                        drawPath(diamondPath, airportColor)

                        // Label
                        drawContext.canvas.nativeCanvas.drawText(
                            airport.ident,
                            pos.x,
                            pos.y - d - 3f,
                            airportLabelPaint,
                        )
                    }

                    // Draw aircraft
                    renderedPositions.clear()
                    for (ac in aircraft) {
                        val pos = latLonToScreen(
                            userLat, userLon,
                            ac.latitude, ac.longitude,
                            cx, cy, metersPerPixel, mapRotation,
                        )

                        renderedPositions.add(ac to pos)

                        // Aircraft icon with altitude-based coloring
                        val trackDeg = ac.trackDegrees?.toFloat() ?: 0f
                        val rotation = trackDeg + mapRotation
                        val altColor = getAltitudeColor(ac.altitudeFeet)
                        drawAircraftIcon(pos, rotation, 20f, altColor)

                        // Label
                        val label = ac.registration ?: ac.aircraftModel
                        drawContext.canvas.nativeCanvas.drawText(
                            label,
                            pos.x,
                            pos.y - 13f,
                            aircraftLabelPaint,
                        )
                    }

                    // User position: filled circle at center
                    drawCircle(
                        color = userColor,
                        radius = 5f,
                        center = Offset(cx, cy),
                    )

                    // User heading indicator line (only drawn when heading is known)
                    if (userHeading != null) {
                        val headingLineLen = 20f
                        val headingAngleRad = if (northUp) {
                            Math.toRadians(userHeading.toDouble() - 90.0)
                        } else {
                            Math.toRadians(-90.0) // Points straight up in heading-up mode
                        }
                        drawLine(
                            color = userColor,
                            start = Offset(cx, cy),
                            end = Offset(
                                cx + headingLineLen * cos(headingAngleRad).toFloat(),
                                cy + headingLineLen * sin(headingAngleRad).toFloat(),
                            ),
                            strokeWidth = 2f,
                        )
                    }
                }

                // Range display overlay (top-right of radar)
                val rangeText = if (radarRangeNm >= 1f) {
                    String.format(Locale.US, "%.0f NM", radarRangeNm)
                } else {
                    String.format(Locale.US, "%.1f NM", radarRangeNm)
                }
                Text(
                    text = rangeText,
                    modifier = Modifier
                        .align(Alignment.TopEnd)
                        .padding(8.dp)
                        .background(
                            MaterialTheme.colorScheme.surface.copy(alpha = 0.75f),
                            MaterialTheme.shapes.small,
                        )
                        .padding(horizontal = 8.dp, vertical = 4.dp),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                } // Box

                // Bottom telemetry panel
                HorizontalDivider()
                TelemetryPanel(
                    altitudeFt = altFt,
                    aglFt = aglFt,
                    altimeterFt = altimeterFt,
                    altSettingInHg = altSettingInHg,
                    groundSpeedKnots = gsKnots,
                    headingDeg = trueHeading,
                    varioFpm = varioFpm,
                    distToDepartureNm = distToDepartureNm,
                    bearingToDepartureDeg = bearingToDeparture,
                    departureIdent = departureIdent,
                )
            }

            // Aircraft detail popup
            selectedAircraft?.let { ac ->
                LiveViewAircraftPopup(
                    aircraft = ac,
                    userLat = userLat,
                    userLon = userLon,
                    heading = userHeading,
                    onDismiss = { selectedAircraft = null },
                )
            }
        }
    }
}

@Composable
private fun TelemetryPanel(
    altitudeFt: Double?,
    aglFt: Int?,
    altimeterFt: Double?,
    altSettingInHg: Double?,
    groundSpeedKnots: Double?,
    headingDeg: Double?,
    varioFpm: Double?,
    distToDepartureNm: Double?,
    bearingToDepartureDeg: Double?,
    departureIdent: String?,
) {
    val mono = FontFamily.Monospace
    val varioColor = when {
        varioFpm == null -> Color.Unspecified
        varioFpm > 50 -> Green
        varioFpm < -50 -> Red
        else -> Color.Unspecified
    }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .background(MaterialTheme.colorScheme.surfaceVariant)
            .padding(horizontal = 8.dp, vertical = 6.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        // Row 1: Altitude | AGL | Altimeter | Alt Setting
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            TelemetryCell(
                label = "Alt",
                value = altitudeFt?.let { String.format(Locale.US, "%.0f ft", it) } ?: "--",
                mono = mono,
            )
            TelemetryCell(
                label = "AGL",
                value = aglFt?.let { String.format(Locale.US, "%d ft", it) } ?: "--",
                mono = mono,
            )
            TelemetryCell(
                label = "Altimeter",
                value = altimeterFt?.let { String.format(Locale.US, "%.0f ft", it) } ?: "--",
                mono = mono,
            )
            TelemetryCell(
                label = "Setting",
                value = altSettingInHg?.let { String.format(Locale.US, "%.2f\"", it) } ?: "--",
                mono = mono,
            )
        }
        // Row 2: Ground Speed | Heading | Vario
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            TelemetryCell(
                label = "Ground Speed",
                value = groundSpeedKnots?.let { String.format(Locale.US, "%.0f kt", it) } ?: "--",
                mono = mono,
            )
            TelemetryCell(
                label = "Heading",
                value = headingDeg?.let { String.format(Locale.US, "%03.0f\u00B0", it) } ?: "--",
                mono = mono,
            )
            TelemetryCell(
                label = "Vario",
                value = varioFpm?.let {
                    val sign = if (it >= 0) "+" else ""
                    String.format(Locale.US, "%s%.0f fpm", sign, it)
                } ?: "--",
                mono = mono,
                valueColor = varioColor,
            )
        }
        // Row 3: Distance & bearing to departure airport (only if applicable)
        if (departureIdent != null && distToDepartureNm != null && bearingToDepartureDeg != null) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                TelemetryCell(
                    label = "Home ($departureIdent)",
                    value = String.format(Locale.US, "%.1f NM", distToDepartureNm),
                    mono = mono,
                )
                TelemetryCell(
                    label = "Fly heading",
                    value = String.format(Locale.US, "%03.0f\u00B0", bearingToDepartureDeg),
                    mono = mono,
                )
            }
        }
    }
}

@Composable
private fun RowScope.TelemetryCell(
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
            style = MaterialTheme.typography.bodyMedium,
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

/**
 * Convert lat/lon to screen coordinates relative to user position.
 */
private fun latLonToScreen(
    userLat: Double,
    userLon: Double,
    targetLat: Double,
    targetLon: Double,
    cx: Float,
    cy: Float,
    metersPerPixel: Float,
    mapRotation: Float,
): Offset {
    // Approximate meters offset using equirectangular projection
    val dLatMeters = (targetLat - userLat) * 111_320.0
    val dLonMeters = (targetLon - userLon) * 111_320.0 * cos(Math.toRadians(userLat))

    // Screen offset (before rotation): x = east, y = north (inverted for screen coords)
    val rawX = (dLonMeters / metersPerPixel).toFloat()
    val rawY = (-dLatMeters / metersPerPixel).toFloat() // negative because screen Y goes down

    // Apply map rotation
    val rotRad = Math.toRadians(mapRotation.toDouble())
    val rotX = rawX * cos(rotRad).toFloat() - rawY * sin(rotRad).toFloat()
    val rotY = rawX * sin(rotRad).toFloat() + rawY * cos(rotRad).toFloat()

    val screenX = cx + rotX
    val screenY = cy + rotY

    return Offset(screenX, screenY)
}

/** Aircraft detail popup for live view — tap an aircraft on the radar to see details. */
@Composable
private fun LiveViewAircraftPopup(
    aircraft: NearbyAircraftInfo,
    userLat: Double,
    userLon: Double,
    heading: Float?,
    onDismiss: () -> Unit,
) {
    val distNm = aircraft.distanceMeters / NM_TO_METERS
    val bearing = calculateBearing(userLat, userLon, aircraft.latitude, aircraft.longitude)
    val arrowRotation = if (heading != null) (bearing - heading).toFloat() else null

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black.copy(alpha = 0.5f))
            .clickable(onClick = onDismiss),
        contentAlignment = Alignment.Center,
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(32.dp)
                .background(
                    MaterialTheme.colorScheme.surface,
                    RoundedCornerShape(16.dp),
                )
                .clickable {} // Consume clicks on the popup
                .padding(20.dp),
        ) {
            // Header
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    modifier = Modifier.weight(1f),
                ) {
                    if (arrowRotation != null) {
                        Text(
                            text = "\u2191",
                            color = Green,
                            style = MaterialTheme.typography.headlineMedium,
                            modifier = Modifier.rotate(arrowRotation),
                        )
                    }
                    Column {
                        Text(
                            text = aircraft.registration ?: aircraft.aircraftModel,
                            style = MaterialTheme.typography.titleLarge,
                        )
                        if (aircraft.registration != null) {
                            Text(
                                text = aircraft.aircraftModel,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                }
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Default.Close, contentDescription = "Close")
                }
            }

            Spacer(modifier = Modifier.height(12.dp))
            HorizontalDivider()
            Spacer(modifier = Modifier.height(12.dp))

            AircraftDetailRow("Altitude", aircraft.altitudeFeet?.let {
                String.format(Locale.US, "%.0f ft", it)
            } ?: "Unknown")
            AircraftDetailRow("Ground Speed", aircraft.groundSpeedKnots?.let {
                String.format(Locale.US, "%.0f kt", it)
            } ?: "Unknown")
            AircraftDetailRow("Track", aircraft.trackDegrees?.let {
                String.format(Locale.US, "%03.0f\u00B0", it)
            } ?: "Unknown")
            aircraft.climbFpm?.let { climb ->
                val sign = if (climb >= 0) "+" else ""
                val climbColor = when {
                    climb > 50 -> Green
                    climb < -50 -> Red
                    else -> Color.Unspecified
                }
                AircraftDetailRow(
                    "Climb Rate",
                    String.format(Locale.US, "%s%.0f fpm", sign, climb),
                    valueColor = climbColor,
                )
            }
            AircraftDetailRow("Distance", String.format(Locale.US, "%.1f nm", distNm))
            AircraftDetailRow("Bearing", String.format(Locale.US, "%03.0f\u00B0", bearing))
            AircraftDetailRow("Position", String.format(
                Locale.US, "%.4f, %.4f", aircraft.latitude, aircraft.longitude,
            ))
            aircraft.lastFixAt?.let {
                val timeStr = if (it.contains("T")) it.substringAfter("T").take(8) else it
                AircraftDetailRow("Last Seen", timeStr)
            }
        }
    }
}
