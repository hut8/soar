package com.soar.tracker.data.repository

import com.soar.tracker.data.api.LoginRequest
import com.soar.tracker.data.api.SoarApi
import com.soar.tracker.data.local.TokenManager

class AuthRepository(
    private val tokenManager: TokenManager,
) {
    suspend fun login(api: SoarApi, email: String, password: String): Result<String> {
        return try {
            val response = api.login(LoginRequest(email, password))
            if (response.isSuccessful) {
                val body = response.body() ?: return Result.failure(Exception("Empty response"))
                tokenManager.saveToken(body.token)
                val name = listOfNotNull(body.user.firstName, body.user.lastName)
                    .joinToString(" ")
                    .ifBlank { body.user.email ?: "User" }
                tokenManager.saveUserName(name)
                Result.success(name)
            } else {
                val errorBody = response.errorBody()?.string() ?: "Login failed"
                Result.failure(Exception(errorBody))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun logout() {
        tokenManager.clear()
    }

    fun isLoggedIn(): Boolean = tokenManager.isLoggedIn()

    fun getUserName(): String? = tokenManager.getUserName()
}
