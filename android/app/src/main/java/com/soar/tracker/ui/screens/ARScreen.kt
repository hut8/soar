package com.soar.tracker.ui.screens

import android.Manifest
import android.app.Activity
import android.content.res.Configuration
import android.opengl.GLSurfaceView
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.List
import androidx.compose.material3.HorizontalDivider
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
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import com.soar.tracker.ar.ARCoreSessionManager
import com.soar.tracker.data.api.NearbyAircraftInfo
import com.soar.tracker.ui.components.AircraftDetailRow
import com.soar.tracker.service.TrackingService
import com.soar.tracker.ui.theme.Green
import com.soar.tracker.ui.theme.Red
import com.soar.tracker.ui.util.ARAircraftPosition
import com.soar.tracker.ui.util.drawAircraftIcon
import com.soar.tracker.ui.util.getAltitudeColor
import com.soar.tracker.ui.util.normalizeBearing
import com.soar.tracker.ui.util.projectToScreen
import com.soar.tracker.ui.util.toARAircraftPosition
import android.content.pm.PackageManager
import androidx.core.content.ContextCompat
import com.soar.tracker.ui.util.NM_TO_METERS
import com.soar.tracker.ui.util.calculateBearing
import java.util.Locale
import kotlin.math.roundToInt
import kotlin.math.sqrt
private const val TAP_RADIUS_PX = 60f // Hit-test radius for tapping aircraft markers

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
    var cameraPermissionGranted by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED,
        )
    }
    val cameraPermissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted -> cameraPermissionGranted = granted }

    LaunchedEffect(Unit) {
        if (!cameraPermissionGranted) {
            cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
        }
    }

    // Range slider (5–150 NM)
    var rangeNm by remember { mutableFloatStateOf(50f) }

    // Aircraft list modal
    var showAircraftList by remember { mutableStateOf(false) }

    // Target aircraft for direction indicator
    var targetAircraftId by remember { mutableStateOf<String?>(null) }

    // Selected aircraft for detail popup (tapped on marker)
    var selectedAircraft by remember { mutableStateOf<NearbyAircraftInfo?>(null) }

    // Rendered aircraft positions for hit-testing taps
    // Updated each frame by the Canvas draw pass
    val renderedPositions = remember { mutableListOf<Pair<NearbyAircraftInfo, Offset>>() }

    // ARCore session manager (null until camera permission granted and ARCore attempted)
    var arCoreManager by remember { mutableStateOf<ARCoreSessionManager?>(null) }

    // Collect ARCore values (heading, pitch, FOV) — null when unavailable
    val arHeading by arCoreManager?.headingDegrees?.collectAsState()
        ?: remember { mutableStateOf(null) }
    val arPitch by arCoreManager?.pitchDegrees?.collectAsState()
        ?: remember { mutableStateOf(null) }
    val arFovH by arCoreManager?.fovHDegrees?.collectAsState()
        ?: remember { mutableStateOf(null) }
    val arFovV by arCoreManager?.fovVDegrees?.collectAsState()
        ?: remember { mutableStateOf(null) }
    val arAvailable by arCoreManager?.isAvailable?.collectAsState()
        ?: remember { mutableStateOf(false) }

    // Use ARCore heading/pitch when available, fall back to sensor
    val currentHeading = if (arAvailable) arHeading ?: liveHeading else liveHeading
    val currentPitch = if (arAvailable) arPitch ?: livePitch else livePitch
    val hasSensors = currentHeading != null && currentPitch != null

    // FOV: prefer ARCore's actual camera FOV, fall back to hardcoded estimates
    val isPortrait = configuration.orientation == Configuration.ORIENTATION_PORTRAIT
    val fovH = arFovH ?: if (isPortrait) 45f else 60f
    val fovV = arFovV ?: if (isPortrait) 60f else 45f

    // Feed sensor heading to ARCore for north calibration
    LaunchedEffect(liveHeading, arCoreManager) {
        val heading = liveHeading ?: return@LaunchedEffect
        arCoreManager?.calibrateNorth(heading)
    }

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
        // Camera preview: ARCore GLSurfaceView or CameraX fallback
        AndroidView(
            factory = { ctx ->
                val activity = ctx as Activity
                val manager = ARCoreSessionManager(activity)

                if (manager.createSession()) {
                    // ARCore available — use GLSurfaceView with ARCore rendering
                    liveHeading?.let { manager.calibrateNorth(it) }
                    val glView = GLSurfaceView(ctx).apply {
                        preserveEGLContextOnPause = true
                        setEGLContextClientVersion(2)
                        setEGLConfigChooser(8, 8, 8, 8, 16, 0)
                        setRenderer(manager)
                        renderMode = GLSurfaceView.RENDERMODE_CONTINUOUSLY
                    }
                    manager.resume()
                    arCoreManager = manager
                    glView
                } else {
                    // ARCore unavailable — fall back to CameraX
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
                    }, androidx.core.content.ContextCompat.getMainExecutor(ctx))
                    previewView
                }
            },
            modifier = Modifier.fillMaxSize(),
        )

        // Lifecycle management: pause/resume ARCore, cleanup on dispose
        DisposableEffect(lifecycleOwner) {
            val observer = LifecycleEventObserver { _, event ->
                when (event) {
                    Lifecycle.Event.ON_RESUME -> arCoreManager?.resume()
                    Lifecycle.Event.ON_PAUSE -> arCoreManager?.pause()
                    else -> {}
                }
            }
            lifecycleOwner.lifecycle.addObserver(observer)
            onDispose {
                lifecycleOwner.lifecycle.removeObserver(observer)
                arCoreManager?.destroy()
                arCoreManager = null
                try {
                    val cameraProvider = ProcessCameraProvider.getInstance(context).get()
                    cameraProvider.unbindAll()
                } catch (_: Exception) {
                    // Ignore
                }
            }
        }

        // Aircraft overlay with tap detection
        if (userLat != null && userLon != null && hasSensors) {
            val heading = currentHeading!!
            val pitch = currentPitch!!

            Canvas(
                modifier = Modifier
                    .fillMaxSize()
                    .pointerInput(Unit) {
                        detectTapGestures { tapOffset ->
                            // Find the closest rendered aircraft to the tap point
                            var closest: NearbyAircraftInfo? = null
                            var closestDist = TAP_RADIUS_PX
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
                    },
            ) {
                val w = size.width
                val h = size.height

                // Clear and rebuild rendered positions for hit-testing
                renderedPositions.clear()

                for (ac in aircraft) {
                    if (ac.distanceMeters / NM_TO_METERS > rangeNm) continue

                    val arPos = toARAircraftPosition(ac, userLat, userLon, userAltM)
                    val screenPos = projectToScreen(
                        arPos, heading, pitch, fovH, fovV, w, h,
                    )
                    if (!screenPos.visible) continue

                    val center = Offset(screenPos.x, screenPos.y)
                    renderedPositions.add(ac to center)

                    // Aircraft icon with altitude-based coloring
                    val iconSize = 24f
                    val trackDeg = ac.trackDegrees?.toFloat() ?: 0f
                    val altColor = getAltitudeColor(ac.altitudeFeet)
                    drawAircraftIcon(center, trackDeg, iconSize, altColor)

                    // Info label below marker
                    val label = ac.registration ?: ac.aircraftModel
                    val altLabel = ac.altitudeFeet?.let {
                        String.format(Locale.US, "%.0fft", it)
                    } ?: ""
                    val distLabel = String.format(
                        Locale.US, "%.1fnm", ac.distanceMeters / NM_TO_METERS,
                    )
                    val infoText = listOfNotNull(label, altLabel.ifEmpty { null }, distLabel).joinToString("  ")

                    val textWidth = labelPaint.measureText(infoText)
                    val pillPadH = 6f
                    val pillPadV = 3f
                    val pillTop = center.y + iconSize / 2f + 4f
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

        // Aircraft detail popup (shown when tapping a marker)
        selectedAircraft?.let { ac ->
            AircraftDetailPopup(
                aircraft = ac,
                userLat = userLat,
                userLon = userLon,
                heading = currentHeading,
                onDismiss = { selectedAircraft = null },
                onTrack = {
                    targetAircraftId = ac.id
                    selectedAircraft = null
                },
            )
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

/** Aircraft detail popup shown when tapping a marker on the AR view. */
@Composable
private fun AircraftDetailPopup(
    aircraft: NearbyAircraftInfo,
    userLat: Double?,
    userLon: Double?,
    heading: Float?,
    onDismiss: () -> Unit,
    onTrack: () -> Unit,
) {
    val distNm = aircraft.distanceMeters / NM_TO_METERS
    val bearing = if (userLat != null && userLon != null) {
        calculateBearing(
            userLat, userLon, aircraft.latitude, aircraft.longitude,
        )
    } else {
        null
    }
    val arrowRotation = if (bearing != null && heading != null) {
        (bearing - heading).toFloat()
    } else {
        null
    }

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
                .background(Color(0xFF1A1A2E), RoundedCornerShape(16.dp))
                .clickable {} // Consume clicks on the popup itself
                .padding(20.dp),
        ) {
            // Header with direction arrow and name
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
                            color = Color.White,
                            style = MaterialTheme.typography.titleLarge,
                        )
                        if (aircraft.registration != null) {
                            Text(
                                text = aircraft.aircraftModel,
                                color = Color.White.copy(alpha = 0.6f),
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                }
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Default.Close, contentDescription = "Close", tint = Color.White)
                }
            }

            Spacer(modifier = Modifier.height(12.dp))
            HorizontalDivider(color = Color.White.copy(alpha = 0.15f))
            Spacer(modifier = Modifier.height(12.dp))

            // Position section
            val arLabelColor = Color.White.copy(alpha = 0.5f)
            AircraftDetailRow("Altitude", aircraft.altitudeFeet?.let {
                String.format(Locale.US, "%.0f ft", it)
            } ?: "Unknown", labelColor = arLabelColor, valueColor = Color.White)
            AircraftDetailRow("Ground Speed", aircraft.groundSpeedKnots?.let {
                String.format(Locale.US, "%.0f kt", it)
            } ?: "Unknown", labelColor = arLabelColor, valueColor = Color.White)
            AircraftDetailRow("Track", aircraft.trackDegrees?.let {
                String.format(Locale.US, "%03.0f\u00B0", it)
            } ?: "Unknown", labelColor = arLabelColor, valueColor = Color.White)
            aircraft.climbFpm?.let { climb ->
                val sign = if (climb >= 0) "+" else ""
                val climbColor = when {
                    climb > 50 -> Green
                    climb < -50 -> Red
                    else -> Color.White
                }
                AircraftDetailRow(
                    "Climb Rate",
                    String.format(Locale.US, "%s%.0f fpm", sign, climb),
                    labelColor = arLabelColor,
                    valueColor = climbColor,
                )
            }
            AircraftDetailRow("Distance", String.format(Locale.US, "%.1f nm", distNm), labelColor = arLabelColor, valueColor = Color.White)
            bearing?.let {
                AircraftDetailRow("Bearing", String.format(Locale.US, "%03.0f\u00B0", it), labelColor = arLabelColor, valueColor = Color.White)
            }
            AircraftDetailRow("Position", String.format(
                Locale.US, "%.4f, %.4f", aircraft.latitude, aircraft.longitude,
            ), labelColor = arLabelColor, valueColor = Color.White)
            aircraft.lastFixAt?.let {
                // Show just the time portion if ISO format
                val timeStr = if (it.contains("T")) it.substringAfter("T").take(8) else it
                AircraftDetailRow("Last Seen", timeStr, labelColor = arLabelColor, valueColor = Color.White)
            }

            Spacer(modifier = Modifier.height(12.dp))

            // Track button
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .background(Green.copy(alpha = 0.2f), RoundedCornerShape(8.dp))
                    .clickable(onClick = onTrack)
                    .padding(12.dp),
                horizontalArrangement = Arrangement.Center,
            ) {
                Text(
                    text = "Track this aircraft",
                    color = Green,
                    style = MaterialTheme.typography.labelLarge,
                )
            }
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
                        val bearing = calculateBearing(
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
                                text = "\u2191", // up arrow
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
                                } ?: "\u2014"
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
                text = "\u2191", // up arrow
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
