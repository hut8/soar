package com.soar.tracker.ui.util

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.geometry.Offset

/**
 * Aircraft icon paths and altitude-based coloring, ported from the web's aircraftIcons.ts.
 *
 * SVG paths from tar1090 project (https://github.com/wiedehopf/tar1090).
 */

/**
 * Draw an aircraft icon centered at [center], rotated by [trackDegrees].
 * Uses the "unknown" aircraft silhouette from tar1090 — a generic airplane shape
 * that works well at small sizes on mobile.
 *
 * @param center Screen position to draw at
 * @param trackDegrees Aircraft track heading for rotation
 * @param size Icon size in pixels (height)
 * @param fillColor Fill color (typically altitude-based)
 * @param strokeColor Outline color
 */
fun DrawScope.drawAircraftIcon(
    center: Offset,
    trackDegrees: Float,
    size: Float,
    fillColor: Color,
    strokeColor: Color = Color.White,
) {
    withTransform({
        translate(center.x, center.y)
        rotate(trackDegrees)
    }) {
        // "unknown" icon from tar1090: viewBox "-2.5 -2.5 22 22" → 22x22 units, centered at ~8.5,8
        // Scale from 22-unit viewBox to desired pixel size
        val scale = size / 22f
        val cx = 8.5f // Approximate center X of the path
        val cy = 8.5f // Approximate center Y of the path

        val path = Path().apply {
            // Simplified "unknown" airplane silhouette
            // Body
            moveTo((4.256f - cx) * scale, (15.496f - cy) * scale)
            // Left wing sweep
            cubicTo(
                (3.979f - cx) * scale, (14.340f - cy) * scale,
                (7.280f - cx) * scale, (13.606f - cy) * scale,
                (7.280f - cx) * scale, (13.606f - cy) * scale,
            )
            lineTo((7.280f - cx) * scale, (8.650f - cy) * scale)
            lineTo((1.280f - cx) * scale, (10.650f - cy) * scale)
            cubicTo(
                (0.600f - cx) * scale, (10.650f - cy) * scale,
                (0.280f - cx) * scale, (10.300f - cy) * scale,
                (0.280f - cx) * scale, (9.990f - cy) * scale,
            )
            cubicTo(
                (0.242f - cx) * scale, (9.595f - cy) * scale,
                (0.496f - cx) * scale, (9.231f - cy) * scale,
                (0.880f - cx) * scale, (9.130f - cy) * scale,
            )
            cubicTo(
                (1.140f - cx) * scale, (9.0f - cy) * scale,
                (4.800f - cx) * scale, (7.0f - cy) * scale,
                (7.280f - cx) * scale, (5.630f - cy) * scale,
            )
            lineTo((7.280f - cx) * scale, (3.0f - cy) * scale)
            cubicTo(
                (7.280f - cx) * scale, (1.890f - cy) * scale,
                (7.720f - cx) * scale, (0.290f - cy) * scale,
                (8.510f - cx) * scale, (0.290f - cy) * scale,
            )
            cubicTo(
                (9.300f - cx) * scale, (0.290f - cy) * scale,
                (9.770f - cx) * scale, (1.840f - cy) * scale,
                (9.770f - cx) * scale, (3.0f - cy) * scale,
            )
            lineTo((9.770f - cx) * scale, (5.630f - cy) * scale)
            cubicTo(
                (12.220f - cx) * scale, (7.0f - cy) * scale,
                (15.870f - cx) * scale, (9.0f - cy) * scale,
                (16.140f - cx) * scale, (9.130f - cy) * scale,
            )
            cubicTo(
                (16.530f - cx) * scale, (9.223f - cy) * scale,
                (16.791f - cx) * scale, (9.591f - cy) * scale,
                (16.750f - cx) * scale, (9.990f - cy) * scale,
            )
            cubicTo(
                (16.700f - cx) * scale, (10.300f - cy) * scale,
                (16.390f - cx) * scale, (10.660f - cy) * scale,
                (15.700f - cx) * scale, (10.660f - cy) * scale,
            )
            lineTo((9.770f - cx) * scale, (8.660f - cy) * scale)
            lineTo((9.770f - cx) * scale, (13.606f - cy) * scale)
            cubicTo(
                (9.770f - cx) * scale, (13.606f - cy) * scale,
                (13.070f - cx) * scale, (14.340f - cy) * scale,
                (12.794f - cx) * scale, (15.496f - cy) * scale,
            )
            cubicTo(
                (12.463f - cx) * scale, (16.880f - cy) * scale,
                (9.964f - cx) * scale, (15.874f - cy) * scale,
                (8.540f - cx) * scale, (15.874f - cy) * scale,
            )
            cubicTo(
                (7.106f - cx) * scale, (15.874f - cy) * scale,
                (4.590f - cx) * scale, (16.890f - cy) * scale,
                (4.256f - cx) * scale, (15.496f - cy) * scale,
            )
            close()
        }

        drawPath(path, fillColor)
        drawPath(path, strokeColor, style = Stroke(width = 1.2f))
    }
}

/** Altitude gradient stop: altitude in feet to RGB color components. */
private data class AltColorStop(val alt: Double, val r: Int, val g: Int, val b: Int)

private val ALTITUDE_COLOR_STOPS = listOf(
    AltColorStop(0.0, 239, 68, 68),       // Red    #ef4444
    AltColorStop(5000.0, 249, 115, 22),    // Orange #f97316
    AltColorStop(10000.0, 234, 179, 8),    // Yellow #eab308
    AltColorStop(20000.0, 34, 197, 94),    // Green  #22c55e
    AltColorStop(30000.0, 6, 182, 212),    // Cyan   #06b6d4
    AltColorStop(40000.0, 56, 189, 248),   // Blue   #38bdf8
)

/**
 * Altitude-based color gradient matching the web view.
 * Interpolates from red (ground) through orange, yellow, green, cyan to light blue (40,000 ft).
 *
 * @param altitudeFeet Altitude in feet MSL, or null for unknown
 * @return Color for the aircraft marker
 */
fun getAltitudeColor(altitudeFeet: Double?): Color {
    if (altitudeFeet == null) return Color(0xFF6B7280) // Gray for unknown

    val alt = altitudeFeet.coerceIn(0.0, 40000.0)

    // Find bracketing stops
    val stops = ALTITUDE_COLOR_STOPS
    var lower = stops.first()
    var upper = stops.last()
    for (i in 0 until stops.size - 1) {
        if (alt >= stops[i].alt && alt < stops[i + 1].alt) {
            lower = stops[i]
            upper = stops[i + 1]
            break
        }
    }

    val range = upper.alt - lower.alt
    val t = if (range > 0) ((alt - lower.alt) / range).toFloat() else 0f

    val r = (lower.r + (upper.r - lower.r) * t).toInt()
    val g = (lower.g + (upper.g - lower.g) * t).toInt()
    val b = (lower.b + (upper.b - lower.b) * t).toInt()

    return Color(r, g, b)
}
