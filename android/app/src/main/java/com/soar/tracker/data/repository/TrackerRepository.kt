package com.soar.tracker.data.repository

import com.soar.tracker.data.api.SoarApi
import com.soar.tracker.data.api.TrackerFixRequest
import com.soar.tracker.data.api.TrackerFixResponse

class TrackerRepository(private val api: SoarApi) {
    suspend fun submitFix(request: TrackerFixRequest): Result<TrackerFixResponse> {
        return try {
            val response = api.submitTrackerFix(request)
            if (response.isSuccessful) {
                val body = response.body() ?: return Result.failure(Exception("Empty response"))
                Result.success(body)
            } else {
                val errorBody = response.errorBody()?.string() ?: "Request failed"
                Result.failure(Exception(errorBody))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
