package com.soar.tracker.data.api

import retrofit2.Response
import retrofit2.http.Body
import retrofit2.http.POST

interface SoarApi {
    @POST("data/auth/login")
    suspend fun login(@Body request: LoginRequest): Response<LoginResponse>

    @POST("data/auth/password-reset/request")
    suspend fun requestPasswordReset(@Body request: PasswordResetRequest): Response<Unit>

    @POST("data/tracker")
    suspend fun submitTrackerFix(@Body request: TrackerFixRequest): Response<TrackerFixResponse>
}
