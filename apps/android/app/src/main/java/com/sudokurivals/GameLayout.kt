package com.sudokurivals

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * Developer-only extra text on the game screens (app title, selected-cell line, save note,
 * "game in progress"). Off by default; only a debug build honours `--ez debug_ui true`, so
 * staging and release can never switch it on. Not persisted.
 */
internal object UiDebug {
    var enabled by mutableStateOf(false)
}

private val MIN_BOARD = 280.dp
private val MAX_BOARD = 560.dp
private val GAP = 8.dp
private val SIDE = 12.dp
private val PANEL = 300.dp

/**
 * One-screen game layout shared by solo, race and duel: [top] (navigation + status), the
 * board (the largest square the remaining height allows), then [bottom] (digits, tools,
 * transient notices). Wide landscape puts top+bottom in a side panel. When even a
 * [MIN_BOARD] board cannot fit, the page scrolls instead of clipping a control.
 */
@Composable
internal fun GameScreenLayout(top: @Composable () -> Unit, bottom: @Composable () -> Unit, board: @Composable (Dp) -> Unit) {
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val viewportW = constraints.maxWidth
        val viewportH = constraints.maxHeight
        Box(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
            SubcomposeLayout(Modifier.fillMaxWidth()) { _ ->
                val gap = GAP.roundToPx()
                val side = SIDE.roundToPx()
                val vPad = GAP.roundToPx()
                val minBoard = MIN_BOARD.roundToPx()
                val maxBoard = MAX_BOARD.roundToPx()
                val landscape = viewportW >= 560.dp.roundToPx() && viewportW > viewportH * 6 / 5
                if (landscape) {
                    val panelW = PANEL.roundToPx()
                    val boardSide = minOf(viewportH - 2 * vPad, viewportW - 2 * side - panelW - gap, maxBoard).coerceAtLeast(minBoard)
                    val panel = subcompose("panel") { Column(verticalArrangement = Arrangement.spacedBy(GAP)) { top(); bottom() } }
                        .map { it.measure(Constraints(maxWidth = panelW)) }
                    val boardP = subcompose("board") { board(boardSide.toDp()) }.map { it.measure(Constraints.fixed(boardSide, boardSide)) }
                    val panelH = panel.maxOfOrNull { it.height } ?: 0
                    val height = maxOf(viewportH, boardSide + 2 * vPad, panelH + 2 * vPad)
                    layout(viewportW, height) {
                        val used = boardSide + gap + panelW
                        val left = ((viewportW - used) / 2).coerceAtLeast(side)
                        boardP.forEach { it.placeRelative(left, (height - boardSide) / 2) }
                        panel.forEach { it.placeRelative(left + boardSide + gap, ((height - panelH) / 2).coerceAtLeast(vPad)) }
                    }
                } else {
                    val w = viewportW - 2 * side
                    val topP = subcompose("top", top).map { it.measure(Constraints(maxWidth = w)) }
                    val bottomP = subcompose("bottom", bottom).map { it.measure(Constraints(maxWidth = w)) }
                    val topH = topP.sumOf { it.height }
                    val bottomH = bottomP.sumOf { it.height }
                    val free = viewportH - 2 * vPad - topH - bottomH - 2 * gap
                    val fits = free >= minBoard
                    val boardSide = minOf(w, maxBoard, if (fits) free else minBoard)
                    val boardP = subcompose("board") { board(boardSide.toDp()) }.map { it.measure(Constraints.fixed(boardSide, boardSide)) }
                    val content = vPad + topH + gap + boardSide + gap + bottomH + vPad
                    val height = if (fits) viewportH else content
                    layout(viewportW, height) {
                        // Navigation stays at the top. Spare height (a board capped by width) is
                        // split above the board and below the controls, keeping controls beside it.
                        val spare = if (fits) (height - content).coerceAtLeast(0) else 0
                        var y = vPad
                        topP.forEach { it.placeRelative(side, y); y += it.height }
                        y += gap + spare / 2
                        boardP.forEach { it.placeRelative(side + (w - boardSide) / 2, y) }
                        y += boardSide + gap
                        bottomP.forEach { it.placeRelative(side, y); y += it.height }
                    }
                }
            }
        }
    }
}

/** Navigation row: back · `Mode · N×N` + time · right slot. */
@Composable
internal fun GameNavBar(modeLabel: String, timeText: String, timeDescription: String, backDescription: String, onBack: () -> Unit, trailing: @Composable () -> Unit) {
    Row(Modifier.fillMaxWidth().heightIn(min = 48.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.widthIn(min = 72.dp), contentAlignment = Alignment.CenterStart) {
            TextButton(onClick = onBack, modifier = Modifier.width(48.dp).heightIn(min = 48.dp).testTag("back").semantics { contentDescription = backDescription }, contentPadding = PaddingValues(4.dp)) {
                Text("‹", Modifier.clearAndSetSemantics { }, fontSize = 24.sp)
            }
        }
        Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
            Text(modeLabel, Modifier.testTag("mode-label").semantics { heading() }, style = MaterialTheme.typography.titleMedium, textAlign = TextAlign.Center)
            Text(timeText, Modifier.testTag("timer").semantics { contentDescription = timeDescription }, style = MaterialTheme.typography.bodyMedium)
        }
        Box(Modifier.widthIn(min = 72.dp), contentAlignment = Alignment.CenterEnd) { trailing() }
    }
}

/** Small filled dot for a healthy connection; the dot carries the state for screen readers. */
@Composable
internal fun StatusDot(color: Color, description: String, tag: String) {
    Box(Modifier.size(24.dp).testTag(tag).semantics { contentDescription = description; liveRegion = LiveRegionMode.Polite }, contentAlignment = Alignment.Center) {
        Box(Modifier.size(10.dp).background(color, androidx.compose.foundation.shape.CircleShape))
    }
}

/** Digit pad: one row on 6×6 (3×2 when narrower than six 48dp targets), 5+4 centred on 9×9. */
@Composable
internal fun DigitPad(n: Int, enabled: Boolean, onDigit: (Int) -> Unit) {
    BoxWithConstraints(Modifier.fillMaxWidth()) {
        val gap = 6.dp
        val columns = if (n == 6) (if (maxWidth >= 48.dp * 6 + gap * 5) 6 else 3) else 5
        Column(verticalArrangement = Arrangement.spacedBy(gap)) {
            for (digits in (1..n).chunked(columns)) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(gap)) {
                val spare = columns - digits.size
                if (spare > 0) Spacer(Modifier.weight(spare / 2f))
                for (digit in digits) OutlinedButton(onClick = { onDigit(digit) }, enabled = enabled, modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("number-$digit"), contentPadding = PaddingValues(4.dp)) { Text(digit.toString()) }
                if (spare > 0) Spacer(Modifier.weight(spare / 2f))
            }
        }
    }
}

/** Shared board: tag, grid semantics and per-cell labels are identical for solo and live. */
@Composable
internal fun SudokuBoard(
    n: Int, side: Dp, selected: Int, conflicts: Collection<Int>,
    value: (Int) -> Int, given: (Int) -> Boolean, notes: (Int) -> List<Int>, label: (Int) -> String, onSelect: (Int) -> Unit
) {
    val fontScale = LocalDensity.current.fontScale
    val boxRows = if (n == 6) 2 else 3
    val symbolSize = (minOf(24f, side.value / n * 0.55f) / fontScale).sp
    val noteSize = (maxOf(7f, side.value / n * 0.27f) / fontScale).sp
    Column(Modifier.size(side).testTag("game-board").semantics { collectionInfo = CollectionInfo(n, n) }) {
        for (row in 0 until n) Row(Modifier.weight(1f)) {
            for (column in 0 until n) {
                val index = row * n + column
                val v = value(index)
                val noteList = notes(index)
                val isGiven = given(index)
                val isSelected = selected == index
                val conflict = index in conflicts
                val line = MaterialTheme.colorScheme.onSurface
                Box(
                    Modifier.weight(1f).fillMaxHeight()
                        .background(if (isSelected) MaterialTheme.colorScheme.primaryContainer else Color.Transparent)
                        .drawBehind {
                            drawLine(line, Offset.Zero, Offset(size.width, 0f), if (row % boxRows == 0) 2.dp.toPx() else 0.5.dp.toPx())
                            drawLine(line, Offset.Zero, Offset(0f, size.height), if (column % 3 == 0) 2.dp.toPx() else 0.5.dp.toPx())
                            if (row == n - 1) drawLine(line, Offset(0f, size.height), Offset(size.width, size.height), 2.dp.toPx())
                            if (column == n - 1) drawLine(line, Offset(size.width, 0f), Offset(size.width, size.height), 2.dp.toPx())
                        }
                        .clickable { onSelect(index) }
                        .testTag("cell-${row + 1}-${column + 1}")
                        .clearAndSetSemantics {
                            contentDescription = label(index)
                            this.selected = isSelected
                            collectionItemInfo = CollectionItemInfo(row, 1, column, 1)
                            onClick("Hücreyi seç") { onSelect(index); true }
                        },
                    contentAlignment = Alignment.Center
                ) {
                    if (v != 0) Text(v.toString() + if (conflict) "!" else "", fontSize = symbolSize, fontWeight = if (isGiven) FontWeight.Bold else FontWeight.Normal, color = if (conflict) MaterialTheme.colorScheme.error else if (isGiven) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.primary)
                    // Notes are drawn as small digits (3 per line), so the selected-cell text line is not needed.
                    else if (noteList.isNotEmpty()) Text(noteList.chunked(3).joinToString("\n") { it.joinToString(" ") }, fontSize = noteSize, lineHeight = noteSize * 1.1f, textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.primary)
                }
            }
        }
    }
}
