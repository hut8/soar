package com.soar.tracker.ui.components

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlin.math.cos
import kotlin.math.sin

/**
 * Radar sweep animation matching the frontend's RadarLoader.svelte.
 * Draws three concentric rings, a center dot, and a rotating sweep line.
 */
@Composable
fun RadarSweep(
    modifier: Modifier = Modifier,
    size: Dp = 20.dp,
) {
    val color = MaterialTheme.colorScheme.primary

    val transition = rememberInfiniteTransition(label = "radar")
    val angle by transition.animateFloat(
        initialValue = 0f,
        targetValue = 360f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 2000, easing = LinearEasing),
            repeatMode = RepeatMode.Restart,
        ),
        label = "sweep",
    )

    Canvas(modifier = modifier.size(size)) {
        val cx = this.size.width / 2f
        val cy = this.size.height / 2f
        val unit = this.size.minDimension / 24f // match SVG viewBox 0..24

        val ringColor = color.copy(alpha = 0.3f)
        val ringStroke = Stroke(width = unit * 0.8f)

        // Three concentric rings (radii 10, 7, 4 in SVG units)
        drawCircle(color = ringColor, radius = unit * 10f, center = Offset(cx, cy), style = ringStroke)
        drawCircle(color = ringColor, radius = unit * 7f, center = Offset(cx, cy), style = ringStroke)
        drawCircle(color = ringColor, radius = unit * 4f, center = Offset(cx, cy), style = ringStroke)

        // Center dot (radius 1.5 in SVG units)
        drawCircle(color = color, radius = unit * 1.5f, center = Offset(cx, cy))

        // Sweep line (from center to top, rotated)
        val lineLength = unit * 10f
        val radians = Math.toRadians((angle - 90).toDouble()) // -90 so 0° points up
        val endX = cx + lineLength * cos(radians).toFloat()
        val endY = cy + lineLength * sin(radians).toFloat()
        drawLine(
            color = color.copy(alpha = 0.9f),
            start = Offset(cx, cy),
            end = Offset(endX, endY),
            strokeWidth = unit * 1.5f,
            cap = StrokeCap.Round,
        )
    }
}
