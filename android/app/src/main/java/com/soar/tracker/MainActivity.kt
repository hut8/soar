package com.soar.tracker

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.soar.tracker.ui.navigation.NavGraph
import com.soar.tracker.ui.theme.SoarTrackerTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            SoarTrackerTheme {
                NavGraph()
            }
        }
    }
}
