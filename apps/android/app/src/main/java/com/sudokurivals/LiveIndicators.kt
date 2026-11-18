package com.sudokurivals

import android.provider.Settings
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
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

/**
 * A passing match message (server unreachable, an action that failed) for the overlay over the title. Each
 * distinct message shows once for six seconds or until dismissed: a retry loop failing the same way does not
 * flash it again, and it returns only after the message cleared and came back. Returns the text to show (or
 * null) and the dismiss action.
 */
@Composable
internal fun rememberLiveToast(message: String?): Pair<String?, () -> Unit> {
    var shown by remember { mutableStateOf<String?>(null) }
    // Keyed on the text: the same failure repeating keeps the key, so it neither restarts nor reappears.
    LaunchedEffect(message) {
        shown = message
        if (message != null) { delay(6_000); shown = null }
    }
    return shown to { shown = null }
}
