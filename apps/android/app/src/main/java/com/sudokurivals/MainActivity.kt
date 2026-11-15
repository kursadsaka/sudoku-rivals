package com.sudokurivals

import android.os.Bundle
import android.content.Intent
import android.view.KeyEvent
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import org.json.JSONObject

class MainActivity : ComponentActivity() {
    private lateinit var solo: SoloState
    private lateinit var live: LiveState
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        solo = SoloState(filesDir)
        live = LiveState(filesDir, getString(R.string.guest_name))
        live.onHome = { if (solo.game != null) solo.exit() else solo.screen = SoloScreen.Home }
        if (BuildConfig.DEBUG) applyDebugExtras(intent)
        intent.dataString?.let { solo.pause(); live.openInvite(it) }
        setContent { SoloApp(solo, live) }
    }
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (BuildConfig.DEBUG) applyDebugExtras(intent)
        intent.dataString?.let { solo.pause(); live.openInvite(it) }
    }
    private fun applyDebugExtras(intent: Intent) {
        intent.getStringExtra("dev_server_url")?.let(live::configureServer)
        if (intent.hasExtra("debug_ui")) UiDebug.enabled = intent.getBooleanExtra("debug_ui", false)
    }
    override fun onStart() {
        super.onStart()
        if (::live.isInitialized) live.setForeground(true)
    }
    override fun onDestroy() {
        if (::live.isInitialized) live.close()
        super.onDestroy()
    }
    override fun onStop() {
        if (::solo.isInitialized) solo.pause()
        if (::live.isInitialized) live.setForeground(false)
        super.onStop()
    }
    // Public Activity override; AndroidX marks its intermediate implementation restricted.
    @android.annotation.SuppressLint("RestrictedApi")
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        val keyCode = event.keyCode
        if (::live.isInitialized && live.visible && event.action == KeyEvent.ACTION_DOWN && live.snapshot()?.optString("phase") == "playing" && !live.leaveConfirmation && !live.resetConfirmation) {
            val n = live.snapshot()!!.getInt("size")
            val race = live.snapshot()!!.getString("mode") == "race"
            val digit = when (keyCode) {
                in KeyEvent.KEYCODE_1..KeyEvent.KEYCODE_9 -> keyCode - KeyEvent.KEYCODE_0
                in KeyEvent.KEYCODE_NUMPAD_1..KeyEvent.KEYCODE_NUMPAD_9 -> keyCode - KeyEvent.KEYCODE_NUMPAD_0
                else -> null
            }
            if (digit != null && digit <= n) { if (event.repeatCount == 0) live.enter(digit); return true }
            when (keyCode) {
                KeyEvent.KEYCODE_DPAD_LEFT -> live.moveSelection(0, -1)
                KeyEvent.KEYCODE_DPAD_RIGHT -> live.moveSelection(0, 1)
                KeyEvent.KEYCODE_DPAD_UP -> live.moveSelection(-1, 0)
                KeyEvent.KEYCODE_DPAD_DOWN -> live.moveSelection(1, 0)
                KeyEvent.KEYCODE_N -> if (race && event.repeatCount == 0) { live.noteMode = !live.noteMode }
                KeyEvent.KEYCODE_DEL, KeyEvent.KEYCODE_FORWARD_DEL -> if (race && live.canInput()) live.edit("erase")
                KeyEvent.KEYCODE_Z -> if (event.isCtrlPressed || event.isMetaPressed) { if (race && live.canInput()) live.edit(if (event.isShiftPressed) "redo" else "undo") } else return super.dispatchKeyEvent(event)
                KeyEvent.KEYCODE_ESCAPE -> live.goHome()
                else -> return super.dispatchKeyEvent(event)
            }
            return true
        }
        if (::solo.isInitialized && solo.screen == SoloScreen.Game && solo.hint == null && event.action == KeyEvent.ACTION_DOWN) {
            val digit = when (keyCode) {
                in KeyEvent.KEYCODE_1..KeyEvent.KEYCODE_9 -> keyCode - KeyEvent.KEYCODE_0
                in KeyEvent.KEYCODE_NUMPAD_1..KeyEvent.KEYCODE_NUMPAD_9 -> keyCode - KeyEvent.KEYCODE_NUMPAD_0
                else -> null
            }
            if (digit != null && digit <= solo.size) {
                if (event.repeatCount == 0) solo.enter(digit)
                return true
            }
            when (keyCode) {
                KeyEvent.KEYCODE_DPAD_LEFT -> solo.move(0, -1)
                KeyEvent.KEYCODE_DPAD_RIGHT -> solo.move(0, 1)
                KeyEvent.KEYCODE_DPAD_UP -> solo.move(-1, 0)
                KeyEvent.KEYCODE_DPAD_DOWN -> solo.move(1, 0)
                KeyEvent.KEYCODE_DEL, KeyEvent.KEYCODE_FORWARD_DEL -> solo.apply("erase")
                KeyEvent.KEYCODE_N -> if (event.repeatCount == 0) { solo.noteMode = !solo.noteMode }
                KeyEvent.KEYCODE_Z -> if (event.isCtrlPressed || event.isMetaPressed) solo.apply(if (event.isShiftPressed) "redo" else "undo") else return super.dispatchKeyEvent(event)
                KeyEvent.KEYCODE_ESCAPE -> solo.pause()
                else -> return super.dispatchKeyEvent(event)
            }
            return true
        }
        return super.dispatchKeyEvent(event)
    }
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun SoloApp(solo: SoloState, live: LiveState) {
    val dark = isSystemInDarkTheme()
    val colors = if (dark) darkColorScheme(
        background = Color(0xff111827), surface = Color(0xff111827), onSurface = Color(0xfff9fafb),
        primary = Color(0xff93c5fd), onPrimary = Color(0xff111827), primaryContainer = Color(0xff1e3a5f), error = Color(0xfffca5a5)
    ) else lightColorScheme(
        background = Color.White, surface = Color.White, onSurface = Color(0xff111827),
        primary = Color(0xff1d4ed8), onPrimary = Color.White, primaryContainer = Color(0xffdbeafe), error = Color(0xffb91c1c)
    )
    MaterialTheme(colorScheme = colors) {
        Surface(Modifier.fillMaxSize().semantics { testTagsAsResourceId = true }) {
            if (live.visible) {
                LiveScreens(live)
            } else {
            BackHandler(enabled = solo.screen != SoloScreen.Home) {
                when (solo.screen) {
                    SoloScreen.Game -> solo.pause()
                    SoloScreen.Paused, SoloScreen.Result -> solo.exit()
                    else -> solo.screen = SoloScreen.Home
                }
            }
            LaunchedEffect(solo.screen) {
                while (solo.screen == SoloScreen.Game) { solo.tick(); delay(1000) }
            }
            if (solo.screen == SoloScreen.Game) {
                Box(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)) { Game(solo) }
            } else Column(
                Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)
                    .verticalScroll(rememberScrollState()).padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                Text(res(R.string.app_title), Modifier.testTag("app-title").semantics { heading() }, style = MaterialTheme.typography.headlineMedium)
                if (solo.screen != SoloScreen.Home) TextButton(onClick = {
                    when (solo.screen) {
                        SoloScreen.Game -> solo.pause()
                        SoloScreen.Paused, SoloScreen.Result -> solo.exit()
                        else -> solo.screen = SoloScreen.Home
                    }
                }, modifier = Modifier.testTag("back")) { Text(res(R.string.action_back)) }
                if (solo.storageError != null) StorageStatus(solo)
                when (solo.screen) {
                    SoloScreen.Home -> Home(solo, live)
                    SoloScreen.Setup -> Setup(solo)
                    SoloScreen.Game -> Unit
                    SoloScreen.Paused -> {
                        Text(res(R.string.paused_title), style = MaterialTheme.typography.titleLarge)
                        Text(res(R.string.paused_body, formatTime(solo.elapsedDisplay)))
                        Button(onClick = solo::resume, modifier = Modifier.testTag("resume")) { Text(res(R.string.action_resume)) }
                        OutlinedButton(onClick = { solo.exit() }, modifier = Modifier.testTag("save-exit")) { Text(res(R.string.action_save_exit)) }
                    }
                    SoloScreen.Result -> {
                        Text(res(R.string.solo_result_title), Modifier.testTag("result-heading").semantics { heading() }, style = MaterialTheme.typography.titleLarge)
                        Text("${solo.size}×${solo.size} · ${formatTime(solo.elapsedDisplay)}", Modifier.testTag("result-details"))
                        Button(onClick = { solo.exit() }, modifier = Modifier.testTag("home")) { Text(res(R.string.action_home)) }
                    }
                    SoloScreen.Settings -> {
                        Text(res(R.string.home_settings), style = MaterialTheme.typography.titleLarge)
                        Text(res(R.string.settings_body_system))
                        Text(res(R.string.settings_body_storage))
                    }
                }
                if (solo.storageError == null) StorageStatus(solo)
            }
            solo.hint?.let { hint ->
                val row = hint.getInt("index") / solo.size + 1
                val column = hint.getInt("index") % solo.size + 1
                val value = hint.getInt("value")
                val explanation = if (hint.getString("technique") == "naked_single")
                    res(R.string.hint_naked, value)
                else {
                    val unit = hint.getJSONObject("unit")
                    res(when (unit.getString("kind")) { "row" -> R.string.hint_hidden_row; "column" -> R.string.hint_hidden_column; else -> R.string.hint_hidden_box }, unit.getInt("index") + 1, value)
                }
                AlertDialog(
                    onDismissRequest = { solo.hint = null }, title = { Text(res(R.string.hint_title, row, column, value)) },
                    text = { Text(explanation) },
                    confirmButton = { TextButton(onClick = solo::acceptHint) { Text(res(R.string.action_apply)) } },
                    dismissButton = { TextButton(onClick = { solo.hint = null }) { Text(res(R.string.action_cancel)) } }
                )
            }
            if (solo.replaceRequested) AlertDialog(
                onDismissRequest = { solo.replaceRequested = false },
                title = { Text(res(R.string.replace_title)) },
                text = { Text(res(if (solo.corruptReplacement) R.string.replace_body_corrupt else R.string.replace_body)) },
                confirmButton = { TextButton(onClick = solo::newGame) { Text(res(R.string.action_new_game)) } },
                dismissButton = { TextButton(onClick = { solo.replaceRequested = false }) { Text(res(R.string.action_cancel)) } }
            )
            }
        }
    }
}

@Composable
private fun Home(solo: SoloState, live: LiveState) {
    Text(res(R.string.home_title), style = MaterialTheme.typography.titleLarge)
    Text(res(R.string.home_subtitle))
    Button(onClick = solo::openSetup, modifier = Modifier.fillMaxWidth().testTag("solo-start")) { Text(res(R.string.home_solo_start)) }
    SizeChoice(solo)
    if (solo.slotStatus[solo.size] != "empty") {
        OutlinedButton(onClick = solo::resumeSaved, modifier = Modifier.fillMaxWidth().testTag("solo-resume")) {
            Text(res(if (solo.slotStatus[solo.size] == "corrupt") R.string.home_reopen_size else R.string.home_resume_size, solo.size))
        }
    }
    OutlinedButton(onClick = live::open, modifier = Modifier.fillMaxWidth().testTag("friend-start")) { Text(res(if (live.snapshot() != null && !live.view.optBoolean("voluntarily_left")) R.string.home_friend_return else R.string.home_friend_start)) }
    TextButton(onClick = { solo.screen = SoloScreen.Settings }, modifier = Modifier.testTag("settings-open")) { Text(res(R.string.home_settings)) }
}

@Composable
private fun SizeChoice(solo: SoloState) {
    val boardSizeLabel = res(R.string.board_size)
    Column(Modifier.testTag("board-size").semantics { contentDescription = boardSizeLabel }) {
        Text(res(R.string.board_size), style = MaterialTheme.typography.labelLarge)
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            for (size in listOf(6, 9)) FilterChip(
                selected = solo.size == size,
                onClick = { solo.selectSize(size) },
                label = { Text("$size×$size") },
                modifier = Modifier.testTag("board-size-$size").heightIn(min = 48.dp)
            )
        }
    }
}

@Composable
private fun Setup(solo: SoloState) {
    Text(res(R.string.setup_title), style = MaterialTheme.typography.titleLarge)
    SizeChoice(solo)
    Text(res(R.string.setup_note))
    Button(onClick = solo::prepareNew, modifier = Modifier.fillMaxWidth().testTag("game-start")) { Text(res(R.string.setup_start)) }
}

@Composable
private fun Game(solo: SoloState) {
    val state = JSONObject(solo.game ?: return)
    val cells = state.getJSONArray("cells")
    val givens = state.getJSONArray("givens")
    val conflicts = solo.conflicts()
    val valid = conflicts.isEmpty()
    val editable = givens.getInt(solo.selected) == 0
    val context = LocalContext.current
    LocalConfiguration.current
    val statusDescription = res(if (valid) R.string.status_in_progress else R.string.status_conflict_desc)
    val notesDescription = res(R.string.cd_notes)
    val undoDescription = res(R.string.cd_undo)
    val redoDescription = res(R.string.cd_redo)
    val hintDescription = res(R.string.cd_hint)
    GameScreenLayout(
        top = {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                if (UiDebug.enabled) Text(res(R.string.app_title), Modifier.testTag("app-title"), style = MaterialTheme.typography.labelMedium)
                GameNavBar(
                    modeLabel = res(R.string.solo_mode_label, solo.size), timeText = formatTime(solo.elapsedDisplay),
                    timeDescription = res(R.string.elapsed_description, formatTime(solo.elapsedDisplay)), backDescription = res(R.string.action_back), onBack = solo::pause
                ) { TextButton(onClick = solo::pause, modifier = Modifier.heightIn(min = 48.dp).testTag("pause"), contentPadding = PaddingValues(4.dp)) { Text(res(R.string.action_pause)) } }
                // One fixed-height status row. Healthy play shows no text (screen readers still get it).
                Row(Modifier.fillMaxWidth().heightIn(min = 24.dp).testTag("game-status").semantics { liveRegion = LiveRegionMode.Polite; contentDescription = statusDescription }, verticalAlignment = Alignment.CenterVertically) {
                    if (!valid) Text(res(R.string.status_conflict), Modifier.clearAndSetSemantics { }, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                    else if (UiDebug.enabled) Text(res(R.string.status_in_progress), Modifier.clearAndSetSemantics { }, style = MaterialTheme.typography.bodyMedium)
                }
            }
        },
        board = { side ->
            SudokuBoard(
                n = solo.size, side = side, selected = solo.selected, conflicts = conflicts,
                value = { cells.getJSONObject(it).getInt("value") }, given = { givens.getInt(it) != 0 },
                notes = { i -> val bits = cells.getJSONObject(i).getInt("notes"); (1..solo.size).filter { bits and (1 shl (it - 1)) != 0 } },
                label = { index ->
                    val cell = cells.getJSONObject(index)
                    val value = cell.getInt("value")
                    cellLabel(context, index, solo.size, if (givens.getInt(index) != 0) R.string.cell_given else R.string.cell_editable, value,
                        (1..solo.size).filter { cell.getInt("notes") and (1 shl (it - 1)) != 0 }, index in conflicts, " ")
                },
                onSelect = { solo.selected = it }
            )
        },
        bottom = {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (UiDebug.enabled) {
                    val selectedCell = cells.getJSONObject(solo.selected)
                    val selectedNotes = selectedCell.getInt("notes")
                    Text(res(R.string.selected_cell_line, solo.selected / solo.size + 1, solo.selected % solo.size + 1,
                        (if (selectedCell.getInt("value") == 0) res(R.string.selected_cell_empty) else res(R.string.selected_cell_value, selectedCell.getInt("value"))) +
                            if (selectedNotes == 0) "" else res(R.string.selected_cell_notes_suffix, (1..solo.size).filter { selectedNotes and (1 shl (it - 1)) != 0 }.joinToString(", "))),
                        Modifier.testTag("selected-cell"), style = MaterialTheme.typography.bodyMedium)
                }
                DigitPad(solo.size, editable) { solo.enter(it) }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    FilterChip(selected = solo.noteMode, onClick = { solo.noteMode = !solo.noteMode }, label = { Text("✎", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }, modifier = Modifier.weight(1f).testTag("notes-toggle").heightIn(min = 48.dp).semantics { contentDescription = notesDescription })
                    OutlinedButton(onClick = { solo.apply("erase") }, enabled = editable, modifier = Modifier.weight(1f).testTag("erase").heightIn(min = 48.dp), contentPadding = PaddingValues(4.dp)) { Text(res(R.string.action_erase)) }
                    OutlinedButton(onClick = { solo.apply("undo") }, enabled = state.getJSONArray("undo").length() > 0, modifier = Modifier.weight(1f).testTag("undo").heightIn(min = 48.dp).semantics { contentDescription = undoDescription }, contentPadding = PaddingValues(4.dp)) { Text("↶", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
                    OutlinedButton(onClick = { solo.apply("redo") }, enabled = state.getJSONArray("redo").length() > 0, modifier = Modifier.weight(1f).testTag("redo").heightIn(min = 48.dp).semantics { contentDescription = redoDescription }, contentPadding = PaddingValues(4.dp)) { Text("↷", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
                    OutlinedButton(onClick = solo::requestHint, modifier = Modifier.weight(1f).testTag("hint").heightIn(min = 48.dp).semantics { contentDescription = hintDescription }, contentPadding = PaddingValues(4.dp)) { Text("?", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
                }
                // Transient notices only; each grows the area only while it is shown.
                if (solo.storageError != null || UiDebug.enabled) StorageStatus(solo)
                solo.hintMessage?.let { Text(it.asString(), Modifier.semantics { liveRegion = LiveRegionMode.Polite }, style = MaterialTheme.typography.bodyMedium) }
                solo.inputError?.let { Text(it.asString(), Modifier.semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error) }
            }
        }
    )
}

@Composable
private fun StorageStatus(solo: SoloState) {
    if (solo.storageError != null) {
        Text(solo.storageError!!.asString(), Modifier.testTag("storage-status").semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error)
        OutlinedButton(onClick = { if (solo.screen == SoloScreen.Setup) solo.prepareNew() else if (solo.game == null) solo.resumeSaved() else solo.save() }, modifier = Modifier.testTag("retry")) { Text(res(R.string.action_retry)) }
    } else if (solo.game != null) Text(res(R.string.storage_saved), Modifier.testTag("storage-status"), style = MaterialTheme.typography.bodySmall)
}

private fun formatTime(milliseconds: Long): String {
    val seconds = milliseconds / 1000
    return "%02d:%02d".format(seconds / 60, seconds % 60)
}
