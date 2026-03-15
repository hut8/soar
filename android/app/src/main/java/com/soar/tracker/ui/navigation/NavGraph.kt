package com.soar.tracker.ui.navigation

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.soar.tracker.SoarTrackerApp
import com.soar.tracker.ui.screens.LoginScreen
import com.soar.tracker.ui.screens.TrackerScreen

@Composable
fun NavGraph() {
    val context = LocalContext.current
    val app = context.applicationContext as SoarTrackerApp
    val navController = rememberNavController()

    val startDestination = if (app.authRepository.isLoggedIn()) "tracker" else "login"

    NavHost(navController = navController, startDestination = startDestination) {
        composable("login") {
            LoginScreen(
                onLoginSuccess = {
                    navController.navigate("tracker") {
                        popUpTo("login") { inclusive = true }
                    }
                },
            )
        }
        composable("tracker") {
            TrackerScreen()
        }
    }
}
