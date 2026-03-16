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
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.soar.tracker.service.TrackingService
import com.soar.tracker.ui.theme.Green
import com.soar.tracker.ui.util.ARScreenPosition
import com.soar.tracker.ui.util.projectToScreen
import com.soar.tracker.ui.util.smoothAngle
import com.soar.tracker.ui.util.smoothLinear
import com.soar.tracker.ui.util.toARAircraftPosition
import java.util.Locale

private val RANGE_PRESETS = listOf(10, 25, 50, 100)
private const val SMOOTHING_ALPHA = 0.3f

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

    // Range selector
    var rangeNm by remember { mutableIntStateOf(50) }

    // FOV based on orientation
    val isPortrait = configuration.orientation == Configuration.ORIENTATION_PORTRAIT
    val fovH = if (isPortrait) 45f else 60f
    val fovV = if (isPortrait) 60f else 45f

    // Smoothed heading and pitch
    var smoothedHeading by remember { mutableFloatStateOf(0f) }
    var smoothedPitch by remember { mutableFloatStateOf(0f) }
    var hasInitialized by remember { mutableStateOf(false) }

    // Update smoothed values when sensors change
    val rawHeading = liveHeading
    val rawPitch = livePitch
    if (rawHeading != null && rawPitch != null) {
        if (!hasInitialized) {
            smoothedHeading = rawHeading
            smoothedPitch = rawPitch
            hasInitialized = true
        } else {
            smoothedHeading = smoothAngle(smoothedHeading, rawHeading, SMOOTHING_ALPHA)
            smoothedPitch = smoothLinear(smoothedPitch, rawPitch, SMOOTHING_ALPHA)
        }
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
    val compassPaint = remember(labelTextSize) {
        android.graphics.Paint().apply {
            color = android.graphics.Color.WHITE
            textSize = labelTextSize * 1.3f
            textAlign = android.graphics.Paint.Align.CENTER
            isAntiAlias = true
            isFakeBoldText = true
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
        if (userLat != null && userLon != null && hasInitialized) {
            Canvas(modifier = Modifier.fillMaxSize()) {
                val w = size.width
                val h = size.height

                for (ac in aircraft) {
                    if (ac.distanceMeters / 1852.0 > rangeNm) continue

                    val arPos = toARAircraftPosition(ac, userLat, userLon, userAltM)
                    val screenPos = projectToScreen(
                        arPos, smoothedHeading, smoothedPitch, fovH, fovV, w, h,
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
                        Locale.US, "%.1fnm", ac.distanceMeters / 1852.0,
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
        }

        // Compass overlay (top-center)
        if (hasInitialized) {
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
                    Locale.US, "%03.0f\u00B0", smoothedHeading.toDouble(),
                )
                val cardinal = when {
                    smoothedHeading < 22.5f || smoothedHeading >= 337.5f -> "N"
                    smoothedHeading < 67.5f -> "NE"
                    smoothedHeading < 112.5f -> "E"
                    smoothedHeading < 157.5f -> "SE"
                    smoothedHeading < 202.5f -> "S"
                    smoothedHeading < 247.5f -> "SW"
                    smoothedHeading < 292.5f -> "W"
                    else -> "NW"
                }
                Text(
                    text = "$cardinal  $headingText",
                    color = Color.White,
                    style = MaterialTheme.typography.titleMedium,
                )
            }
        }

        // Range selector (bottom row)
        Row(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .padding(bottom = 16.dp, start = 16.dp, end = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
        ) {
            for (preset in RANGE_PRESETS) {
                FilledTonalButton(
                    onClick = { rangeNm = preset },
                ) {
                    Text(
                        text = "${preset} NM",
                        color = if (rangeNm == preset) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        },
                    )
                }
            }
        }
    }
}
