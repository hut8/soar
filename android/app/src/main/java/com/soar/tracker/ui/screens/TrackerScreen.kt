package com.soar.tracker.ui.screens

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
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
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ExitToApp
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Stop
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Icon
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import com.soar.tracker.SoarTrackerApp
import com.soar.tracker.service.TrackingService
import com.soar.tracker.ui.components.FlightStatusCard
import com.soar.tracker.ui.components.NearbyAircraftList
import com.soar.tracker.ui.components.SensorDisplay
import com.soar.tracker.ui.theme.Green
import com.soar.tracker.ui.theme.Red

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TrackerScreen(onLogout: () -> Unit = {}, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val isTracking by TrackingService.isTracking.collectAsState()
    val latestResponse by TrackingService.latestResponse.collectAsState()
    val lastError by TrackingService.lastError.collectAsState()
    val lastUpdateTime by TrackingService.lastUpdateTime.collectAsState()
    val sensorData by TrackingService.lastSensorData.collectAsState()
    val groundPressureHpa by TrackingService.groundPressureHpa.collectAsState()

    var hasLocationPermission by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(
                context, Manifest.permission.ACCESS_FINE_LOCATION,
            ) == PackageManager.PERMISSION_GRANTED
                || ContextCompat.checkSelfPermission(
                context, Manifest.permission.ACCESS_COARSE_LOCATION,
            ) == PackageManager.PERMISSION_GRANTED,
        )
    }

    val permissionLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { permissions ->
        hasLocationPermission = permissions[Manifest.permission.ACCESS_FINE_LOCATION] == true
            || permissions[Manifest.permission.ACCESS_COARSE_LOCATION] == true
        if (hasLocationPermission) {
            TrackingService.start(context)
        }
    }

    Scaffold(
        modifier = modifier,
        topBar = {
            TopAppBar(
                title = { Text("SOAR Tracker") },
                actions = {
                    // Logout button
                    IconButton(onClick = {
                        if (isTracking) {
                            TrackingService.stop(context)
                        }
                        val app = context.applicationContext as SoarTrackerApp
                        app.authRepository.logout()
                        onLogout()
                    }) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ExitToApp,
                            contentDescription = "Sign out",
                        )
                    }
                    // Connection status indicator
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.padding(end = 16.dp),
                    ) {
                        Box(
                            modifier = Modifier
                                .size(10.dp)
                                .clip(CircleShape)
                                .background(if (isTracking && lastError == null) Green else Red),
                        )
                        if (lastUpdateTime > 0) {
                            val ago = (System.currentTimeMillis() - lastUpdateTime) / 1000
                            Text(
                                text = " ${ago}s ago",
                                style = MaterialTheme.typography.bodySmall,
                                modifier = Modifier.padding(start = 4.dp),
                            )
                        }
                    }
                },
            )
        },
        floatingActionButton = {
            FloatingActionButton(
                onClick = {
                    if (isTracking) {
                        TrackingService.stop(context)
                    } else if (hasLocationPermission) {
                        TrackingService.start(context)
                    } else {
                        val permissions = mutableListOf(
                            Manifest.permission.ACCESS_FINE_LOCATION,
                            Manifest.permission.ACCESS_COARSE_LOCATION,
                        )
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                            permissions.add(Manifest.permission.POST_NOTIFICATIONS)
                        }
                        permissionLauncher.launch(permissions.toTypedArray())
                    }
                },
            ) {
                Icon(
                    imageVector = if (isTracking) Icons.Default.Stop else Icons.Default.PlayArrow,
                    contentDescription = if (isTracking) "Stop tracking" else "Start tracking",
                )
            }
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Spacer(modifier = Modifier.height(4.dp))

            // Error display
            lastError?.let { error ->
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.errorContainer,
                    ),
                ) {
                    Text(
                        text = error,
                        modifier = Modifier.padding(12.dp),
                        color = MaterialTheme.colorScheme.onErrorContainer,
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
            }

            val response = latestResponse

            // Aircraft + Flight card
            FlightStatusCard(
                aircraft = response?.matchedAircraft,
                flight = response?.flight,
            )

            // Sensor card
            SensorDisplay(
                accelX = sensorData?.accelX,
                accelY = sensorData?.accelY,
                accelZ = sensorData?.accelZ,
                pressureHpa = sensorData?.pressureHpa,
                verticalSpeedMps = sensorData?.verticalSpeedMps,
                latitude = sensorData?.latitude,
                longitude = sensorData?.longitude,
                altitudeMeters = sensorData?.altitudeMeters,
                altitudeAglFeet = response?.altitudeAglFeet,
                groundPressureHpa = groundPressureHpa,
            )

            // Nearby aircraft
            NearbyAircraftList(
                aircraft = response?.nearbyAircraft ?: emptyList(),
            )

            // Open website button
            OutlinedButton(
                onClick = {
                    val intent = Intent(Intent.ACTION_VIEW, Uri.parse("https://glider.flights"))
                    context.startActivity(intent)
                },
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text("Open glider.flights")
            }

            Spacer(modifier = Modifier.height(80.dp)) // FAB clearance
        }
    }
}
