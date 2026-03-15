package com.soar.tracker.ui.navigation

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.LocationOn
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.soar.tracker.SoarTrackerApp
import com.soar.tracker.ui.screens.LiveViewScreen
import com.soar.tracker.ui.screens.LoginScreen
import com.soar.tracker.ui.screens.TrackerScreen

@Composable
fun NavGraph() {
    val context = LocalContext.current
    val app = context.applicationContext as SoarTrackerApp
    val navController = rememberNavController()

    val startDestination = if (app.authRepository.isLoggedIn()) "main" else "login"

    NavHost(navController = navController, startDestination = startDestination) {
        composable("login") {
            LoginScreen(
                onLoginSuccess = {
                    navController.navigate("main") {
                        popUpTo("login") { inclusive = true }
                    }
                },
            )
        }
        composable("main") {
            MainScreen(
                onLogout = {
                    navController.navigate("login") {
                        popUpTo("main") { inclusive = true }
                    }
                },
            )
        }
    }
}

@Composable
private fun MainScreen(onLogout: () -> Unit) {
    var selectedTab by rememberSaveable { mutableIntStateOf(0) }

    Scaffold(
        bottomBar = {
            NavigationBar {
                NavigationBarItem(
                    selected = selectedTab == 0,
                    onClick = { selectedTab = 0 },
                    icon = { Icon(Icons.Default.PlayArrow, contentDescription = null) },
                    label = { Text("Tracker") },
                )
                NavigationBarItem(
                    selected = selectedTab == 1,
                    onClick = { selectedTab = 1 },
                    icon = { Icon(Icons.Default.LocationOn, contentDescription = null) },
                    label = { Text("Live View") },
                )
            }
        },
    ) { padding ->
        when (selectedTab) {
            0 -> TrackerScreen(
                onLogout = onLogout,
                modifier = Modifier.padding(bottom = padding.calculateBottomPadding()),
            )
            1 -> LiveViewScreen(
                modifier = Modifier.padding(bottom = padding.calculateBottomPadding()),
            )
        }
    }
}
