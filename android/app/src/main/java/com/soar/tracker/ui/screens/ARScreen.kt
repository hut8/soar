package com.soar.tracker.ui.screens

import android.Manifest
import android.content.res.Configuration
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.List
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
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
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.soar.tracker.data.api.NearbyAircraftInfo
import com.soar.tracker.service.TrackingService
import com.soar.tracker.ui.theme.Green
import com.soar.tracker.ui.util.ARAircraftPosition
import com.soar.tracker.ui.util.normalizeBearing
import com.soar.tracker.ui.util.projectToScreen
import com.soar.tracker.ui.util.toARAircraftPosition
import java.util.Locale
import kotlin.math.roundToInt

private const val NM_TO_METERS = 1852.0

@Composable
fun ARScreen(modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val configuration = LocalConfiguration.current

    // Tracking state
    val isTracking by TrackingService.isTracking.collectAsState()
    val latestResponse by TrackingService.latestResponse.collectAsState()
    val sensorData by TrackingService.lastSensorData.collectAsState()
    val liveHeading by TrackingService.liveHeadingDegrees.collectAsState()
    val livePitch by TrackingService.livePitchDegrees.collectAsState()

    val aircraft = latestResponse?.nearbyAircraft ?: emptyList()
    val userLat = sensorData?.latitude
    val userLon = sensorData?.longitude
    val userAltM = sensorData?.altitudeMslMeters ?: sensorData?.altitudeMeters ?: 0.0

    // Camera permission
    var cameraPermissionGranted by remember { mutableStateOf(false) }
    val cameraPermissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted -> cameraPermissionGranted = granted }

    LaunchedEffect(Unit) {
        cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
    }

    // Range slider (5–150 NM)
    var rangeNm by remember { mutableFloatStateOf(50f) }

    // Aircraft list modal
    var showAircraftList by remember { mutableStateOf(false) }

    // Target aircraft for direction indicator
    var targetAircraftId by remember { mutableStateOf<String?>(null) }

    // FOV based on orientation
    val isPortrait = configuration.orientation == Configuration.ORIENTATION_PORTRAIT
    val fovH = if (isPortrait) 45f else 60f
    val fovV = if (isPortrait) 60f else 45f

    // Use raw sensor values directly — smoothing adds visible lag vs camera
    val currentHeading = liveHeading
    val currentPitch = livePitch
    val hasSensors = currentHeading != null && currentPitch != null

    // Not tracking state
    if (!isTracking) {
        Box(
            modifier = modifier.fillMaxSize(),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = "Start tracking to use AR view",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        return
    }

    // Camera permission denied
    if (!cameraPermissionGranted) {
        Box(
            modifier = modifier.fillMaxSize(),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = "Camera permission is required for AR view",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        return
    }

    val density = LocalDensity.current
    val labelTextSize = with(density) { 11.dp.toPx() }

    // Pre-create Paint for aircraft labels
    val labelPaint = remember(labelTextSize) {
        android.graphics.Paint().apply {
            color = android.graphics.Color.WHITE
            textSize = labelTextSize
            textAlign = android.graphics.Paint.Align.CENTER
            isAntiAlias = true
            isFakeBoldText = true
        }
    }
    val pillPaint = remember {
        android.graphics.Paint().apply {
            color = android.graphics.Color.argb(200, 0, 0, 0)
            isAntiAlias = true
        }
    }

    Box(modifier = modifier.fillMaxSize()) {
        // Camera preview
        AndroidView(
            factory = { ctx ->
                val previewView = PreviewView(ctx).apply {
                    scaleType = PreviewView.ScaleType.FILL_CENTER
                    implementationMode = PreviewView.ImplementationMode.COMPATIBLE
                }
                val cameraProviderFuture = ProcessCameraProvider.getInstance(ctx)
                cameraProviderFuture.addListener({
                    val cameraProvider = cameraProviderFuture.get()
                    val preview = Preview.Builder().build().also {
                        it.surfaceProvider = previewView.surfaceProvider
                    }
                    try {
                        cameraProvider.unbindAll()
                        cameraProvider.bindToLifecycle(
                            lifecycleOwner,
                            CameraSelector.DEFAULT_BACK_CAMERA,
                            preview,
                        )
                    } catch (_: Exception) {
                        // Camera binding failed — preview will be blank
                    }
                }, ctx.mainExecutor)
                previewView
            },
            modifier = Modifier.fillMaxSize(),
        )

        // Unbind camera when leaving the screen
        DisposableEffect(lifecycleOwner) {
            onDispose {
                try {
                    val cameraProvider = ProcessCameraProvider.getInstance(context).get()
                    cameraProvider.unbindAll()
                } catch (_: Exception) {
                    // Ignore
                }
            }
        }

        // Aircraft overlay
        if (userLat != null && userLon != null && hasSensors) {
            val heading = currentHeading!!
            val pitch = currentPitch!!

            Canvas(modifier = Modifier.fillMaxSize()) {
                val w = size.width
                val h = size.height

                for (ac in aircraft) {
                    if (ac.distanceMeters / NM_TO_METERS > rangeNm) continue

                    val arPos = toARAircraftPosition(ac, userLat, userLon, userAltM)
                    val screenPos = projectToScreen(
                        arPos, heading, pitch, fovH, fovV, w, h,
                    )
                    if (!screenPos.visible) continue

                    val center = Offset(screenPos.x, screenPos.y)

                    // Aircraft triangle icon
                    val triSize = 12f
                    val trackDeg = ac.trackDegrees?.toFloat() ?: 0f
                    rotate(trackDeg, pivot = center) {
                        val triPath = Path().apply {
                            moveTo(center.x, center.y - triSize)
                            lineTo(center.x + triSize * 0.6f, center.y + triSize * 0.6f)
                            lineTo(center.x, center.y + triSize * 0.2f)
                            lineTo(center.x - triSize * 0.6f, center.y + triSize * 0.6f)
                            close()
                        }
                        drawPath(triPath, Color.White)
                        drawPath(triPath, Green, style = Stroke(width = 1.5f))
                    }

                    // Info label below marker
                    val label = ac.registration ?: ac.aircraftModel
                    val altLabel = ac.altitudeFeet?.let {
                        String.format(Locale.US, "%.0fft", it)
                    } ?: ""
                    val distLabel = String.format(
                        Locale.US, "%.1fnm", ac.distanceMeters / NM_TO_METERS,
                    )
                    val infoText = "$label  $altLabel  $distLabel"

                    val textWidth = labelPaint.measureText(infoText)
                    val pillPadH = 6f
                    val pillPadV = 3f
                    val pillTop = center.y + triSize + 4f
                    val pillRect = android.graphics.RectF(
                        center.x - textWidth / 2 - pillPadH,
                        pillTop,
                        center.x + textWidth / 2 + pillPadH,
                        pillTop + labelTextSize + pillPadV * 2,
                    )
                    drawContext.canvas.nativeCanvas.drawRoundRect(
                        pillRect, 8f, 8f, pillPaint,
                    )
                    drawContext.canvas.nativeCanvas.drawText(
                        infoText,
                        center.x,
                        pillTop + pillPadV + labelTextSize * 0.85f,
                        labelPaint,
                    )
                }
            }

            // Direction indicator for targeted aircraft
            val targetAc = aircraft.find { it.id == targetAircraftId }
            if (targetAc != null) {
                val arPos = toARAircraftPosition(targetAc, userLat, userLon, userAltM)
                val screenPos = projectToScreen(
                    arPos, heading, pitch, fovH, fovV,
                    // Use approximate screen size — exact size not critical for visibility check
                    with(density) { configuration.screenWidthDp.dp.toPx() },
                    with(density) { configuration.screenHeightDp.dp.toPx() },
                )
                if (!screenPos.visible) {
                    DirectionIndicator(
                        arPos = arPos,
                        heading = heading,
                        pitch = pitch,
                        fovH = fovH,
                        fovV = fovV,
                        onDismiss = { targetAircraftId = null },
                    )
                }
            }
        }

        // Compass overlay (top-center)
        if (hasSensors) {
            val heading = currentHeading!!
            Box(
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .padding(top = 16.dp)
                    .background(
                        Color.Black.copy(alpha = 0.6f),
                        RoundedCornerShape(8.dp),
                    )
                    .padding(horizontal = 16.dp, vertical = 8.dp),
            ) {
                val headingText = String.format(
                    Locale.US, "%03.0f\u00B0", heading.toDouble(),
                )
                val cardinal = when {
                    heading < 22.5f || heading >= 337.5f -> "N"
                    heading < 67.5f -> "NE"
                    heading < 112.5f -> "E"
                    heading < 157.5f -> "SE"
                    heading < 202.5f -> "S"
                    heading < 247.5f -> "SW"
                    heading < 292.5f -> "W"
                    else -> "NW"
                }
                Text(
                    text = "$cardinal  $headingText",
                    color = Color.White,
                    style = MaterialTheme.typography.titleMedium,
                )
            }
        }

        // Bottom controls: range slider + aircraft list button
        Row(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .background(Color.Black.copy(alpha = 0.5f))
                .padding(horizontal = 16.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            // Range slider
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = "${rangeNm.roundToInt()} NM",
                    color = Color.White,
                    style = MaterialTheme.typography.labelSmall,
                )
                Slider(
                    value = rangeNm,
                    onValueChange = { rangeNm = it },
                    valueRange = 5f..150f,
                    colors = SliderDefaults.colors(
                        thumbColor = Color.White,
                        activeTrackColor = Green,
                        inactiveTrackColor = Color.White.copy(alpha = 0.3f),
                    ),
                )
            }

            // Aircraft list button
            IconButton(
                onClick = { showAircraftList = true },
                modifier = Modifier
                    .background(Color.White.copy(alpha = 0.2f), CircleShape)
                    .size(48.dp),
            ) {
                Icon(
                    Icons.Default.List,
                    contentDescription = "Nearby aircraft",
                    tint = Color.White,
                )
            }
        }

        // Aircraft list modal
        if (showAircraftList && userLat != null && userLon != null) {
            AircraftListModal(
                aircraft = aircraft.filter { it.distanceMeters / NM_TO_METERS <= rangeNm },
                userLat = userLat,
                userLon = userLon,
                userAltM = userAltM,
                heading = currentHeading,
                onSelect = { ac ->
                    targetAircraftId = ac.id
                    showAircraftList = false
                },
                onDismiss = { showAircraftList = false },
            )
        }
    }
}

/** Full-screen modal listing nearby aircraft sorted by distance. */
@Composable
private fun AircraftListModal(
    aircraft: List<NearbyAircraftInfo>,
    userLat: Double,
    userLon: Double,
    userAltM: Double,
    heading: Float?,
    onSelect: (NearbyAircraftInfo) -> Unit,
    onDismiss: () -> Unit,
) {
    val sorted = remember(aircraft) {
        aircraft.sortedBy { it.distanceMeters }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black.copy(alpha = 0.85f))
            .clickable(onClick = onDismiss),
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
        ) {
            // Header
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "Nearby Aircraft (${sorted.size})",
                    color = Color.White,
                    style = MaterialTheme.typography.titleMedium,
                )
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Default.Close, contentDescription = "Close", tint = Color.White)
                }
            }

            if (sorted.isEmpty()) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(
                        text = "No aircraft in range",
                        color = Color.White.copy(alpha = 0.6f),
                        style = MaterialTheme.typography.bodyLarge,
                    )
                }
            } else {
                LazyColumn(
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    items(sorted, key = { it.id }) { ac ->
                        val distNm = ac.distanceMeters / NM_TO_METERS
                        val bearing = com.soar.tracker.ui.util.calculateBearing(
                            userLat, userLon, ac.latitude, ac.longitude,
                        )
                        // Arrow rotation: bearing relative to device heading
                        val arrowRotation = if (heading != null) {
                            (bearing - heading).toFloat()
                        } else {
                            bearing.toFloat()
                        }

                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .background(
                                    Color.White.copy(alpha = 0.1f),
                                    RoundedCornerShape(8.dp),
                                )
                                .clickable { onSelect(ac) }
                                .padding(12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                        ) {
                            // Direction arrow
                            Text(
                                text = "\u2191", // ↑
                                color = Green,
                                style = MaterialTheme.typography.titleLarge,
                                modifier = Modifier.rotate(arrowRotation),
                            )

                            // Info
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = ac.registration ?: ac.aircraftModel,
                                    color = Color.White,
                                    style = MaterialTheme.typography.bodyMedium,
                                )
                                val altText = ac.altitudeFeet?.let {
                                    String.format(Locale.US, "%.0f ft", it)
                                } ?: "—"
                                Text(
                                    text = "$altText  \u00B7  ${String.format(Locale.US, "%.1f nm", distNm)}  \u00B7  ${String.format(Locale.US, "%03.0f\u00B0", bearing)}",
                                    color = Color.White.copy(alpha = 0.7f),
                                    style = MaterialTheme.typography.bodySmall,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

/**
 * Edge-of-screen indicator pointing toward an off-screen target aircraft.
 * Shows an arrow at the screen edge and the aircraft registration + distance.
 */
@Composable
private fun DirectionIndicator(
    arPos: ARAircraftPosition,
    heading: Float,
    pitch: Float,
    fovH: Float,
    fovV: Float,
    onDismiss: () -> Unit,
) {
    val relativeBearing = normalizeBearing(arPos.bearing - heading).toFloat()
    val adjustedElevation = (arPos.elevation - pitch).toFloat()

    // Determine direction
    val isLeft = relativeBearing < -fovH / 4
    val isRight = relativeBearing > fovH / 4
    val isUp = adjustedElevation > fovV / 4
    val isDown = adjustedElevation < -fovV / 4

    // Arrow rotation in degrees (0 = up)
    val arrowAngle = Math.toDegrees(
        kotlin.math.atan2(relativeBearing.toDouble(), -adjustedElevation.toDouble()),
    ).toFloat()

    // Position at screen edge
    val alignment = when {
        isUp && isLeft -> Alignment.TopStart
        isUp && isRight -> Alignment.TopEnd
        isDown && isLeft -> Alignment.BottomStart
        isDown && isRight -> Alignment.BottomEnd
        isUp -> Alignment.TopCenter
        isDown -> Alignment.BottomCenter
        isLeft -> Alignment.CenterStart
        isRight -> Alignment.CenterEnd
        else -> Alignment.Center
    }

    val label = arPos.aircraft.registration ?: arPos.aircraft.aircraftModel
    val distText = String.format(Locale.US, "%.1f nm", arPos.distanceNm)

    Box(modifier = Modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .align(alignment)
                .padding(24.dp)
                .background(Color.Black.copy(alpha = 0.7f), RoundedCornerShape(8.dp))
                .clickable(onClick = onDismiss)
                .padding(12.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                text = "\u2191", // ↑
                color = Green,
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.rotate(arrowAngle),
            )
            Text(
                text = label,
                color = Color.White,
                style = MaterialTheme.typography.labelMedium,
            )
            Text(
                text = distText,
                color = Color.White.copy(alpha = 0.7f),
                style = MaterialTheme.typography.labelSmall,
            )
        }
    }
}
