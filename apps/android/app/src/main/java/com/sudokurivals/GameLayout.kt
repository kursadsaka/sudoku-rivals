package com.sudokurivals

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
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
    val compact = compactHeight()
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val viewportW = constraints.maxWidth
        val viewportH = constraints.maxHeight
        Box(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
            SubcomposeLayout(Modifier.fillMaxWidth()) { _ ->
                // Short screens (web max-height:600px) trim the chrome before the board has to give way.
                val gap = (if (compact) 4.dp else GAP).roundToPx()
                val side = SIDE.roundToPx()
                val vPad = (if (compact) 4.dp else GAP).roundToPx()
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

/**
 * Navigation row (web `.game-top`): a square back button · `Mode · N×N · Level` over a large time · the
 * right slot (pause in solo, empty in a match). A passing notice floats over the heading, between the two
 * buttons, so it never moves the board.
 */
@Composable
internal fun GameNavBar(modeLabel: String, timeText: String, timeDescription: String, backDescription: String, onBack: () -> Unit, notice: String? = null, onDismissNotice: () -> Unit = {}, trailing: @Composable () -> Unit) {
    val p = ink
    Box(Modifier.fillMaxWidth().heightIn(min = 48.dp)) {
        Row(Modifier.fillMaxWidth().heightIn(min = 48.dp), verticalAlignment = Alignment.CenterVertically) {
            SquareIconButton(Glyph.back, backDescription, onBack, Modifier.testTag("back"))
            Column(Modifier.weight(1f).padding(horizontal = 8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Text(modeLabel, Modifier.testTag("mode-label").semantics { heading() }, style = Type.label, color = p.muted, textAlign = TextAlign.Center, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text(timeText, Modifier.testTag("timer").semantics { contentDescription = timeDescription }, fontSize = 22.sp, lineHeight = 26.sp, fontWeight = FontWeight.Bold, color = p.foreground)
            }
            Box(Modifier.widthIn(min = 48.dp), contentAlignment = Alignment.CenterEnd) { trailing() }
        }
        if (notice != null) Row(
            Modifier.align(Alignment.Center).padding(horizontal = 56.dp).fillMaxWidth().heightIn(min = 48.dp)
                .clip(RoundedCornerShape(14.dp)).background(p.foreground).testTag("game-notice")
                .semantics { liveRegion = LiveRegionMode.Polite },
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(notice, Modifier.weight(1f).padding(start = 12.dp, top = 6.dp, bottom = 6.dp), color = p.canvas, fontSize = 13.sp, lineHeight = 17.sp)
            val close = res(R.string.action_dismiss)
            Box(Modifier.size(44.dp).clickable(role = Role.Button, onClick = onDismissNotice).semantics { contentDescription = close }, contentAlignment = Alignment.Center) {
                Icon(Glyph.close, null, Modifier.size(20.dp), tint = p.canvas)
            }
        }
    }
}

/** Small filled dot for a healthy connection; the dot carries the state for screen readers. */
@Composable
internal fun StatusDot(color: Color, description: String, tag: String) {
    Box(Modifier.size(22.dp).testTag(tag).semantics { contentDescription = description; liveRegion = LiveRegionMode.Polite }, contentAlignment = Alignment.Center) {
        Box(Modifier.size(8.dp).background(color, CircleShape))
    }
}

/** Short screen (web `max-height:600px`): tighter gaps, no race meters, 48dp tools. */
@Composable
internal fun compactHeight(): Boolean = LocalConfiguration.current.screenHeightDp < 600

/** Key height: taller on tall phones (the thumb zone), never under the 48dp target. */
@Composable
internal fun keyHeight(): Dp = if (LocalConfiguration.current.screenHeightDp >= 760) 56.dp else 48.dp

/**
 * Digit pad: one row on 6×6 (3×2 when narrower than six 48dp targets), 5+4 centred on 9×9. Each key shows
 * how many of that digit are still missing; a finished digit steps back. In notes mode the keys turn to
 * pencil (dashed, grey).
 */
@Composable
internal fun DigitPad(n: Int, enabled: Boolean, notes: Boolean = false, remaining: (Int) -> Int = { 1 }, label: (Int) -> String = { it.toString() }, onDigit: (Int) -> Unit) {
    val p = ink
    val height = keyHeight()
    BoxWithConstraints(Modifier.fillMaxWidth()) {
        val gap = 6.dp
        val columns = if (n == 6) (if (maxWidth >= 48.dp * 6 + gap * 5) 6 else 3) else 5
        Column(verticalArrangement = Arrangement.spacedBy(gap)) {
            for (digits in (1..n).chunked(columns)) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(gap)) {
                val spare = columns - digits.size
                if (spare > 0) Spacer(Modifier.weight(spare / 2f))
                for (digit in digits) {
                    val left = remaining(digit)
                    val shape = RoundedCornerShape(14.dp)
                    val description = label(digit)
                    Box(
                        Modifier.weight(1f).height(height).clip(shape)
                            .background(if (left == 0) Color.Transparent else p.background)
                            .then(if (notes) Modifier.drawBehind {
                                drawRoundRect(p.border, cornerRadius = androidx.compose.ui.geometry.CornerRadius(14.dp.toPx()),
                                    style = Stroke(1.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(4.dp.toPx(), 3.dp.toPx()))))
                            } else Modifier.border(1.dp, p.line, shape))
                            .clickable(enabled = enabled, role = Role.Button) { onDigit(digit) }
                            .testTag("number-$digit")
                            .clearAndSetSemantics { contentDescription = description; if (!enabled) disabled() ; onClick { onDigit(digit); true } },
                        contentAlignment = Alignment.Center,
                    ) {
                        val color = when { !enabled -> p.muted.copy(alpha = .45f); notes || left == 0 -> p.muted; else -> p.accent }
                        Text(digit.toString(), color = color, fontSize = if (notes) 20.sp else 24.sp, fontWeight = FontWeight.SemiBold)
                        if (left > 0) Text(left.toString(), Modifier.align(Alignment.TopEnd).padding(top = 3.dp, end = 6.dp), color = p.muted.copy(alpha = if (enabled) 1f else .45f), fontSize = 10.sp, lineHeight = 10.sp, fontWeight = FontWeight.SemiBold)
                    }
                }
                if (spare > 0) Spacer(Modifier.weight(spare / 2f))
            }
        }
    }
}

/** One tool under the digits: icon over a short label; the notes switch fills when on. */
@Composable
internal fun RowScope.Tool(icon: ImageVector, label: String, tag: String, enabled: Boolean = true, checked: Boolean? = null, description: String = label, onClick: () -> Unit) {
    val p = ink
    val on = checked == true
    Column(
        Modifier.weight(1f).heightIn(min = if (compactHeight()) 48.dp else 52.dp).clip(RoundedCornerShape(14.dp)).background(if (on) p.brand else Color.Transparent)
            .then(if (checked != null) Modifier.toggleable(on, enabled = enabled, role = Role.Switch) { onClick() } else Modifier.clickable(enabled = enabled, role = Role.Button, onClick = onClick))
            .testTag(tag).semantics { contentDescription = description }.padding(vertical = 4.dp),
        horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center,
    ) {
        val alpha = if (enabled) 1f else .45f
        Icon(icon, null, Modifier.size(22.dp), tint = (if (on) p.onBrand else p.foreground).copy(alpha = alpha))
        Text(label, color = (if (on) p.onBrand else p.muted).copy(alpha = alpha), fontSize = 11.sp, lineHeight = 13.sp, fontWeight = FontWeight.Medium, maxLines = 1, modifier = Modifier.clearAndSetSemantics { })
    }
}

/**
 * The keys (digits + tools) with an optional card laid over them (the hint). The card takes exactly the
 * keys' area, so the board and the keys keep their place and size while it is read.
 */
@Composable
internal fun KeysWithOverlay(keys: @Composable () -> Unit, overlay: (@Composable () -> Unit)?) {
    Box(Modifier.fillMaxWidth()) {
        Column(verticalArrangement = Arrangement.spacedBy(if (compactHeight()) 6.dp else 8.dp)) { keys() }
        // A pointer handler makes the card the hit target, so a tap on it never reaches a key underneath
        // (it consumes nothing, so the card's own buttons still work).
        if (overlay != null) Box(Modifier.matchParentSize().pointerInput(Unit) {
            awaitPointerEventScope { while (true) awaitPointerEvent(PointerEventPass.Final) }
        }) { overlay() }
    }
}

/** Shared board: tag, grid semantics and per-cell labels are identical for solo and live. */
@Composable
internal fun SudokuBoard(
    n: Int, side: Dp, selected: Int, conflicts: Collection<Int>,
    value: (Int) -> Int, given: (Int) -> Boolean, notes: (Int) -> List<Int>, label: (Int) -> String, onSelect: (Int) -> Unit,
    marked: Set<Int> = emptySet(),
) {
    val p = ink
    val fontScale = LocalDensity.current.fontScale
    val selectAction = res(R.string.cell_select_action)
    val boxRows = if (n == 6) 2 else 3
    val symbolSize = (side.value / n * 0.56f / fontScale).sp
    val noteSize = (maxOf(7f, side.value / n * 0.27f) / fontScale).sp
    val selectedRow = selected / n
    val selectedColumn = selected % n
    val selectedValue = value(selected)
    val shape = RoundedCornerShape(12.dp)
    Column(Modifier.size(side).clip(shape).background(p.background).border(2.dp, p.grid, shape).padding(2.dp).testTag("game-board").semantics { collectionInfo = CollectionInfo(n, n) }) {
        for (row in 0 until n) Row(Modifier.weight(1f)) {
            for (column in 0 until n) {
                val index = row * n + column
                val v = value(index)
                val noteList = notes(index)
                val isGiven = given(index)
                val isSelected = selected == index
                val conflict = index in conflicts
                val related = row == selectedRow || column == selectedColumn || (row / boxRows == selectedRow / boxRows && column / 3 == selectedColumn / 3)
                val fill = when {
                    isSelected -> p.brand
                    conflict -> p.errorBackground
                    index in marked -> p.selected
                    selectedValue != 0 && v == selectedValue -> p.same
                    related -> p.related
                    else -> Color.Transparent
                }
                val text = when { isSelected -> p.onBrand; conflict -> p.error; isGiven -> p.foreground; else -> p.accent }
                Box(
                    Modifier.weight(1f).fillMaxHeight().background(fill)
                        .drawBehind {
                            val thin = 1.dp.toPx()
                            val thick = 2.dp.toPx()
                            if (column < n - 1) { val w = if (column % 3 == 2) thick else thin; drawLine(if (column % 3 == 2) p.grid else p.line, Offset(size.width - w / 2, 0f), Offset(size.width - w / 2, size.height), w) }
                            if (row < n - 1) { val w = if (row % boxRows == boxRows - 1) thick else thin; drawLine(if (row % boxRows == boxRows - 1) p.grid else p.line, Offset(0f, size.height - w / 2), Offset(size.width, size.height - w / 2), w) }
                            if (index in marked && !isSelected) drawRect(p.accent, style = Stroke(2.dp.toPx()))
                        }
                        .clickable { onSelect(index) }
                        .testTag("cell-${row + 1}-${column + 1}")
                        .clearAndSetSemantics {
                            contentDescription = label(index)
                            this.selected = isSelected
                            collectionItemInfo = CollectionItemInfo(row, 1, column, 1)
                            onClick(selectAction) { onSelect(index); true }
                        },
                    contentAlignment = Alignment.Center
                ) {
                    if (v != 0) Text(v.toString(), fontSize = symbolSize, lineHeight = symbolSize, fontWeight = if (isGiven) FontWeight.SemiBold else FontWeight.Medium, color = text)
                    else if (noteList.isNotEmpty()) Column(Modifier.fillMaxSize().padding(2.dp), verticalArrangement = Arrangement.Center) {
                        // Pencil notes on a 3-column grid, each digit in its own place.
                        for (line in (1..n).chunked(3)) Row(Modifier.fillMaxWidth()) {
                            for (digit in line) Text(if (digit in noteList) digit.toString() else "", Modifier.weight(1f), fontSize = noteSize, lineHeight = noteSize * 1.25f,
                                textAlign = TextAlign.Center, color = if (isSelected) p.onBrand else p.muted, fontWeight = FontWeight.Medium)
                        }
                    }
                    if (conflict) Text("!", Modifier.align(Alignment.TopEnd).padding(top = 1.dp, end = 3.dp), fontSize = 9.sp, lineHeight = 9.sp, fontWeight = FontWeight.ExtraBold, color = text)
                }
            }
        }
    }
}

/** How many of each digit are still missing on the board (index = digit). */
internal fun remainingCounts(n: Int, value: (Int) -> Int): IntArray {
    val left = IntArray(n + 1) { n }
    for (i in 0 until n * n) { val v = value(i); if (v in 1..n) left[v] = (left[v] - 1).coerceAtLeast(0) }
    return left
}

/** Share of the empty cells that now hold a number (the same measure the server reports for a rival). */
internal fun filledShare(n: Int, value: (Int) -> Int, given: (Int) -> Boolean): Float {
    var blanks = 0; var filled = 0
    for (i in 0 until n * n) if (!given(i)) { blanks++; if (value(i) != 0) filled++ }
    return if (blanks == 0) 0f else filled.toFloat() / blanks
}
