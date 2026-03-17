package com.soar.tracker.ui.util

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.geometry.Offset

/**
 * Aircraft icon drawing and altitude-based coloring.
 */

/**
 * Draw an aircraft icon centered at [center], rotated by [trackDegrees].
 * Uses a simple airplane silhouette: pointed nose, swept wings, tail fin.
 *
 * @param center Screen position to draw at
 * @param trackDegrees Aircraft track heading for rotation (0 = north/up)
 * @param size Icon size in pixels (total height nose-to-tail)
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
        val h = size / 2f // Half height
        val w = size * 0.35f // Half wingspan

        val path = Path().apply {
            // Nose (top)
            moveTo(0f, -h)
            // Right side of fuselage to wing root
            lineTo(w * 0.2f, -h * 0.2f)
            // Right wing tip
            lineTo(w, h * 0.1f)
            // Right wing trailing edge back to fuselage
            lineTo(w * 0.2f, h * 0.25f)
            // Right side of fuselage to tail
            lineTo(w * 0.2f, h * 0.65f)
            // Right tail fin
            lineTo(w * 0.55f, h)
            // Tail center
            lineTo(0f, h * 0.8f)
            // Left tail fin
            lineTo(-w * 0.55f, h)
            // Left side of fuselage from tail
            lineTo(-w * 0.2f, h * 0.65f)
            // Left wing trailing edge
            lineTo(-w * 0.2f, h * 0.25f)
            // Left wing tip
            lineTo(-w, h * 0.1f)
            // Left side of fuselage to nose
            lineTo(-w * 0.2f, -h * 0.2f)
            close()
        }

        drawPath(path, fillColor)
        drawPath(path, strokeColor, style = Stroke(width = 1.5f))
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
