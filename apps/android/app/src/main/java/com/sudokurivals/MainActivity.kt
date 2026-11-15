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
        live = LiveState(filesDir)
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
                Text("Sudoku Rivals", Modifier.testTag("app-title").semantics { heading() }, style = MaterialTheme.typography.headlineMedium)
                if (solo.screen != SoloScreen.Home) TextButton(onClick = {
                    when (solo.screen) {
                        SoloScreen.Game -> solo.pause()
                        SoloScreen.Paused, SoloScreen.Result -> solo.exit()
                        else -> solo.screen = SoloScreen.Home
                    }
                }, modifier = Modifier.testTag("back")) { Text("Geri") }
                if (solo.storageError != null) StorageStatus(solo)
                when (solo.screen) {
                    SoloScreen.Home -> Home(solo, live)
                    SoloScreen.Setup -> Setup(solo)
                    SoloScreen.Game -> Unit
                    SoloScreen.Paused -> {
                        Text("Oyun duraklatıldı", style = MaterialTheme.typography.titleLarge)
                        Text("Tahtan gizli. ${formatTime(solo.elapsedDisplay)} oynadın.")
                        Button(onClick = solo::resume, modifier = Modifier.testTag("resume")) { Text("Devam et") }
                        OutlinedButton(onClick = { solo.exit() }, modifier = Modifier.testTag("save-exit")) { Text("Kaydedip çık") }
                    }
                    SoloScreen.Result -> {
                        Text("Sudoku tamamlandı", Modifier.testTag("result-heading").semantics { heading() }, style = MaterialTheme.typography.titleLarge)
                        Text("${solo.size}×${solo.size} · ${formatTime(solo.elapsedDisplay)}", Modifier.testTag("result-details"))
                        Button(onClick = { solo.exit() }, modifier = Modifier.testTag("home")) { Text("Ana sayfa") }
                    }
                    SoloScreen.Settings -> {
                        Text("Ayarlar", style = MaterialTheme.typography.titleLarge)
                        Text("Sistem teması ve yazı boyutu kullanılır. Oyun ses ve titreşim üretmez; hareketli geçiş içermez.")
                        Text("Kayıtlar yalnız bu cihazda tutulur. Uygulama silinirse kayıtlar kaybolur.")
                    }
                }
                if (solo.storageError == null) StorageStatus(solo)
            }
            solo.hint?.let { hint ->
                val row = hint.getInt("index") / solo.size + 1
                val column = hint.getInt("index") % solo.size + 1
                val value = hint.getInt("value")
                val explanation = if (hint.getString("technique") == "naked_single")
                    "Bu hücrede satır, sütun ve kutu kurallarına göre yalnız $value kalıyor."
                else {
                    val unit = hint.getJSONObject("unit")
                    val name = when (unit.getString("kind")) { "row" -> "satırda"; "column" -> "sütunda"; else -> "kutuda" }
                    "${unit.getInt("index") + 1}. $name $value rakamının yerleşebileceği tek hücre burası."
                }
                AlertDialog(
                    onDismissRequest = { solo.hint = null }, title = { Text("$row. satır, $column. sütun: $value") },
                    text = { Text(explanation) },
                    confirmButton = { TextButton(onClick = solo::acceptHint) { Text("Uygula") } },
                    dismissButton = { TextButton(onClick = { solo.hint = null }) { Text("Vazgeç") } }
                )
            }
            if (solo.replaceRequested) AlertDialog(
                onDismissRequest = { solo.replaceRequested = false },
                title = { Text("Yeni oyun başlatılsın mı?") },
                text = { Text(if (solo.corruptReplacement) "Açılamayan kayıt ayrıca korunacak. Bu boyut için yeni oyun başlatılacak." else "Yeni oyun bu boyuttaki kaydının yerini alacak. Diğer boyuttaki oyun korunur.") },
                confirmButton = { TextButton(onClick = solo::newGame) { Text("Yeni oyun") } },
                dismissButton = { TextButton(onClick = { solo.replaceRequested = false }) { Text("Vazgeç") } }
            )
            }
        }
    }
}

@Composable
private fun Home(solo: SoloState, live: LiveState) {
    Text("Kendi hızında çöz", style = MaterialTheme.typography.titleLarge)
    Text("6×6 veya 9×9 Sudoku. İnternet gerekmez.")
    Button(onClick = solo::openSetup, modifier = Modifier.fillMaxWidth().testTag("solo-start")) { Text("Tek başına oyna") }
    SizeChoice(solo)
    if (solo.slotStatus[solo.size] != "empty") {
        OutlinedButton(onClick = solo::resumeSaved, modifier = Modifier.fillMaxWidth().testTag("solo-resume")) {
            Text(if (solo.slotStatus[solo.size] == "corrupt") "${solo.size}×${solo.size} kaydını tekrar aç" else "${solo.size}×${solo.size} oyununa devam et")
        }
    }
    OutlinedButton(onClick = live::open, modifier = Modifier.fillMaxWidth().testTag("friend-start")) { Text(if (live.snapshot() != null && !live.view.optBoolean("voluntarily_left")) "Maçına dön" else "Arkadaşınla yarış") }
    TextButton(onClick = { solo.screen = SoloScreen.Settings }, modifier = Modifier.testTag("settings-open")) { Text("Ayarlar") }
}

@Composable
private fun SizeChoice(solo: SoloState) {
    Column(Modifier.testTag("board-size").semantics { contentDescription = "Tahta boyutu" }) {
        Text("Tahta boyutu", style = MaterialTheme.typography.labelLarge)
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
    Text("Tek başına", style = MaterialTheme.typography.titleLarge)
    SizeChoice(solo)
    Text("Her bulmacanın tek çözümü vardır. İnsan tekniklerine göre zorluk sınıflandırması henüz sunulmuyor.")
    Button(onClick = solo::prepareNew, modifier = Modifier.fillMaxWidth().testTag("game-start")) { Text("Oyunu başlat") }
}

@Composable
private fun Game(solo: SoloState) {
    val state = JSONObject(solo.game ?: return)
    val cells = state.getJSONArray("cells")
    val givens = state.getJSONArray("givens")
    val conflicts = solo.conflicts()
    val valid = conflicts.isEmpty()
    val editable = givens.getInt(solo.selected) == 0
    GameScreenLayout(
        top = {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                if (UiDebug.enabled) Text("Sudoku Rivals", Modifier.testTag("app-title"), style = MaterialTheme.typography.labelMedium)
                GameNavBar(
                    modeLabel = "Tek başına · ${solo.size}×${solo.size}", timeText = formatTime(solo.elapsedDisplay),
                    timeDescription = "Geçen süre: ${formatTime(solo.elapsedDisplay)}", backDescription = "Geri", onBack = solo::pause
                ) { TextButton(onClick = solo::pause, modifier = Modifier.heightIn(min = 48.dp).testTag("pause"), contentPadding = PaddingValues(4.dp)) { Text("Duraklat") } }
                // One fixed-height status row. Healthy play shows no text (screen readers still get it).
                Row(Modifier.fillMaxWidth().heightIn(min = 24.dp).testTag("game-status").semantics { liveRegion = LiveRegionMode.Polite; contentDescription = if (valid) "Oyun devam ediyor" else "Tahtada çakışma var" }, verticalAlignment = Alignment.CenterVertically) {
                    if (!valid) Text("Çakışma var", Modifier.clearAndSetSemantics { }, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                    else if (UiDebug.enabled) Text("Oyun devam ediyor", Modifier.clearAndSetSemantics { }, style = MaterialTheme.typography.bodyMedium)
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
                    val notesText = (1..solo.size).filter { cell.getInt("notes") and (1 shl (it - 1)) != 0 }.joinToString(" ")
                    "Satır ${index / solo.size + 1}, sütun ${index % solo.size + 1}, ${if (givens.getInt(index) != 0) "verilen" else "girilebilir"}, ${if (value == 0) "boş" else value.toString()}, notlar: ${notesText.ifEmpty { "yok" }}${if (index in conflicts) ", çakışma" else ""}"
                },
                onSelect = { solo.selected = it }
            )
        },
        bottom = {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (UiDebug.enabled) {
                    val selectedCell = cells.getJSONObject(solo.selected)
                    val selectedNotes = selectedCell.getInt("notes")
                    Text("${solo.selected / solo.size + 1}. satır, ${solo.selected % solo.size + 1}. sütun · ${if (selectedCell.getInt("value") == 0) "Boş" else "Değer: ${selectedCell.getInt("value")}"}" + if (selectedNotes == 0) "" else " · Notlar: ${(1..solo.size).filter { selectedNotes and (1 shl (it - 1)) != 0 }.joinToString(", ")}", Modifier.testTag("selected-cell"), style = MaterialTheme.typography.bodyMedium)
                }
                DigitPad(solo.size, editable) { solo.enter(it) }
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    FilterChip(selected = solo.noteMode, onClick = { solo.noteMode = !solo.noteMode }, label = { Text("✎", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }, modifier = Modifier.weight(1f).testTag("notes-toggle").heightIn(min = 48.dp).semantics { contentDescription = "Kalem" })
                    OutlinedButton(onClick = { solo.apply("erase") }, enabled = editable, modifier = Modifier.weight(1f).testTag("erase").heightIn(min = 48.dp), contentPadding = PaddingValues(4.dp)) { Text("Sil") }
                    OutlinedButton(onClick = { solo.apply("undo") }, enabled = state.getJSONArray("undo").length() > 0, modifier = Modifier.weight(1f).testTag("undo").heightIn(min = 48.dp).semantics { contentDescription = "Geri al" }, contentPadding = PaddingValues(4.dp)) { Text("↶", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
                    OutlinedButton(onClick = { solo.apply("redo") }, enabled = state.getJSONArray("redo").length() > 0, modifier = Modifier.weight(1f).testTag("redo").heightIn(min = 48.dp).semantics { contentDescription = "Yinele" }, contentPadding = PaddingValues(4.dp)) { Text("↷", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
                    OutlinedButton(onClick = solo::requestHint, modifier = Modifier.weight(1f).testTag("hint").heightIn(min = 48.dp).semantics { contentDescription = "İpucu" }, contentPadding = PaddingValues(4.dp)) { Text("?", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
                }
                // Transient notices only; each grows the area only while it is shown.
                if (solo.storageError != null || UiDebug.enabled) StorageStatus(solo)
                solo.hintMessage?.let { Text(it, Modifier.semantics { liveRegion = LiveRegionMode.Polite }, style = MaterialTheme.typography.bodyMedium) }
                solo.inputError?.let { Text(it, Modifier.semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error) }
            }
        }
    )
}

@Composable
private fun StorageStatus(solo: SoloState) {
    if (solo.storageError != null) {
        Text(solo.storageError!!, Modifier.testTag("storage-status").semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error)
        OutlinedButton(onClick = { if (solo.screen == SoloScreen.Setup) solo.prepareNew() else if (solo.game == null) solo.resumeSaved() else solo.save() }, modifier = Modifier.testTag("retry")) { Text("Tekrar dene") }
    } else if (solo.game != null) Text("Oyun cihazına kaydedildi", Modifier.testTag("storage-status"), style = MaterialTheme.typography.bodySmall)
}

private fun formatTime(milliseconds: Long): String {
    val seconds = milliseconds / 1000
    return "%02d:%02d".format(seconds / 60, seconds % 60)
}
