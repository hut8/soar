package com.soar.tracker.ui.screens

import java.util.Locale
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import com.soar.tracker.data.api.NearbyAircraftInfo
import com.soar.tracker.data.api.NearbyAirportInfo
import com.soar.tracker.service.TrackingService
import com.soar.tracker.ui.theme.Green
import com.soar.tracker.ui.theme.Red
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

private const val NM_TO_METERS = 1852.0

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LiveViewScreen(modifier: Modifier = Modifier) {
    val latestResponse by TrackingService.latestResponse.collectAsState()
    val isTracking by TrackingService.isTracking.collectAsState()
    val sensorData by TrackingService.lastSensorData.collectAsState()

    val aircraft = latestResponse?.nearbyAircraft ?: emptyList()
    val airports = latestResponse?.nearbyAirports ?: emptyList()
    val userLat = sensorData?.latitude
    val userLon = sensorData?.longitude
    val userHeading = sensorData?.magneticHeadingDegrees?.toFloat() ?: 0f

    var northUp by remember { mutableStateOf(true) }

    // Range rings in nautical miles
    val rangeRingsNm = listOf(1.0, 2.0, 5.0, 10.0, 25.0, 50.0)
    // Scale: the outermost ring fills the canvas
    val maxRangeNm = rangeRingsNm.last()

    val ringColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.4f)
    val ringLabelColor = MaterialTheme.colorScheme.onSurfaceVariant
    val aircraftColor = Green
    val airportColor = MaterialTheme.colorScheme.tertiary
    val userColor = MaterialTheme.colorScheme.primary
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
            val mapRotation = if (northUp) 0f else -userHeading

            Canvas(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .background(MaterialTheme.colorScheme.surface),
            ) {
                val cx = size.width / 2f
                val cy = size.height / 2f
                val maxRadius = min(cx, cy) * 0.9f
                val metersPerPixel = (maxRangeNm * NM_TO_METERS).toFloat() / maxRadius

                // Draw range rings
                for (rangeNm in rangeRingsNm) {
                    val ringRadius = ((rangeNm * NM_TO_METERS) / metersPerPixel).toFloat()
                    drawCircle(
                        color = ringColor,
                        radius = ringRadius,
                        center = Offset(cx, cy),
                        style = Stroke(width = 1f),
                    )

                    // Range label at top of ring
                    val labelText = if (rangeNm >= 1.0 && rangeNm == rangeNm.toLong().toDouble()) {
                        "${rangeNm.toInt()} NM"
                    } else {
                        "$rangeNm NM"
                    }
                    val labelAngleRad = Math.toRadians((mapRotation - 90.0).toDouble())
                    val labelX = cx + ringRadius * cos(labelAngleRad).toFloat()
                    val labelY = cy + ringRadius * sin(labelAngleRad).toFloat()
                    drawContext.canvas.nativeCanvas.drawText(
                        labelText,
                        labelX,
                        labelY - 4f,
                        android.graphics.Paint().apply {
                            color = ringLabelColor.hashCode()
                            textSize = labelTextSize
                            textAlign = android.graphics.Paint.Align.CENTER
                            isAntiAlias = true
                        },
                    )
                }

                // North indicator
                val northAngleRad = Math.toRadians(mapRotation.toDouble() - 90.0)
                val northX = cx + (maxRadius + 15f) * cos(northAngleRad).toFloat()
                val northY = cy + (maxRadius + 15f) * sin(northAngleRad).toFloat()
                drawContext.canvas.nativeCanvas.drawText(
                    "N",
                    northX,
                    northY + labelTextSize / 3f,
                    android.graphics.Paint().apply {
                        color = textColor.hashCode()
                        textSize = labelTextSize * 1.4f
                        textAlign = android.graphics.Paint.Align.CENTER
                        isFakeBoldText = true
                        isAntiAlias = true
                    },
                )

                // Draw airports
                for (airport in airports) {
                    val pos = latLonToScreen(
                        userLat, userLon,
                        airport.latitude, airport.longitude,
                        cx, cy, metersPerPixel, mapRotation,
                    ) ?: continue

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
                        android.graphics.Paint().apply {
                            color = airportColor.hashCode()
                            textSize = labelTextSize
                            textAlign = android.graphics.Paint.Align.CENTER
                            isAntiAlias = true
                        },
                    )
                }

                // Draw aircraft
                for (ac in aircraft) {
                    val pos = latLonToScreen(
                        userLat, userLon,
                        ac.latitude, ac.longitude,
                        cx, cy, metersPerPixel, mapRotation,
                    ) ?: continue

                    // Aircraft triangle, rotated by track
                    val trackDeg = ac.trackDegrees?.toFloat() ?: 0f
                    val rotation = trackDeg + mapRotation
                    val triSize = 8f

                    rotate(rotation, pivot = pos) {
                        val triPath = Path().apply {
                            moveTo(pos.x, pos.y - triSize)
                            lineTo(pos.x + triSize * 0.6f, pos.y + triSize * 0.6f)
                            lineTo(pos.x, pos.y + triSize * 0.2f)
                            lineTo(pos.x - triSize * 0.6f, pos.y + triSize * 0.6f)
                            close()
                        }
                        drawPath(triPath, aircraftColor)
                    }

                    // Label
                    val label = ac.registration ?: ac.aircraftModel
                    drawContext.canvas.nativeCanvas.drawText(
                        label,
                        pos.x,
                        pos.y - triSize - 3f,
                        android.graphics.Paint().apply {
                            color = aircraftColor.hashCode()
                            textSize = labelTextSize
                            textAlign = android.graphics.Paint.Align.CENTER
                            isAntiAlias = true
                        },
                    )
                }

                // User position: filled circle at center
                drawCircle(
                    color = userColor,
                    radius = 5f,
                    center = Offset(cx, cy),
                )

                // User heading indicator line
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
    }
}

/**
 * Convert lat/lon to screen coordinates relative to user position.
 * Returns null if the point is outside the visible area.
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
): Offset? {
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
