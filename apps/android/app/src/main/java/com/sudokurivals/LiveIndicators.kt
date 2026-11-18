package com.sudokurivals

import android.provider.Settings
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalAccessibilityManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay

/** Connection states as the core reports them, folded to what the dot shows. */
internal fun connectionKind(connection: String) = when (connection) {
    "connected", "reconnecting", "session_lost" -> connection
    else -> "disconnected" // "disconnected" and "reconcile"
}

/**
 * The connection dot, the same on every screen. Shape and colour both carry the state, so it never relies on
 * colour alone: filled green (connected), half-filled amber (reconnecting), hollow red (disconnected or boards
 * differ), struck-through red (session lost). The caller supplies the words (visible or for TalkBack).
 */
@Composable
internal fun ConnectionDot(connection: String, modifier: Modifier = Modifier, size: Dp = 9.dp) {
    val p = ink
    val kind = connectionKind(connection)
    val color = when (kind) { "connected" -> p.ok; "reconnecting" -> p.warn; else -> p.error }
    Canvas(modifier.size(size)) {
        val stroke = 1.5.dp.toPx()
        val r = this.size.minDimension / 2
        when (kind) {
            "connected" -> drawCircle(color, r)
            "reconnecting" -> {
                drawArc(color, 90f, 180f, useCenter = true, size = Size(r * 2, r * 2))
                drawCircle(color, r - stroke / 2, style = Stroke(stroke))
            }
            else -> {
                drawCircle(color, r - stroke / 2, style = Stroke(stroke))
                if (kind == "session_lost") drawLine(color, Offset(r * .45f, r * .45f), Offset(r * 1.55f, r * 1.55f), stroke, StrokeCap.Round)
            }
        }
    }
}

/** True when the system "Remove animations" setting (animator duration scale 0) is on. */
@Composable
internal fun animationsOff(): Boolean {
    val resolver = LocalContext.current.contentResolver
    return remember { Settings.Global.getFloat(resolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f }
}

/**
 * Waiting for the server: a small turning arc in place of the old sentence, in the same footprint, so nothing
 * moves. With "Remove animations" on it stands still (a static arc). TalkBack reads [description].
 */
@Composable
internal fun PendingIndicator(description: String, modifier: Modifier = Modifier, size: Dp = 18.dp, color: Color = ink.foreground, tag: String = "pending-indicator") {
    val still = animationsOff()
    val angle = if (still) 0f else {
        val turn by rememberInfiniteTransition().animateFloat(
            0f, 360f, infiniteRepeatable(tween(900, easing = LinearEasing), RepeatMode.Restart),
        )
        turn
    }
    Canvas(modifier.size(size).testTag(tag).semantics { contentDescription = description }.graphicsLayer { rotationZ = angle }) {
        val stroke = size.toPx() / 7.5f
        val inset = stroke / 2
        val arc = Size(this.size.width - stroke, this.size.height - stroke)
        drawCircle(color.copy(alpha = .25f), this.size.minDimension / 2 - inset, style = Stroke(stroke))
        drawArc(color, -90f, 90f, useCenter = false, topLeft = Offset(inset, inset), size = arc, style = Stroke(stroke, cap = StrokeCap.Round))
    }
}

@Composable
internal fun TransientNotice(text: String, onDismiss: () -> Unit, tag: String, modifier: Modifier = Modifier) {
    val p = ink
    Row(
        modifier.fillMaxWidth().heightIn(min = 48.dp).clip(RoundedCornerShape(14.dp)).background(p.foreground)
            .pointerInput(Unit) { detectTapGestures { } },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(text, Modifier.weight(1f).padding(start = 12.dp, top = 6.dp, bottom = 6.dp).testTag(tag)
            .semantics { liveRegion = LiveRegionMode.Polite }, color = p.canvas, fontSize = 13.sp, lineHeight = 17.sp)
        val close = res(R.string.action_dismiss)
        Box(Modifier.size(48.dp).clickable(role = Role.Button, onClick = onDismiss)
            .semantics { contentDescription = close }, contentAlignment = Alignment.Center) {
            Icon(Glyph.close, null, Modifier.size(20.dp), tint = p.canvas)
        }
    }
}

/** Repeated failures with identical text during automatic retries keep the same timeout. */
@Composable
internal fun rememberLiveToast(message: String?): Pair<String?, () -> Unit> {
    var shown by remember(message) { mutableStateOf(message != null) }
    val accessibility = LocalAccessibilityManager.current
    LaunchedEffect(message) {
        if (message != null) {
            delay(accessibility?.calculateRecommendedTimeoutMillis(6_000, containsText = true, containsControls = true) ?: 6_000)
            shown = false
        }
    }
    return message.takeIf { shown } to { shown = false }
}
