package com.sudokurivals

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.text
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp

/**
 * Paper and ink, the same tokens as the web (`apps/web/styles.css`) and iOS (`Design.swift`): printed givens,
 * ballpoint-blue entries, pencil notes; amber is always the rival. Light/dark follow the system unless
 * Settings pins a theme.
 */
@Immutable
internal data class Palette(
    val canvas: Color, val background: Color, val foreground: Color, val muted: Color, val line: Color, val border: Color, val grid: Color,
    /** Text and icons in the brand hue (entries, links, selected outlines). */
    val accent: Color,
    /** Filled surfaces in the brand hue (primary buttons, the selected cell, the solo card). */
    val brand: Color, val onBrand: Color,
    val selected: Color, val related: Color, val same: Color,
    val error: Color, val errorBackground: Color, val ok: Color,
    /** Reconnecting: neither healthy nor failed. */
    val warn: Color,
    val rival: Color, val rivalSoft: Color, val track: Color,
    val dark: Boolean,
)

private val LightPalette = Palette(
    canvas = Color(0xfff4f1ea), background = Color(0xfffffdf9), foreground = Color(0xff1c1b18), muted = Color(0xff5d584e),
    line = Color(0xffddd6c8), border = Color(0xff8b8476), grid = Color(0xff2a2824), accent = Color(0xff1d4ed8),
    brand = Color(0xff1d4ed8), onBrand = Color.White, selected = Color(0xffdbe6ff), related = Color(0xffeff2fa), same = Color(0xffcddbff),
    error = Color(0xffb42318), errorBackground = Color(0xfffdecea), ok = Color(0xff15803d), warn = Color(0xffa15c07), rival = Color(0xffa8500a), rivalSoft = Color(0xfffbe6c4),
    track = Color(0xffe6e0d3), dark = false,
)
private val DarkPalette = Palette(
    canvas = Color(0xff121214), background = Color(0xff1c1c20), foreground = Color(0xfff4f2ec), muted = Color(0xffb6b2a8),
    line = Color(0xff34343b), border = Color(0xff8f8b7f), grid = Color(0xffa5a196), accent = Color(0xff93b4ff),
    brand = Color(0xff3b66f0), onBrand = Color.White, selected = Color(0xff27375f), related = Color(0xff22242d), same = Color(0xff2f4273),
    error = Color(0xffffa59c), errorBackground = Color(0xff3a1d1c), ok = Color(0xff4ade80), warn = Color(0xfffbbf24), rival = Color(0xfff5b453), rivalSoft = Color(0xff3b2c12),
    track = Color(0xff2e2e35), dark = true,
)

private val LocalPalette = staticCompositionLocalOf { LightPalette }

/** The current paper/ink tokens. */
internal val ink: Palette @Composable @ReadOnlyComposable get() = LocalPalette.current

@Composable
internal fun SudokuTheme(appearance: String, content: @Composable () -> Unit) {
    val dark = when (appearance) { "light" -> false; "dark" -> true; else -> isSystemInDarkTheme() }
    val p = if (dark) DarkPalette else LightPalette
    val base = if (dark) darkColorScheme() else lightColorScheme()
    // Material components (dialogs, text fields, switches) take the paper tokens too.
    val scheme = base.copy(
        primary = p.brand, onPrimary = p.onBrand, primaryContainer = p.selected, onPrimaryContainer = p.accent,
        secondary = p.accent, secondaryContainer = p.selected, onSecondaryContainer = p.foreground,
        background = p.canvas, onBackground = p.foreground, surface = p.background, onSurface = p.foreground,
        surfaceVariant = p.track, onSurfaceVariant = p.muted, surfaceContainerHigh = p.background, surfaceContainerHighest = p.track,
        surfaceContainer = p.background, surfaceContainerLow = p.background, outline = p.border, outlineVariant = p.line,
        error = p.error, errorContainer = p.errorBackground,
    )
    CompositionLocalProvider(LocalPalette provides p) {
        MaterialTheme(colorScheme = scheme) {
            CompositionLocalProvider(LocalContentColor provides p.foreground, content = content)
        }
    }
}

/** Type scale: tight display headings, the web's eyebrow and label styles. */
internal object Type {
    val hero = TextStyle(fontSize = 38.sp, lineHeight = 41.sp, fontWeight = FontWeight.Bold, letterSpacing = (-0.035).em)
    val title = TextStyle(fontSize = 30.sp, lineHeight = 33.sp, fontWeight = FontWeight.Bold, letterSpacing = (-0.03).em)
    val subtitle = TextStyle(fontSize = 19.sp, lineHeight = 24.sp, fontWeight = FontWeight.Bold, letterSpacing = (-0.02).em)
    val body = TextStyle(fontSize = 16.sp, lineHeight = 24.sp)
    val small = TextStyle(fontSize = 14.sp, lineHeight = 19.sp)
    val eyebrow = TextStyle(fontSize = 12.sp, lineHeight = 16.sp, fontWeight = FontWeight.Bold, letterSpacing = 0.12.em)
    val label = TextStyle(fontSize = 12.sp, lineHeight = 16.sp, fontWeight = FontWeight.SemiBold, letterSpacing = 0.1.em)
}

/** Stroke icons from the web's SVG paths (24×24, 1.7 stroke); tinted by `Icon`. */
internal object Glyph {
    private fun icon(name: String, vararg paths: String, width: Float = 1.7f) = ImageVector.Builder(name, 24.dp, 24.dp, 24f, 24f).apply {
        for (path in paths) addPath(addPathNodes(path), stroke = SolidColor(Color.Black), strokeLineWidth = width, strokeLineCap = StrokeCap.Round, strokeLineJoin = StrokeJoin.Round)
    }.build()
    val arrow = icon("arrow", "M5 12h14m-5-5 5 5-5 5")
    val back = icon("back", "m14 6-6 6 6 6")
    val pause = icon("pause", "M9 5v14M15 5v14")
    val notes = icon("notes", "m15 4 5 5M4 20l5-1L21 7a2 2 0 0 0-4-4L5 15l-1 5Z")
    val erase = icon("erase", "m4 12 7-8a2 2 0 0 1 3 0l6 6a2 2 0 0 1 0 3l-7 7H9l-5-5a2 2 0 0 1 0-3ZM9 7l9 9M13 20h8")
    val undo = icon("undo", "M4 10h10a6 6 0 0 1 0 12M4 10l5-5M4 10l5 5")
    val redo = icon("redo", "M20 10H10a6 6 0 0 0 0 12M20 10l-5-5M20 10l-5 5")
    val hint = icon("hint", "M9 18h6M10 21h4M8 13a6 6 0 1 1 8 0c-1 1-1 2-1 3H9c0-1 0-2-1-3Z")
    val check = icon("check", "m5 12 4 4L19 6", width = 2.2f)
    val warning = icon("warning", "m12 3 10 18H2L12 3ZM12 9v5M12 17v1")
    val play = icon("play", "m9 5 11 7-11 7V5Z")
    val trophy = icon("trophy", "M7 3h10v6a5 5 0 0 1-10 0V3ZM7 5H3v1a4 4 0 0 0 4 4M17 5h4v1a4 4 0 0 1-4 4M12 14v4M7.5 21h9M9.5 18h5", width = 2.2f)
    val flag = icon("flag", "M6 21V4m0 0h11l-2.5 4L17 12H6", width = 2.2f)
    val equal = icon("equal", "M6 9h12M6 15h12", width = 2.2f)
    val share = icon("share", "M12 15V4m0 0L8 8m4-4 4 4M5 13v5a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-5")
    val copy = icon("copy", "M10 9h9a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1ZM5 15V5a1 1 0 0 1 1-1h10")
    val close = icon("close", "M6 6l12 12M18 6 6 18")
    val settings = icon("settings", "M4 7h2M12 7h8M4 17h8M18 17h2M6 7a3 3 0 1 0 6 0a3 3 0 1 0-6 0M12 17a3 3 0 1 0 6 0a3 3 0 1 0-6 0", width = 1.8f)
}

/** The app mark, drawn from the same geometry as `apps/web/icon.svg` (192 units). */
@Composable
internal fun BrandMark(size: Dp = 34.dp) {
    Canvas(Modifier.size(size).clip(RoundedCornerShape(size * 9 / 34))) {
        val unit = this.size.width / 192f
        scale(unit, unit, Offset.Zero) {
            drawRect(Color(0xff1d4ed8), size = Size(192f, 192f))
            rotate(12f, Offset(96f, 96f)) {
                translate(11f, -13f) { drawRoundRect(Color(0xfff5b453), Offset(39f, 39f), Size(114f, 114f), androidx.compose.ui.geometry.CornerRadius(25f)) }
            }
            drawRoundRect(Color.White, Offset(39f, 39f), Size(114f, 114f), androidx.compose.ui.geometry.CornerRadius(25f))
            for (x in listOf(72.33f, 112.67f)) drawRect(Color(0xff1d4ed8), Offset(x, 39f), Size(7f, 114f))
            for (y in listOf(72.33f, 112.67f)) drawRect(Color(0xff1d4ed8), Offset(39f, y), Size(114f, 7f))
            drawRect(Color(0xfff5b453), Offset(79.33f, 79.33f), Size(33.33f, 33.33f))
        }
    }
}

/** Mode art: one player for solo; two for a friend match, you in blue and the rival in amber (40-unit art). */
@Composable
internal fun ModeArt(friends: Boolean, tint: Color, modifier: Modifier = Modifier) {
    val p = ink
    Canvas(modifier.size(40.dp)) {
        val unit = size.width / 40f
        fun path(d: String) = PathParser().parsePathString(d).toPath()
        scale(unit, unit, Offset.Zero) {
            if (!friends) {
                drawCircle(tint, 7f, Offset(20f, 13f))
                drawPath(path("M6 36c0-8.3 6.3-14 14-14s14 5.7 14 14Z"), tint)
            } else {
                drawCircle(p.rival, 5.5f, Offset(27.5f, 14f))
                drawPath(path("M17.5 34c0-6.5 4.2-10.5 10-10.5s10 4 10 10.5Z"), p.rival)
                val you = path("M1.5 35c0-7.6 5.2-12.5 12-12.5s12 4.9 12 12.5Z")
                drawCircle(p.accent, 6.5f, Offset(13.5f, 13f))
                drawCircle(p.canvas, 6.5f, Offset(13.5f, 13f), style = Stroke(2.5f))
                drawPath(you, p.accent)
                drawPath(you, p.canvas, style = Stroke(2.5f))
            }
        }
    }
}

/** A board-size picture: the outline with its box lines. */
@Composable
internal fun SizeArt(size: Int, tint: Color) {
    Canvas(Modifier.size(40.dp)) {
        val u = this.size.width / 36f
        val stroke = Stroke(2f * u)
        drawRoundRect(tint, Offset(1.5f * u, 1.5f * u), Size(33f * u, 33f * u), androidx.compose.ui.geometry.CornerRadius(6f * u), style = stroke)
        val vertical = if (size == 6) listOf(18f) else listOf(12.5f, 23.5f)
        for (x in vertical) drawLine(tint, Offset(x * u, 1.5f * u), Offset(x * u, 34.5f * u), 2f * u)
        for (y in listOf(12.5f, 23.5f)) drawLine(tint, Offset(1.5f * u, y * u), Offset(34.5f * u, y * u), 2f * u)
    }
}

/** A paper card: hairline edge and a soft shadow. */
internal fun Modifier.card(p: Palette, radius: Dp = 22.dp, fill: Color = p.background, stroke: Color = p.line, elevated: Boolean = true): Modifier {
    val shape = RoundedCornerShape(radius)
    return (if (elevated) shadow(2.dp, shape, ambientColor = Color.Black.copy(alpha = .25f), spotColor = Color.Black.copy(alpha = .2f)) else this)
        .clip(shape).background(fill).border(1.dp, stroke, shape)
}

internal val ButtonShape: Shape = RoundedCornerShape(16.dp)

@Composable
internal fun PrimaryButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, pending: String? = null) {
    val p = ink
    Button(
        onClick = onClick, enabled = enabled, shape = ButtonShape, modifier = modifier.heightIn(min = 52.dp),
        colors = ButtonDefaults.buttonColors(containerColor = p.brand, contentColor = p.onBrand, disabledContainerColor = p.brand.copy(alpha = .45f), disabledContentColor = p.onBrand.copy(alpha = .8f)),
        contentPadding = PaddingValues(horizontal = 20.dp, vertical = 12.dp),
    ) {
        // While the server has not answered, a spinner (named [pending] for TalkBack) takes the title's place at the same size.
        Box(contentAlignment = Alignment.Center) {
            Text(text, Modifier.alpha(if (pending != null) 0f else 1f).then(if (pending != null) Modifier.clearAndSetSemantics { } else Modifier), fontWeight = FontWeight.SemiBold, fontSize = 16.sp, textAlign = TextAlign.Center)
            if (pending != null) PendingIndicator(pending, size = 22.dp, color = p.onBrand)
        }
    }
}

@Composable
internal fun SecondaryButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, icon: ImageVector? = null) {
    val p = ink
    OutlinedButton(
        onClick = onClick, enabled = enabled, shape = ButtonShape, modifier = modifier.heightIn(min = 52.dp),
        border = BorderStroke(1.dp, p.line),
        colors = ButtonDefaults.outlinedButtonColors(containerColor = p.background, contentColor = p.accent, disabledContentColor = p.muted.copy(alpha = .6f)),
        contentPadding = PaddingValues(horizontal = 18.dp, vertical = 12.dp),
    ) {
        if (icon != null) { Icon(icon, null, Modifier.size(20.dp)); Spacer(Modifier.width(8.dp)) }
        Text(text, fontWeight = FontWeight.SemiBold, fontSize = 16.sp, textAlign = TextAlign.Center)
    }
}

@Composable
internal fun QuietButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    TextButton(onClick = onClick, modifier = modifier.heightIn(min = 48.dp), colors = ButtonDefaults.textButtonColors(contentColor = ink.muted)) { Text(text) }
}

/** A 48dp square icon button on paper (the game screens' back and pause). */
@Composable
internal fun SquareIconButton(icon: ImageVector, description: String, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val p = ink
    Box(
        modifier.size(48.dp).card(p, 14.dp, elevated = false)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = description },
        contentAlignment = Alignment.Center
    ) { Icon(icon, null, tint = p.foreground) }
}

/** Page headers: the small blue eyebrow above a display heading. */
@Composable
internal fun Eyebrow(text: String, color: Color = ink.accent) {
    // Shown in capitals (with the language's own rules: Turkish i → İ); read and exposed as written.
    Text(text.uppercase(LocalConfiguration.current.locales[0]), Modifier.clearAndSetSemantics { this.text = AnnotatedString(text) }, style = Type.eyebrow, color = color)
}

@Composable
internal fun PageTitle(text: String, modifier: Modifier = Modifier, style: TextStyle = Type.title) {
    Text(text, modifier.semantics { heading() }, style = style, color = ink.foreground)
}

@Composable
internal fun Lede(text: String) { Text(text, style = Type.body, color = ink.muted) }

@Composable
internal fun SectionLabel(text: String, modifier: Modifier = Modifier) {
    Text(text.uppercase(LocalConfiguration.current.locales[0]), modifier.clearAndSetSemantics { this.text = AnnotatedString(text); heading() }, style = Type.label, color = ink.muted)
}

/** A notice card: neutral, or red for errors. */
@Composable
internal fun Notice(text: String, modifier: Modifier = Modifier, error: Boolean = false, tag: String? = null, action: (@Composable () -> Unit)? = null) {
    val p = ink
    Column(modifier.fillMaxWidth().card(p, 14.dp, if (error) p.errorBackground else p.background, if (error) p.error else p.line, elevated = false).padding(horizontal = 14.dp, vertical = 10.dp)) {
        // The tag sits on the message itself, so tests and screen readers get its text; it is announced when it changes.
        Text(text, (if (tag != null) Modifier.testTag(tag) else Modifier).semantics { liveRegion = LiveRegionMode.Polite }, style = Type.small, color = if (error) p.error else p.foreground)
        action?.invoke()
    }
}

/** Back link at the top of a page: a 48dp target around the glyph, with its label. */
@Composable
internal fun BackLink(text: String, onClick: () -> Unit, tag: String = "back", description: String = text) {
    val p = ink
    TextButton(onClick = onClick, modifier = Modifier.heightIn(min = 48.dp).offset(x = (-12).dp).testTag(tag).semantics { this.contentDescription = description },
        colors = ButtonDefaults.textButtonColors(contentColor = p.muted), contentPadding = PaddingValues(start = 4.dp, end = 12.dp)) {
        Icon(Glyph.back, null, Modifier.size(24.dp)); Spacer(Modifier.width(4.dp)); Text(text, fontWeight = FontWeight.Medium)
    }
}

/**
 * A segmented choice on the track (web `.segmented`). Widths follow the words, but every option keeps at
 * least a 48dp touch width, so five levels fit one row on a 320dp phone.
 */
@Composable
internal fun Segmented(options: List<Pair<String, String>>, selected: String, onSelect: (String) -> Unit, modifier: Modifier = Modifier, tagPrefix: String? = null, enabled: Boolean = true, compact: Boolean = false) {
    val p = ink
    Row(modifier.clip(RoundedCornerShape(16.dp)).background(p.track).padding(4.dp).selectableGroup(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        for ((value, label) in options) {
            val on = value == selected
            Box(
                Modifier.weight(maxOf(label.length, 5) + 3f).heightIn(min = if (compact) 40.dp else 44.dp)
                    .then(if (on) Modifier.shadow(1.dp, RoundedCornerShape(12.dp)).clip(RoundedCornerShape(12.dp)).background(p.background).border(1.5.dp, p.accent, RoundedCornerShape(12.dp)) else Modifier.clip(RoundedCornerShape(12.dp)))
                    .selectable(on, enabled = enabled, role = Role.RadioButton) { onSelect(value) }
                    .let { if (tagPrefix != null) it.testTag("$tagPrefix$value") else it }
                    .padding(horizontal = 4.dp, vertical = 6.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(label, color = if (!enabled) p.muted.copy(alpha = .5f) else if (on) p.foreground else p.muted, fontWeight = FontWeight.SemiBold,
                    fontSize = if (options.size > 3) 13.sp else 15.sp, lineHeight = 16.sp, textAlign = TextAlign.Center, maxLines = 2)
            }
        }
    }
}

/** A thin progress meter (filled share, rival share or time left). */
@Composable
internal fun Meter(fraction: Float, color: Color, modifier: Modifier = Modifier, fromEnd: Boolean = false) {
    val p = ink
    Box(modifier.fillMaxWidth().height(6.dp).clip(RoundedCornerShape(6.dp)).background(p.track)) {
        Box(Modifier.fillMaxHeight().fillMaxWidth(fraction.coerceIn(0f, 1f)).align(if (fromEnd) Alignment.CenterEnd else Alignment.CenterStart).clip(RoundedCornerShape(6.dp)).background(color))
    }
}
