# Moshi
-keep class com.soar.tracker.data.api.** { *; }
-keepclassmembers class com.soar.tracker.data.api.** { *; }

# Retrofit
-keepattributes Signature
-keepattributes *Annotation*
-keep class retrofit2.** { *; }
-keepclasseswithmembers class * {
    @retrofit2.http.* <methods>;
}
