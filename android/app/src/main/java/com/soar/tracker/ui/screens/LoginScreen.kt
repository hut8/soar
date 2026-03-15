package com.soar.tracker.ui.screens

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import com.soar.tracker.SoarTrackerApp
import com.soar.tracker.ui.components.RadarSweep
import kotlinx.coroutines.launch

private const val PRODUCTION_URL = "https://glider.flights"
private const val STAGING_URL = "https://staging.glider.flights"

@Composable
fun LoginScreen(onLoginSuccess: () -> Unit) {
    val context = LocalContext.current
    val app = context.applicationContext as SoarTrackerApp
    val scope = rememberCoroutineScope()

    var useStaging by remember {
        mutableStateOf(app.tokenManager.getServerUrl() == STAGING_URL)
    }
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var resetSent by remember { mutableStateOf(false) }

    val serverUrl = if (useStaging) STAGING_URL else PRODUCTION_URL

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(32.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = "SOAR Tracker",
            style = MaterialTheme.typography.headlineLarge,
        )

        Spacer(modifier = Modifier.height(8.dp))

        Text(
            text = "Sign in to start tracking",
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        Spacer(modifier = Modifier.height(32.dp))

        OutlinedTextField(
            value = email,
            onValueChange = { email = it },
            label = { Text("Email") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            keyboardOptions = KeyboardOptions(
                keyboardType = KeyboardType.Email,
                imeAction = ImeAction.Next,
            ),
        )

        Spacer(modifier = Modifier.height(16.dp))

        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("Password") },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            keyboardOptions = KeyboardOptions(
                keyboardType = KeyboardType.Password,
                imeAction = ImeAction.Done,
            ),
        )

        Spacer(modifier = Modifier.height(4.dp))

        // Forgot password link
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.End,
        ) {
            TextButton(
                onClick = {
                    if (email.isNotBlank()) {
                        resetSent = false
                        error = null
                        scope.launch {
                            val loginApi = app.createLoginApi(serverUrl)
                            try {
                                val response = loginApi.requestPasswordReset(
                                    com.soar.tracker.data.api.PasswordResetRequest(email),
                                )
                                if (response.isSuccessful) {
                                    resetSent = true
                                } else {
                                    error = response.errorBody()?.string()
                                        ?: "Could not send reset email. Please verify the email and try again."
                                }
                            } catch (e: Exception) {
                                error = "Could not send reset email. Please check your connection."
                            }
                        }
                    } else {
                        // Open the password reset page in browser as fallback
                        val intent = Intent(
                            Intent.ACTION_VIEW,
                            Uri.parse("$serverUrl/reset-password"),
                        )
                        context.startActivity(intent)
                    }
                },
            ) {
                Text(
                    text = "Forgot password?",
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }

        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Checkbox(
                checked = useStaging,
                onCheckedChange = { useStaging = it },
            )
            Text(
                text = "Staging (development only)",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        if (resetSent) {
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = "Password reset email sent. Check your inbox.",
                color = MaterialTheme.colorScheme.primary,
                style = MaterialTheme.typography.bodySmall,
            )
        }

        if (error != null) {
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = error!!,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodySmall,
            )
        }

        Spacer(modifier = Modifier.height(24.dp))

        Button(
            onClick = {
                isLoading = true
                error = null
                resetSent = false
                scope.launch {
                    val loginApi = app.createLoginApi(serverUrl)
                    val result = app.authRepository.login(loginApi, email, password)
                    result.onSuccess {
                        app.tokenManager.saveServerUrl(serverUrl)
                        app.initializeApiClient(serverUrl)
                        isLoading = false
                        onLoginSuccess()
                    }.onFailure { e ->
                        error = e.message ?: "Login failed"
                        isLoading = false
                    }
                }
            },
            modifier = Modifier.fillMaxWidth(),
            enabled = !isLoading && email.isNotBlank() && password.isNotBlank(),
        ) {
            if (isLoading) {
                RadarSweep(size = 18.dp)
            } else {
                Text("Sign In")
            }
        }
    }
}
