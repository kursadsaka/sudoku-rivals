package com.sudokurivals

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.AtomicFile
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URI
import java.security.SecureRandom
import java.util.concurrent.Executors
import uniffi.sudoku_ffi.MultiplayerClient

/** The opaque Rust save contains credentials. It never enters the presentation model. */
private class LiveStore(directory: File) {
    private val file = AtomicFile(File(directory, "live-session.json"))
    private val directory = directory
    fun exists() = file.baseFile.exists() || File(directory, "live-session.json.bak").exists()
    fun load(): Pair<String, String> {
        val bytes = file.openRead().use { input ->
            val out = ByteArrayOutputStream()
            val buffer = ByteArray(8192)
            while (true) {
                val count = input.read(buffer)
                if (count < 0) break
                require(out.size() + count <= 2_200_000)
                out.write(buffer, 0, count)
            }
            out.toByteArray()
        }
        val record = JSONObject(String(bytes, Charsets.UTF_8))
        require(record.keys().asSequence().toSet() == setOf("version", "origin", "client"))
        require(record.get("version") == 1 && record.get("origin") is String && record.get("client") is String)
        return record.getString("origin") to record.getString("client")
    }
    fun save(origin: String, opaque: String) {
        val bytes = JSONObject().put("version", 1).put("origin", origin).put("client", opaque)
            .toString().toByteArray(Charsets.UTF_8)
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
    fun preserve() {
        if (!exists()) return
        val backup = AtomicFile(File(directory, "live-session-recovery-${System.currentTimeMillis()}.json"))
        var stream: FileOutputStream? = null
        try {
            stream = backup.startWrite()
            file.openRead().use { it.copyTo(stream) }
            stream.fd.sync()
            backup.finishWrite(stream)
        } catch (error: Exception) {
            backup.failWrite(stream)
            throw error
        }
    }
}

/** HTTP/storage adapter only. The Rust object owns game, sequence, retry and clock state. */
internal class LiveState(directory: File) {
    private val store = LiveStore(directory)
    // Presentation-only cache: no protocol or opaque Rust-save changes.
    private val scoreFile = AtomicFile(File(directory, "live-result-score.json"))
    private var savedResultScore = runCatching {
        val bytes = scoreFile.openRead().use { input -> ByteArray(1024).let { buffer ->
            val count = input.read(buffer)
            require(count > 0 && input.read() == -1)
            String(buffer, 0, count, Charsets.UTF_8)
        } }
        JSONObject(bytes).let { it.getString("match_id") to it.getLong("score").also { score -> require(score >= 0) } }
    }.getOrNull()
    private val random = SecureRandom()
    private fun secureGuest() = ByteArray(32).also { random.nextBytes(it) }.joinToString("") { "%02x".format(it.toInt() and 255) }
    private var guest = secureGuest()
    private var client = MultiplayerClient(random.nextInt().toUInt())
    private val main = Handler(Looper.getMainLooper())
    private val io = Executors.newFixedThreadPool(2)
    private var foreground = false
    @Volatile private var destroyed = false
    @Volatile private var transportEpoch = 0L
    @Volatile private var activeId: Long? = null
    private val transportLock = Any()
    @Volatile private var connection: HttpURLConnection? = null
    private var resumeRequired = false
    private var deferredRequest: JSONObject? = null
    private var pendingInvite: String? = null
    private var pendingOrigin: String? = null
    private var leaving = false

    var origin by mutableStateOf(if (BuildConfig.DEBUG) "http://10.0.2.2:3000" else BuildConfig.MATCH_SERVER_URL)
        private set
    var onHome: () -> Unit = {}
    var visible by mutableStateOf(false)
        private set
    var view by mutableStateOf(JSONObject())
        private set
    var name by mutableStateOf("Misafir")
    var mode by mutableStateOf("race")
    var size by mutableStateOf(6)
    var invite by mutableStateOf("")
    var selected by mutableStateOf(0)
    var noteMode by mutableStateOf(false)
    var message by mutableStateOf<String?>(null)
        private set
    var storageError by mutableStateOf(false)
        private set
    var protectedSave by mutableStateOf(false)
        private set
    var leaveConfirmation by mutableStateOf(false)
    var resetConfirmation by mutableStateOf(false)
    var inviteReceived by mutableStateOf(false)
        private set

    init {
        if (store.exists()) {
            try {
                val (savedOrigin, opaque) = store.load()
                origin = normalizeOrigin(savedOrigin)
                val restored = MultiplayerClient.restore(opaque)
                client.close()
                client = restored
                resumeRequired = true
            } catch (_: Exception) {
                protectedSave = true
                message = "Canlı oyun kaydı açılamadı. Kayıt korunuyor; üzerine yazılmayacak."
            }
        }
        refresh()
        leaving = view.optBoolean("pending_leave")
        visible = view.optBoolean("pending_http") || (snapshot() != null && !view.optBoolean("voluntarily_left"))
        resumeRequired = visible && !view.optBoolean("pending_http") && view.optString("connection") != "session_lost"
    }

    private fun now() = SystemClock.elapsedRealtime().toULong()
    private fun refresh() {
        view = JSONObject(client.view(now()))
        val s = snapshot() ?: return
        if (s.optString("phase") != "finished" || s.optString("mode") != "duel" || s.optJSONObject("result") == null) return
        val score = (s.optJSONObject("opponent")?.opt("score") as? Number)?.toLong() ?: return
        val next = s.getString("match_id") to score
        if (next == savedResultScore) return
        var stream: FileOutputStream? = null
        try {
            stream = scoreFile.startWrite()
            stream.write(JSONObject().put("match_id", next.first).put("score", next.second).toString().toByteArray(Charsets.UTF_8))
            scoreFile.finishWrite(stream)
        } catch (_: Exception) {
            scoreFile.failWrite(stream)
        }
        savedResultScore = next
    }
    fun opponentScore(): Long? {
        val s = snapshot() ?: return null
        val current = (s.optJSONObject("opponent")?.opt("score") as? Number)?.toLong()
        if (current != null) return current
        return savedResultScore?.takeIf {
            s.optString("phase") == "finished" && s.optString("mode") == "duel" && it.first == s.optString("match_id")
        }?.second
    }
    fun snapshot(): JSONObject? = view.optJSONObject("snapshot")
    fun isConfigured() = runCatching { normalizeOrigin(origin) }.isSuccess
    fun open() { visible = true; foreground = true; tick() }
    fun openInvite(raw: String) {
        pendingInvite = raw
        invite = ""
        inviteReceived = true
        visible = true
        foreground = true
        message = when {
            view.optString("connection") == "session_lost" -> "Bu oturuma dönülemiyor. Yeni davete katılmak için kaydı koruyup yeni oturum aç."
            snapshot() != null && !view.optBoolean("voluntarily_left") -> "Yeni davete katılmadan önce mevcut odadan ayrılmalısın."
            else -> null
        }
        tick()
    }
    fun setForeground(active: Boolean) {
        foreground = active && visible
        if (!active) {
            if (snapshot() != null && !view.optBoolean("voluntarily_left") && view.optString("connection") != "session_lost") resumeRequired = true
            cancelTransport()
            if (view.optBoolean("pending_http") || (!view.optBoolean("voluntarily_left") && snapshot() != null)) dispatch(JSONObject().put("type", "network_failed"), quiet = true)
            persist()
        } else if (visible) {
            if (!resumeRequired && !storageError) {
                val deferred = deferredRequest
                deferredRequest = null
                if (deferred != null) send(deferred)
            }
            tick()
        }
    }
    fun close() {
        foreground = false
        persist()
        destroyed = true
        cancelTransport()
        io.shutdownNow()
        client.close()
    }
    private fun cancelTransport() {
        synchronized(transportLock) {
            transportEpoch++
            activeId = null
            connection?.disconnect()
            connection = null
        }
    }
    fun goHome() {
        if (snapshot() != null && !view.optBoolean("voluntarily_left") && view.optString("connection") != "session_lost") {
            leaveConfirmation = true
            return
        }
        visible = false
        foreground = false
        cancelTransport()
        if (view.optBoolean("pending_http")) dispatch(JSONObject().put("type", "network_failed"), quiet = true)
        persist()
        onHome()
    }
    fun confirmLeave() {
        leaveConfirmation = false
        leaving = true
        message = "Odadan ayrılma onayı bekleniyor."
        dispatch(JSONObject().put("type", "leave"))
    }
    fun configureServer(raw: String) {
        if (!BuildConfig.DEBUG) return
        val next = try { normalizeOrigin(raw) } catch (_: Exception) {
            message = "Geçerli bir HTTPS adresi veya izinli yerel geliştirme adresi gir."
            return
        }
        if (next == origin) return
        if (view.optBoolean("pending_http")) { message = "Önce bekleyen isteğin tamamlanmasını bekle."; return }
        if (snapshot() != null || protectedSave) {
            pendingOrigin = next
            resetConfirmation = true
        } else {
            origin = next
            persist()
        }
    }
    fun requestReset() { pendingOrigin = null; resetConfirmation = true }
    fun confirmReset() {
        try {
            store.preserve()
            cancelTransport()
            val fresh = MultiplayerClient(random.nextInt().toUInt())
            client.close()
            client = fresh
            guest = secureGuest()
            origin = pendingOrigin ?: if (isConfigured()) origin else if (BuildConfig.DEBUG) "http://10.0.2.2:3000" else BuildConfig.MATCH_SERVER_URL
            protectedSave = false
            resumeRequired = false
            deferredRequest = null
            leaving = false
            selected = 0
            noteMode = false
            message = null
            pendingOrigin = null
            resetConfirmation = false
            refresh()
            persist()
        } catch (_: Exception) {
            message = "Eski oturum kaydı korunamadı. Yeni oturuma geçilmedi; tekrar deneyebilirsin."
        }
    }
    fun create() {
        if (!isConfigured()) { message = "Canlı oyun sunucusu henüz yapılandırılmadı."; return }
        if (dispatch(JSONObject().put("type", "create").put("guest", guest).put("name", name).put("mode", mode).put("size", size))) resumeRequired = false
    }
    fun join() {
        if (!isConfigured()) { message = "Canlı oyun sunucusu henüz yapılandırılmadı."; return }
        val raw = if (invite.isNotBlank()) invite else pendingInvite.orEmpty()
        if (dispatch(JSONObject().put("type", "join").put("guest", guest).put("invite", raw).put("name", name))) resumeRequired = false
    }
    fun command(type: String, ready: Boolean? = null) {
        val command = JSONObject().put("type", type)
        if (ready != null) command.put("ready", ready)
        dispatch(JSONObject().put("type", "command").put("command", command))
    }
    fun edit(type: String, value: Int? = null) {
        val command = JSONObject().put("type", type)
        if (type in listOf("set", "toggle_note", "erase")) command.put("index", selected)
        if (value != null) command.put("value", value)
        dispatch(JSONObject().put("type", "edit").put("command", command))
    }
    fun enter(value: Int) {
        val s = snapshot() ?: return
        if (!canInput()) return
        if (s.getString("mode") == "race") edit(if (noteMode) "toggle_note" else "set", value)
        else {
            val command = JSONObject().put("type", "move").put("board_id", s.getJSONObject("own").getLong("board_id"))
                .put("index", selected).put("value", value)
            dispatch(JSONObject().put("type", "command").put("command", command))
        }
    }
    fun canInput(): Boolean {
        val s = snapshot() ?: return false
        if (s.optString("phase") != "playing" || view.optBoolean("reconcile_required") || view.optString("connection") == "session_lost") return false
        if (s.optString("mode") == "race") return view.optJSONObject("race") != null
        return view.optString("connection") == "connected" && !view.optBoolean("pending_command") && view.optLong("locked_remaining_ms", 1) == 0L && view.optLong("remaining_ms", 0) > 0
    }
    fun moveSelection(rows: Int, columns: Int) {
        val n = snapshot()?.optInt("size") ?: return
        selected = (selected / n + rows).coerceIn(0, n - 1) * n + (selected % n + columns).coerceIn(0, n - 1)
    }
    fun reconcile(keepLocal: Boolean) { dispatch(JSONObject().put("type", "reconcile").put("keep_local", keepLocal)) }
    fun retryStorage() {
        if (protectedSave) return
        if (persist()) {
            val request = deferredRequest
            deferredRequest = null
            if (resumeRequired && snapshot() != null) tick()
            else if (request != null) send(request)
        }
    }
    private fun persist(): Boolean {
        if (protectedSave) return false
        return try {
            store.save(origin, client.save())
            storageError = false
            true
        } catch (_: Exception) {
            storageError = true
            false
        }
    }
    private fun dispatch(action: JSONObject, quiet: Boolean = false): Boolean {
        if (destroyed || protectedSave) return false
        return try {
            val old = snapshot()
            val effect = JSONObject(client.dispatch(action.toString(), now()))
            if (!quiet) message = null
            val saved = if (effect.optBoolean("changed")) persist() else !storageError
            refresh()
            val s = snapshot()
            if (s != null && (old == null || old.optString("match_id") != s.optString("match_id") || old.optJSONObject("own")?.optInt("board_id") != s.getJSONObject("own").optInt("board_id"))) {
                selected = 0
                noteMode = false
            }
            val request = effect.optJSONObject("request")
            if (request != null) {
                if (saved) send(request) else deferredRequest = request
            }
            if (leaving && action.optString("type") == "receive" && view.optBoolean("voluntarily_left")) {
                leaving = false
                visible = false
                foreground = false
                cancelTransport()
                onHome()
            }
            true
        } catch (_: Exception) {
            if (!quiet) message = "İşlem uygulanamadı. Adı, oda kodunu ve bağlantı durumunu kontrol et; bekleyen istek varsa tamamlanmasını bekle."
            false
        }
    }
    private fun send(request: JSONObject) {
        val id = request.getLong("id")
        if (activeId == id || destroyed) return
        if (!foreground) { deferredRequest = request; return }
        val path = request.getString("path")
        val method = request.getString("method")
        if (method != "POST" || !path.startsWith("/v1/") || path.contains('?') || path.contains('#')) {
            message = "İstek güvenle gönderilemedi."
            return
        }
        val base = try { normalizeOrigin(origin) } catch (_: Exception) { message = "Sunucu adresi geçersiz."; return }
        cancelTransport()
        activeId = id
        val epoch = transportEpoch
        val body = request.getString("body") // Opaque exact body: no adapter-generated counters or auth.
        val timeout = Runnable {
            if (!destroyed && transportEpoch == epoch && activeId == id) {
                connection?.disconnect()
                activeId = null
                connection = null
                message = "Sunucu yanıtı zaman aşımına uğradı; yeniden bağlanılacak."
                dispatch(JSONObject().put("type", "network_failed").put("request_id", id), quiet = true)
            }
        }
        main.postDelayed(timeout, 10_000)
        io.execute {
            var http: HttpURLConnection? = null
            try {
                val opened = URI(base + path).toURL().openConnection() as HttpURLConnection
                http = opened
                synchronized(transportLock) {
                    if (destroyed || transportEpoch != epoch || activeId != id) { opened.disconnect(); return@execute }
                    connection = opened
                }
                opened.requestMethod = method
                opened.instanceFollowRedirects = false
                opened.connectTimeout = 4000
                opened.readTimeout = 4000
                opened.useCaches = false
                opened.doOutput = true
                opened.setRequestProperty("Content-Type", "application/json")
                opened.setRequestProperty("Cache-Control", "no-store")
                opened.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
                val status = opened.responseCode
                val input = if (status in 200..299) opened.inputStream else opened.errorStream
                val output = ByteArrayOutputStream()
                input?.use { stream ->
                    val buffer = ByteArray(8192)
                    while (true) {
                        val count = stream.read(buffer)
                        if (count < 0) break
                        require(output.size() + count <= 131_072)
                        output.write(buffer, 0, count)
                    }
                }
                val response = output.toString("UTF-8")
                main.post {
                    main.removeCallbacks(timeout)
                    if (!destroyed && transportEpoch == epoch && activeId == id) {
                        activeId = null
                        connection = null
                        val accepted = dispatch(JSONObject().put("type", "receive").put("request_id", id).put("status", status).put("response", response), quiet = true)
                        if (!accepted) {
                            message = "Sunucudan geçerli yanıt alınamadı; yeniden bağlanılacak."
                            dispatch(JSONObject().put("type", "network_failed").put("request_id", id), quiet = true)
                        } else if (status !in 200..299) {
                            message = safeServerMessage(response)
                        } else message = null
                    }
                }
            } catch (_: Exception) {
                main.post {
                    main.removeCallbacks(timeout)
                    if (!destroyed && transportEpoch == epoch && activeId == id) {
                        activeId = null
                        connection = null
                        message = when {
                            snapshot()?.optString("phase") != "playing" -> "Sunucuya ulaşılamıyor. Yeniden bağlanılacak."
                            snapshot()?.optString("mode") == "race" -> "Sunucuya ulaşılamıyor. Yerel yarış değişikliklerin korunuyor."
                            snapshot()?.optString("mode") == "duel" -> "Sunucuya ulaşılamıyor. Yeniden bağlanılacak; düello hamleleri ve sonuç sunucu onayı bekliyor."
                            else -> "Sunucuya ulaşılamıyor. Yeniden bağlanılacak."
                        }
                        dispatch(JSONObject().put("type", "network_failed").put("request_id", id), quiet = true)
                    }
                }
            } finally { http?.disconnect() }
        }
    }
    fun tick() {
        if (!foreground || destroyed || protectedSave || !visible) return
        refresh()
        if (!view.optBoolean("pending_http") && (view.optBoolean("voluntarily_left") || view.optString("connection") == "session_lost")) return
        val time = SystemClock.elapsedRealtime()
        val retryAt = if (view.isNull("retry_at_ms")) null else view.optLong("retry_at_ms")
        if (activeId != null || deferredRequest != null || storageError) return
        if (resumeRequired && !leaving && snapshot() != null && view.optString("connection") != "session_lost") {
            if (retryAt == null || time >= retryAt) {
                if (dispatch(JSONObject().put("type", "resume"), quiet = true)) resumeRequired = false
            }
            return
        }
        if (retryAt != null && time >= retryAt) {
            if (!dispatch(JSONObject().put("type", "retry"), quiet = true) && snapshot() != null)
                dispatch(JSONObject().put("type", "resume"), quiet = true)
            return
        }
        if (view.optBoolean("pending_http")) return
        if (view.optString("connection") == "reconcile") {
            if (view.optBoolean("poll_due")) dispatch(JSONObject().put("type", "poll"), quiet = true)
            return
        }
        if (view.optString("connection") != "connected") return
        if (view.optJSONObject("race")?.optBoolean("dirty") == true && snapshot()?.optString("phase") == "playing" && !view.optBoolean("pending_command") && !view.optBoolean("reconcile_required"))
            dispatch(JSONObject().put("type", "sync"), quiet = true)
        else if (view.optBoolean("poll_due")) dispatch(JSONObject().put("type", "poll"), quiet = true)
    }
    private fun normalizeOrigin(raw: String): String {
        val uri = URI(raw.trim())
        require(uri.userInfo == null && uri.query == null && uri.fragment == null && (uri.path.isNullOrEmpty() || uri.path == "/") && !uri.host.isNullOrEmpty())
        require(uri.scheme == "https" || (BuildConfig.DEBUG && uri.scheme == "http" && uri.host in listOf("localhost", "127.0.0.1", "10.0.2.2")))
        require(uri.port == -1 || uri.port in 1..65535)
        return URI(uri.scheme, null, uri.host, uri.port, null, null, null).toString()
    }
    private fun safeServerMessage(raw: String): String = when (runCatching { JSONObject(raw).optString("error") }.getOrNull()) {
        "room_full" -> "Oda dolu. Başka bir davet kullanabilirsin."
        "room_missing", "room_closed" -> "Oda kapandı veya sunucu yeniden başladı. Bu oturumla odaya dönülemiyor."
        "unauthorized" -> "Bu oturumla odaya dönülemiyor. Yerel kayıt korunuyor."
        "rate_limited", "capacity" -> "Sunucu şu anda yeni işlem kabul etmiyor. Biraz bekleyip tekrar dene."
        "stale_generation", "stale_match", "sequence_gap", "sequence_conflict" -> "Oturum eşitleniyor. Yerel değişikliklerin korunuyor."
        else -> "Sunucu işlemi kabul etmedi. Bağlantı ve oda durumunu kontrol et."
    }
}
