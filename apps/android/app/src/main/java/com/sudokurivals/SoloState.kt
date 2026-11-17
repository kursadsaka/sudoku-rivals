package com.sudokurivals

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.AtomicFile
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.io.ByteArrayOutputStream
import java.security.SecureRandom
import java.util.concurrent.Executors
import uniffi.sudoku_ffi.newRatedSolo
import uniffi.sudoku_ffi.soloConflicts
import uniffi.sudoku_ffi.soloDifficulty
import uniffi.sudoku_ffi.soloIsComplete
import uniffi.sudoku_ffi.soloTechniqueHint
import uniffi.sudoku_ffi.updateSolo

internal enum class SoloScreen { Home, Setup, Game, Paused, Result, Settings }
internal data class SoloRecord(val game: String, val elapsedMillis: Long)

/** Disk writes use AtomicFile: a failed write preserves the previous complete record. */
internal class SoloStore(private val directory: File) {
    private fun file(size: Int) = AtomicFile(File(directory, "solo-$size.json"))
    fun exists(size: Int) = file(size).baseFile.exists() || File(directory, "solo-$size.json.bak").exists()
    fun load(size: Int): SoloRecord {
        val bytes = file(size).openRead().use { input ->
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(8192)
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                require(output.size() + count <= 1_200_000)
                output.write(buffer, 0, count)
            }
            output.toByteArray()
        }
        val envelope = JSONObject(String(bytes, Charsets.UTF_8))
        require(envelope.getInt("version") == 1 && envelope.getInt("size") == size)
        val elapsed = envelope.getLong("elapsedMillis")
        require(elapsed in 0..315_360_000_000L)
        val game = envelope.getString("game")
        val state = JSONObject(game)
        require(state.getInt("size") == size)
        soloIsComplete(game) // Validate cells, givens and both history chains in Rust.
        return SoloRecord(game, elapsed)
    }
    fun save(size: Int, record: SoloRecord) {
        val bytes = JSONObject().put("version", 1).put("size", size)
            .put("elapsedMillis", record.elapsedMillis).put("game", record.game)
            .toString().toByteArray(Charsets.UTF_8)
        atomicWrite(file(size), bytes)
    }
    fun preserveCorrupt(size: Int) {
        val recovery = AtomicFile(File(directory, "solo-$size-recovery-${System.currentTimeMillis()}.json"))
        var stream: FileOutputStream? = null
        try {
            stream = recovery.startWrite()
            file(size).openRead().use { it.copyTo(stream) }
            stream.fd.sync()
            recovery.finishWrite(stream)
        } catch (error: Exception) {
            recovery.failWrite(stream)
            throw error
        }
    }
    private fun atomicWrite(file: AtomicFile, bytes: ByteArray) {
        var stream: FileOutputStream? = null
        try {
            stream = file.startWrite()
            stream.write(bytes)
            stream.fd.sync()
            file.finishWrite(stream)
            require(file.openRead().use { it.readBytes() }.contentEquals(bytes))
        } catch (error: Exception) {
            file.failWrite(stream)
            throw error
        }
    }
}

/** What the home screen knows about one size's save: empty, saved (with its details) or corrupt. */
internal data class Slot(val status: String, val complete: Boolean = false, val elapsedMillis: Long = 0, val level: String? = null)

internal class SoloState(directory: File, private val preferences: AppPreferences, private val cue: (Cue) -> Unit = {}) {
    private val store = SoloStore(directory)
    private val random = SecureRandom()
    private val worker = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    var screen by mutableStateOf(SoloScreen.Home)
    /** 9×9 until the player picks a size; the last pick is remembered. */
    var size by mutableStateOf(preferences.soloSize)
    var difficulty by mutableStateOf(preferences.soloDifficulty)
        private set
    var selected by mutableStateOf(0)
    var noteMode by mutableStateOf(false)
    var game by mutableStateOf<String?>(null)
        private set
    var elapsedDisplay by mutableStateOf(0L)
        private set
    var storageError by mutableStateOf<UiText?>(null)
        private set
    var inputError by mutableStateOf<UiText?>(null)
        private set
    var slots by mutableStateOf(mapOf<Int, Slot>())
        private set
    var hint by mutableStateOf<JSONObject?>(null)
    var hintMessage by mutableStateOf<UiText?>(null)
    var replaceRequested by mutableStateOf(false)
    var corruptReplacement by mutableStateOf(false)
    /** A rated puzzle is being dug off the main thread; the start button says so. */
    var preparing by mutableStateOf(false)
        private set
    /** Where Settings returns to. */
    var settingsReturn = SoloScreen.Home
    private var elapsedBase = 0L
    private var runningSince: Long? = null
    private val levels = HashMap<String, String?>()

    init { refreshSlots() }

    /** The puzzle's level, read from its givens by the engine; saves carry no extra field, and unrated or older puzzles have none. */
    fun levelOf(saved: String?): String? {
        if (saved == null) return null
        val key = runCatching { JSONObject(saved).getJSONArray("givens").toString() }.getOrNull() ?: return null
        return levels.getOrPut(key) { runCatching { JSONObject(soloDifficulty(saved)).optString("level").takeIf { it in LEVELS } }.getOrNull() }
    }

    fun refreshSlots() {
        slots = listOf(6, 9).associateWith { size ->
            if (!store.exists(size)) Slot("empty")
            else try {
                val record = store.load(size)
                Slot("saved", soloIsComplete(record.game), record.elapsedMillis, levelOf(record.game))
            } catch (_: Exception) { Slot("corrupt") }
        }
    }
    fun openSetup() {
        storageError = null
        inputError = null
        size = preferences.soloSize
        difficulty = preferences.soloDifficulty
        screen = SoloScreen.Setup
    }
    fun selectSize(newSize: Int) {
        size = newSize
        preferences.soloSize = (newSize)
        storageError = null
    }
    fun selectDifficulty(level: String) {
        if (level !in LEVELS) return
        difficulty = level
        preferences.soloDifficulty = (level)
    }
    fun prepareNew() {
        if (preparing) return
        if (store.exists(size)) {
            corruptReplacement = slots[size]?.status == "corrupt"
            replaceRequested = true
        } else newGame()
    }
    fun newGame() {
        replaceRequested = false
        if (preparing) return
        preparing = true
        val wanted = size
        val level = difficulty
        val seeds = List(3) { random.nextInt().toUInt() }
        // Digging a rated puzzle can take a moment on a phone: never on the main thread. A seed that cannot
        // reach the level is retried, as on the web.
        worker.execute {
            val next = seeds.firstNotNullOfOrNull { seed -> runCatching { newRatedSolo(wanted.toUByte(), seed, level) }.getOrNull() }
            main.post { finishNewGame(wanted, next) }
        }
    }
    private fun finishNewGame(wanted: Int, next: String?) {
        preparing = false
        val replacing = corruptReplacement
        corruptReplacement = false
        if (screen != SoloScreen.Setup || wanted != size) return
        try {
            requireNotNull(next)
            if (replacing && store.exists(size)) store.preserveCorrupt(size)
            game = next
            elapsedBase = 0
            elapsedDisplay = 0
            selected = firstBlank(next)
            noteMode = false
            runningSince = SystemClock.elapsedRealtime()
            storageError = null
            inputError = null
            hint = null
            hintMessage = null
            screen = SoloScreen.Game
            cue(Cue.Open)
            save()
        } catch (_: Exception) {
            storageError = UiText(R.string.solo_err_start)
        }
    }
    private fun firstBlank(saved: String): Int {
        val givens = JSONObject(saved).getJSONArray("givens")
        return (0 until givens.length()).firstOrNull { givens.getInt(it) == 0 } ?: 0
    }
    fun resumeSaved(slotSize: Int) {
        try {
            val record = store.load(slotSize)
            size = slotSize
            game = record.game
            elapsedBase = record.elapsedMillis
            elapsedDisplay = elapsedBase
            selected = firstBlank(record.game)
            noteMode = false
            storageError = null
            inputError = null
            hint = null
            hintMessage = null
            val complete = soloIsComplete(record.game)
            runningSince = null
            cue(Cue.Open)
            // A saved game reopens straight onto its board; a finished one onto its result.
            if (complete) screen = SoloScreen.Result else resume()
        } catch (_: Exception) {
            storageError = UiText(R.string.solo_err_open)
            refreshSlots()
        }
    }
    fun elapsed(): Long = elapsedBase + (runningSince?.let { SystemClock.elapsedRealtime() - it } ?: 0)
    fun tick() { elapsedDisplay = elapsed() }
    fun resume() {
        runningSince = SystemClock.elapsedRealtime()
        screen = SoloScreen.Game
    }
    fun pause() {
        if (screen != SoloScreen.Game) return
        elapsedBase = elapsed()
        elapsedDisplay = elapsedBase
        runningSince = null
        dismissNotices()
        screen = SoloScreen.Paused
        save()
    }
    fun save(): Boolean {
        val current = game ?: return true
        return try {
            store.save(size, SoloRecord(current, elapsed()))
            storageError = null
            slots = slots + (size to Slot("saved", screen == SoloScreen.Result, elapsed(), levelOf(current)))
            true
        } catch (_: Exception) {
            storageError = UiText(R.string.solo_err_save)
            false
        }
    }
    fun exit(): Boolean {
        if (screen == SoloScreen.Game) pause()
        if (!save()) return false
        screen = SoloScreen.Home
        game = null
        hint = null
        runningSince = null
        refreshSlots()
        return true
    }
    fun apply(type: String, value: Int? = null) {
        if (screen != SoloScreen.Game) return
        val current = game ?: return
        try {
            val command = JSONObject().put("type", type)
            if (type in listOf("set", "toggle_note", "erase")) command.put("index", selected)
            if (value != null) command.put("value", value)
            val next = updateSolo(current, command.toString())
            hint = null
            hintMessage = null
            game = next
            inputError = null
            if (soloIsComplete(next)) {
                elapsedBase = elapsed()
                elapsedDisplay = elapsedBase
                runningSince = null
                screen = SoloScreen.Result
                cue(Cue.Success)
            } else cue(when {
                type == "toggle_note" -> Cue.Type
                type == "set" && selected in conflicts() -> Cue.Error
                else -> Cue.Tap
            })
            save()
        } catch (_: Exception) {
            inputError = UiText(R.string.solo_err_input)
            cue(Cue.Error)
        }
    }
    fun conflicts(): Set<Int> = game?.let { saved -> runCatching { soloConflicts(saved).map { index -> index.toInt() }.toSet() }.getOrNull() } ?: emptySet()
    /** Technique hint: the eliminations the placement depends on, then the placement (shown over the keys). */
    fun requestHint() {
        val current = game ?: return
        inputError = null
        try {
            val result = soloTechniqueHint(current)
            hint = if (result == "null") null else JSONObject(result)
            hintMessage = if (hint == null) UiText(R.string.hint_none) else null
            hint?.let { selected = it.getInt("index"); cue(Cue.Ready) }
        } catch (_: Exception) {
            hint = null
            hintMessage = UiText(if (conflicts().isEmpty()) R.string.hint_unavailable else R.string.hint_conflicts)
        }
    }
    fun acceptHint() {
        val currentHint = hint ?: return
        selected = currentHint.getInt("index")
        apply("set", currentHint.getInt("value"))
    }
    fun dismissNotices() { hint = null; hintMessage = null; inputError = null }
    fun select(index: Int) { selected = index; dismissNotices() }
    fun toggleNotes() { noteMode = !noteMode; cue(Cue.Toggle) }
    fun enter(value: Int) = apply(if (noteMode) "toggle_note" else "set", value)
    fun move(rowDelta: Int, columnDelta: Int) {
        val row = (selected / size + rowDelta).coerceIn(0, size - 1)
        val column = (selected % size + columnDelta).coerceIn(0, size - 1)
        select(row * size + column)
    }
    fun close() = worker.shutdownNow()
}
