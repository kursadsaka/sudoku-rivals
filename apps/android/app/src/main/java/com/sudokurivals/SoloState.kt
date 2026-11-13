package com.sudokurivals

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
import uniffi.sudoku_ffi.newGeneratedSolo
import uniffi.sudoku_ffi.soloConflicts
import uniffi.sudoku_ffi.soloHint
import uniffi.sudoku_ffi.soloIsComplete
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

internal class SoloState(directory: File) {
    private val store = SoloStore(directory)
    private val random = SecureRandom()
    var screen by mutableStateOf(SoloScreen.Home)
    var size by mutableStateOf(6)
    var selected by mutableStateOf(0)
    var noteMode by mutableStateOf(false)
    var game by mutableStateOf<String?>(null)
        private set
    var elapsedDisplay by mutableStateOf(0L)
        private set
    var storageError by mutableStateOf<String?>(null)
        private set
    var inputError by mutableStateOf<String?>(null)
        private set
    var slotStatus by mutableStateOf(mapOf<Int, String>())
        private set
    var hint by mutableStateOf<JSONObject?>(null)
    var hintMessage by mutableStateOf<String?>(null)
    var replaceRequested by mutableStateOf(false)
    var corruptReplacement by mutableStateOf(false)
    private var elapsedBase = 0L
    private var runningSince: Long? = null

    init { refreshSlots() }

    fun refreshSlots() {
        slotStatus = listOf(6, 9).associateWith { size ->
            if (!store.exists(size)) "empty"
            else try { store.load(size); "saved" } catch (_: Exception) { "corrupt" }
        }
    }
    fun openSetup() {
        storageError = null
        inputError = null
        screen = SoloScreen.Setup
    }
    fun selectSize(newSize: Int) {
        size = newSize
        storageError = null
    }
    fun prepareNew() {
        if (store.exists(size)) {
            corruptReplacement = slotStatus[size] == "corrupt"
            replaceRequested = true
        } else newGame()
    }
    fun newGame() {
        try {
            val next = (0 until 8).firstNotNullOfOrNull {
                runCatching { newGeneratedSolo(size.toUByte(), random.nextInt().toUInt(), (if (size == 6) 18 else 43).toUByte()) }.getOrNull()
            } ?: error("Puzzle generation failed")
            if (corruptReplacement && store.exists(size)) store.preserveCorrupt(size)
            game = next
            elapsedBase = 0
            elapsedDisplay = 0
            selected = 0
            noteMode = false
            runningSince = SystemClock.elapsedRealtime()
            storageError = null
            inputError = null
            hint = null
            hintMessage = null
            screen = SoloScreen.Game
            save()
        } catch (_: Exception) {
            storageError = "Oyun başlatılamadı. Önceki kayıt korunuyor; tekrar deneyebilirsin."
        }
        replaceRequested = false
        corruptReplacement = false
    }
    fun resumeSaved() {
        try {
            val record = store.load(size)
            game = record.game
            elapsedBase = record.elapsedMillis
            elapsedDisplay = elapsedBase
            selected = 0
            noteMode = false
            storageError = null
            inputError = null
            val complete = soloIsComplete(record.game)
            screen = if (complete) SoloScreen.Result else SoloScreen.Paused
            runningSince = null
        } catch (_: Exception) {
            storageError = "Kayıt açılamadı. Kayıt silinmedi. Yeni oyun başlatırsan önceki kayıt ayrıca korunacak."
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
        hint = null
        screen = SoloScreen.Paused
        save()
    }
    fun save(): Boolean {
        val current = game ?: return true
        return try {
            store.save(size, SoloRecord(current, elapsed()))
            storageError = null
            slotStatus = slotStatus + (size to "saved")
            true
        } catch (_: Exception) {
            storageError = "Oyun kaydedilemedi. Açık oyunun ve önceki kayıt korunuyor; tekrar deneyebilirsin."
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
            }
            save()
        } catch (_: Exception) {
            inputError = "Bu hücrede bu işlem yapılamıyor. Verilen hücreler değiştirilemez; kalem için boş hücre seç."
        }
    }
    fun conflicts(): Set<Int> = game?.let { soloConflicts(it).map { index -> index.toInt() }.toSet() } ?: emptySet()
    fun requestHint() {
        try {
            val result = soloHint(game ?: return)
            hint = if (result == "null") null else JSONObject(result)
            hintMessage = if (hint == null) "Bu konumda tekli tekniklerle açıklanabilir bir ipucu bulunamadı." else null
        } catch (_: Exception) {
            hint = null
            hintMessage = "Önce tahtadaki çakışmaları veya hatalı girişleri düzelt. Bu durumda güvenilir bir ipucu verilemiyor."
        }
    }
    fun acceptHint() {
        val currentHint = hint ?: return
        selected = currentHint.getInt("index")
        apply("set", currentHint.getInt("value"))
    }
    fun enter(value: Int) = apply(if (noteMode) "toggle_note" else "set", value)
    fun move(rowDelta: Int, columnDelta: Int) {
        val row = (selected / size + rowDelta).coerceIn(0, size - 1)
        val column = (selected % size + columnDelta).coerceIn(0, size - 1)
        selected = row * size + column
    }
}
