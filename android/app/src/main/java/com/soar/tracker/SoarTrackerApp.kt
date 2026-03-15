package com.soar.tracker

import android.app.Application
import com.soar.tracker.data.api.AuthInterceptor
import com.soar.tracker.data.api.SoarApi
import com.soar.tracker.data.local.TokenManager
import com.soar.tracker.data.repository.AuthRepository
import com.soar.tracker.data.repository.TrackerRepository
import com.squareup.moshi.Moshi
import com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Retrofit
import retrofit2.converter.moshi.MoshiConverterFactory
import java.util.concurrent.TimeUnit

class SoarTrackerApp : Application() {
    lateinit var tokenManager: TokenManager
        private set
    lateinit var authRepository: AuthRepository
        private set
    var trackerRepository: TrackerRepository? = null
        private set
    var api: SoarApi? = null
        private set

    override fun onCreate() {
        super.onCreate()
        tokenManager = TokenManager(this)
        authRepository = AuthRepository(tokenManager)

        // If already logged in, initialize API client
        if (tokenManager.isLoggedIn()) {
            initializeApiClient(tokenManager.getServerUrl())
        }
    }

    fun initializeApiClient(baseUrl: String) {
        val moshi = Moshi.Builder()
            .addLast(KotlinJsonAdapterFactory())
            .build()

        val loggingInterceptor = HttpLoggingInterceptor().apply {
            level = HttpLoggingInterceptor.Level.BODY
        }

        val client = OkHttpClient.Builder()
            .addInterceptor(AuthInterceptor(tokenManager))
            .addInterceptor(loggingInterceptor)
            .connectTimeout(10, TimeUnit.SECONDS)
            .readTimeout(10, TimeUnit.SECONDS)
            .writeTimeout(10, TimeUnit.SECONDS)
            .build()

        val normalizedUrl = if (baseUrl.endsWith("/")) baseUrl else "$baseUrl/"

        val retrofit = Retrofit.Builder()
            .baseUrl(normalizedUrl)
            .client(client)
            .addConverterFactory(MoshiConverterFactory.create(moshi))
            .build()

        api = retrofit.create(SoarApi::class.java)
        trackerRepository = TrackerRepository(api!!)
    }

    /** Create a temporary API client for login (before token is stored). */
    fun createLoginApi(baseUrl: String): SoarApi {
        val moshi = Moshi.Builder()
            .addLast(KotlinJsonAdapterFactory())
            .build()

        val normalizedUrl = if (baseUrl.endsWith("/")) baseUrl else "$baseUrl/"

        val client = OkHttpClient.Builder()
            .connectTimeout(10, TimeUnit.SECONDS)
            .readTimeout(10, TimeUnit.SECONDS)
            .build()

        val retrofit = Retrofit.Builder()
            .baseUrl(normalizedUrl)
            .client(client)
            .addConverterFactory(MoshiConverterFactory.create(moshi))
            .build()

        return retrofit.create(SoarApi::class.java)
    }
}
