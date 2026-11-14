package com.sudokurivals

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import androidx.activity.compose.BackHandler
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
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import org.json.JSONObject

@Composable
internal fun LiveScreens(live: LiveState) {
    BackHandler { live.goHome() }
    LaunchedEffect(live) { while (true) { live.tick(); delay(250) } }
    val view = live.view
    val snapshot = live.snapshot()
    val lost = view.optString("connection") == "session_lost" && !view.optBoolean("voluntarily_left")
    Column(
        Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing).verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = live::goHome, modifier = Modifier.width(48.dp).testTag("back").semantics { contentDescription = "Geri" }, contentPadding = PaddingValues(4.dp)) { Text("‹", fontSize = 24.sp) }
            Text("Sudoku Rivals", Modifier.weight(1f).testTag("app-title").semantics { heading() }, style = MaterialTheme.typography.titleMedium)
        }
        if (live.storageError) {
            Text("Oturum bu cihaza kaydedilemedi. Açık durum ve önceki kayıt korunuyor. Uygulamayı kapatmadan tekrar kaydetmeyi dene.", Modifier.testTag("live-storage-status").semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error)
            OutlinedButton(onClick = live::retryStorage, modifier = Modifier.testTag("live-save-retry")) { Text("Tekrar kaydet") }
        }
        live.message?.let { Text(it, Modifier.testTag("live-message").semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error) }
        when {
            live.protectedSave -> {
                Text("Canlı oyun kaydı korunuyor", style = MaterialTheme.typography.titleLarge)
                Text("Açılamayan kaydın üzerine yazılmayacak. Kaydı ayrıca koruyup yeni bir oturum açabilirsin.")
                Button(onClick = live::requestReset, modifier = Modifier.testTag("live-recover")) { Text("Kaydı koruyup yeniden başla") }
            }
            snapshot == null || view.optBoolean("voluntarily_left") -> LiveSetup(live)
            snapshot.optString("phase") == "finished" && snapshot.optJSONObject("result") != null -> LiveResult(live, snapshot)
            lost || snapshot.optString("phase") == "closed" -> {
                Connection(live)
                Text("Maç sürdürülemiyor", Modifier.testTag("result-heading").semantics { heading() }, style = MaterialTheme.typography.titleLarge)
                Text(if (snapshot.optString("mode") == "race")
                    "Sunucu sonucu kurtaramadı. Galibiyet veya yenilgi ilan edilmedi. Yerel yarış verin korunuyor."
                else "Sunucu sonucu kurtaramadı. Galibiyet veya yenilgi ilan edilmedi. Düello sonucu sunucudan doğrulanamadı.", Modifier.testTag("result-details"))
                Button(onClick = live::requestReset, modifier = Modifier.testTag("live-recover")) { Text("Kaydı koruyup yeni oturum aç") }
                OutlinedButton(onClick = live::goHome, modifier = Modifier.testTag("home")) { Text("Ana sayfa") }
            }
            snapshot.optString("phase") == "finished" -> LiveResult(live, snapshot)
            snapshot.optString("phase") in listOf("lobby", "countdown") -> LiveLobby(live, snapshot)
            else -> LiveGame(live, snapshot)
        }
    }
    if (live.leaveConfirmation) AlertDialog(
        onDismissRequest = { live.leaveConfirmation = false }, title = { Text("Odadan ayrılmak istiyor musun?") },
        text = { Text(if (snapshot?.optString("phase") == "playing") "Ayrılırsan maçı kaybedersin. Sunucunun onayı gelene kadar oturum korunur." else "Odadan ayrılma isteği sunucuya gönderilecek. Onay gelene kadar oturum korunur.") },
        confirmButton = { TextButton(onClick = live::confirmLeave, modifier = Modifier.dialogTag("leave-confirm")) { Text("Ayrıl") } },
        dismissButton = { TextButton(onClick = { live.leaveConfirmation = false }) { Text("Vazgeç") } }
    )
    if (live.resetConfirmation) AlertDialog(
        onDismissRequest = { live.resetConfirmation = false }, title = { Text("Kaydı koruyup yeni oturuma geçilsin mi?") },
        text = { Text("Mevcut oturum kaydı cihazda ayrıca korunur. Yeni oturum eski odadaki koltuğu otomatik olarak geri alamaz.") },
        confirmButton = { TextButton(onClick = live::confirmReset, modifier = Modifier.dialogTag("live-reset-confirm")) { Text("Yeni oturum") } },
        dismissButton = { TextButton(onClick = { live.resetConfirmation = false }) { Text("Vazgeç") } }
    )
}

@Composable
private fun LiveSetup(live: LiveState) {
    val busy = live.view.optBoolean("pending_http")
    Text("Arkadaşınla yarış", style = MaterialTheme.typography.titleLarge)
    if (live.inviteReceived) Text("Bir davet alındı. Adını girip katıl.")
    if (BuildConfig.DEBUG) {
        var server by remember(live.origin) { mutableStateOf(live.origin) }
        OutlinedTextField(server, { server = it }, label = { Text("Deneme sunucusu (geliştirme)") }, modifier = Modifier.fillMaxWidth().testTag("live-server"), singleLine = true, enabled = !busy)
        TextButton(onClick = { live.configureServer(server) }, modifier = Modifier.testTag("live-server-apply"), enabled = !busy) { Text("Sunucuyu uygula") }
    } else if (!live.isConfigured()) Text("Canlı oyun sunucusu henüz yapılandırılmadı.")
    OutlinedTextField(live.name, { live.name = it }, label = { Text("Görünen ad (1–16 karakter)") }, modifier = Modifier.fillMaxWidth().testTag("guest-name"), singleLine = true, enabled = !busy)
    Row(Modifier.testTag("game-mode"), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        FilterChip(live.mode == "race", { live.mode = "race" }, label = { Text("Klasik yarış") }, enabled = !busy, modifier = Modifier.testTag("mode-race").heightIn(min = 48.dp))
        FilterChip(live.mode == "duel", { live.mode = "duel" }, label = { Text("90 sn düello") }, enabled = !busy, modifier = Modifier.testTag("mode-duel").heightIn(min = 48.dp))
    }
    Row(Modifier.testTag("board-size"), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (size in listOf(6, 9)) FilterChip(live.size == size, { live.size = size }, label = { Text("$size×$size") }, enabled = !busy, modifier = Modifier.testTag("board-size-$size").heightIn(min = 48.dp))
    }
    Button(onClick = live::create, enabled = !busy && live.isConfigured(), modifier = Modifier.fillMaxWidth().testTag("room-create")) { Text(if (busy) "Bekleniyor…" else "Oda kur") }
    OutlinedTextField(live.invite, { live.invite = it }, label = { Text("Oda kodu veya davet bağlantısı") }, modifier = Modifier.fillMaxWidth().testTag("room-code"), singleLine = true, enabled = !busy)
    OutlinedButton(onClick = live::join, enabled = !busy && live.isConfigured(), modifier = Modifier.fillMaxWidth().testTag("room-join")) { Text("Katıl") }
}

@Composable
private fun Connection(live: LiveState) {
    val connection = live.view.optString("connection")
    val text = when (connection) {
        "connected" -> "Bağlandı"
        "reconnecting" -> "Yeniden bağlanılıyor"
        "reconcile" -> "Yerel ve sunucu verileri farklı"
        "session_lost" -> "Oturuma dönülemiyor"
        else -> "Bağlantı kesildi; yeniden denenecek"
    }
    Text(text, Modifier.testTag("connection-status").semantics { liveRegion = LiveRegionMode.Polite }, color = if (connection == "connected") MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
}

@Composable
private fun LiveLobby(live: LiveState, snapshot: JSONObject) {
    val context = LocalContext.current
    val countdown = snapshot.getString("phase") == "countdown"
    val own = snapshot.getJSONObject("own")
    val opponent = snapshot.optJSONObject("opponent")
    val code = snapshot.getString("room_code")
    Text(if (countdown) "Maç başlıyor" else "Arkadaşın bekleniyor", style = MaterialTheme.typography.titleLarge)
    Text("${if (snapshot.getString("mode") == "race") "Klasik yarış" else "90 saniye düello"} · ${snapshot.getInt("size")}×${snapshot.getInt("size")}")
    Connection(live)
    Text(code, Modifier.testTag("room-code").semantics { contentDescription = "Oda kodu: $code" }, style = MaterialTheme.typography.headlineMedium)
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(onClick = {
            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("Sudoku Rivals oda kodu", code))
        }, modifier = Modifier.testTag("invite-copy")) { Text("Kodu kopyala") }
        OutlinedButton(onClick = {
            val invite = live.origin + "/r/" + code
            context.startActivity(Intent.createChooser(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, invite), "Daveti paylaş"))
        }, modifier = Modifier.testTag("invite-share")) { Text("Daveti paylaş") }
    }
    Column(Modifier.testTag("players")) {
        Text("${own.getString("name")} (sen) · ${if (own.getBoolean("ready")) "Hazır" else "Hazır değil"}")
        Text(if (opponent == null) "Arkadaşın henüz katılmadı" else "${opponent.getString("name")} · ${if (opponent.getBoolean("ready")) "Hazır" else "Hazır değil"}${if (!opponent.getBoolean("connected")) " · Bağlantısı kesildi" else ""}")
    }
    if (countdown) {
        val remaining = if (live.view.isNull("estimated_server_ms")) null else (snapshot.getLong("start_ms") - live.view.getLong("estimated_server_ms")).coerceAtLeast(0)
        Text(if (remaining == null) "Başlangıç zamanı eşitleniyor" else if (remaining == 0L) "Sunucu başlangıcı bekleniyor" else "${(remaining + 999) / 1000} saniye sonra başlayacak", Modifier.testTag("timer"), style = MaterialTheme.typography.titleLarge)
    }
    Button(onClick = { live.command("ready", !own.getBoolean("ready")) }, enabled = opponent != null && live.view.optString("connection") == "connected" && !live.view.optBoolean("pending_command"), modifier = Modifier.fillMaxWidth().testTag("ready-toggle")) { Text(if (own.getBoolean("ready")) "Hazır değilim" else "Hazırım") }
}

@Composable
private fun LiveResult(live: LiveState, snapshot: JSONObject) {
    val own = snapshot.getJSONObject("own")
    val opponent = snapshot.optJSONObject("opponent")
    val result = snapshot.optJSONObject("result")
    val reason = result?.optString("reason")
    val noContest = reason in listOf("both_disconnected", "resource_limit")
    val title = when {
        result == null || noContest -> "Maç sonuçsuz bitti"
        result.isNull("winner") -> "Berabere"
        result.getInt("winner") == own.getInt("seat") -> "Maçı kazandın"
        else -> "Rakibin kazandı"
    }
    Text(title, Modifier.testTag("result-heading").semantics { heading() }, style = MaterialTheme.typography.titleLarge)
    Connection(live)
    Text(if (snapshot.getString("mode") == "duel") "Puanın: ${own.getInt("score")} · Rakip: ${live.opponentScore()?.toString() ?: "Skor mevcut değil"}" else "Klasik yarış tamamlandı", Modifier.testTag("result-details"))
    val detail = when (reason) { "resigned" -> "Oyunculardan biri ayrıldı."; "disconnect_timeout" -> "Yeniden bağlanma süresi doldu."; "both_disconnected" -> "İki bağlantının süresi birlikte doldu."; "resource_limit" -> "Sunucu bu maçı güvenle sürdüremedi."; "time" -> "90 saniye doldu."; else -> "İlk doğru bitiş sunucuda doğrulandı." }
    Text(detail)
    if (opponent == null) Text("Arkadaşın odadan ayrıldı; rövanş istenemez. Yeni bir oda açabilirsin.", Modifier.testTag("rematch-status"))
    else if (!opponent.optBoolean("connected")) Text(if (own.optBoolean("rematch")) "Rövanş istedin; arkadaşının yeniden bağlanması bekleniyor." else "Arkadaşının bağlantısı kesildi. Rövanş isteyebilirsin; yeni maç için yeniden bağlanması gerekiyor.", Modifier.testTag("rematch-status"))
    else if (own.optBoolean("rematch")) Text("Rövanş istedin; arkadaşın bekleniyor.")
    else if (opponent?.optBoolean("rematch") == true) Text("Arkadaşın rövanş istiyor.")
    Button(onClick = { live.command("rematch") }, enabled = opponent != null && !live.view.optBoolean("voluntarily_left") && !live.view.optBoolean("pending_command") && live.view.optString("connection") == "connected" && !own.optBoolean("rematch"), modifier = Modifier.fillMaxWidth().testTag("rematch")) { Text("Rövanş iste") }
    if (live.view.optString("connection") != "connected") Text("Rövanş için sunucuya bağlantı gerekiyor.", Modifier.testTag("rematch-connection"))
    if (live.view.optString("connection") == "session_lost") {
        Button(onClick = live::requestReset, modifier = Modifier.testTag("live-recover")) { Text("Kaydı koruyup yeni oturum aç") }
    }
    OutlinedButton(onClick = live::goHome, modifier = Modifier.testTag("home")) { Text("Ana sayfa") }
}

@Composable
private fun LiveGame(live: LiveState, snapshot: JSONObject) {
    val raceMode = snapshot.getString("mode") == "race"
    val own = snapshot.getJSONObject("own")
    val race = live.view.optJSONObject("race")
    val cells = if (raceMode) race?.optJSONArray("cells") ?: return else own.getJSONArray("cells")
    val givens = own.getJSONArray("givens")
    val n = snapshot.getInt("size")
    val conflicts = if (raceMode) race?.optJSONArray("conflicts")?.let { list -> (0 until list.length()).map { list.getInt(it) }.toSet() }.orEmpty() else emptySet()
    val cfg = LocalConfiguration.current
    val columns = if (n == 6) { if (cfg.screenWidthDp >= 360) 6 else 3 } else 5
    Connection(live)
    val estimate = if (live.view.isNull("estimated_server_ms")) null else live.view.getLong("estimated_server_ms")
    val time = if (raceMode) estimate?.let { (it - snapshot.getLong("start_ms")).coerceAtLeast(0) } else if (live.view.isNull("remaining_ms")) null else live.view.getLong("remaining_ms")
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text("${if (raceMode) "Klasik yarış" else "Düello"} · $n×$n")
        Text(time?.let { "%02d:%02d".format(it / 60000, (it / 1000) % 60) } ?: "Eşitleniyor", Modifier.testTag("timer"))
    }
    val opponent = snapshot.optJSONObject("opponent")
    if (raceMode) Text("Rakip: %${opponent?.optInt("progress_percent") ?: 0} dolu hücre", Modifier.testTag("opponent-progress"))
    else {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text("Sen: ${own.getInt("score")}", Modifier.testTag("own-score"))
            Text("Rakip: ${live.opponentScore()?.toString() ?: "Skor mevcut değil"}", Modifier.testTag("opponent-score"))
            Text("Seri: ${own.getInt("combo")}", Modifier.testTag("combo"))
        }
    }
    if (live.view.optBoolean("reconcile_required")) {
        Text("Sunucu tahtası yerel çalışmandan farklı. Yerel not ve geçmişin seçim yapana kadar korunuyor.", Modifier.testTag("reconcile-message").semantics { liveRegion = LiveRegionMode.Polite })
        OutlinedButton(onClick = { live.reconcile(true) }, modifier = Modifier.testTag("reconcile-local")) { Text("Yerel çalışmamı koru") }
        OutlinedButton(onClick = { live.reconcile(false) }, modifier = Modifier.testTag("reconcile-server")) { Text("Sunucu tahtasını kullan") }
    }
    val lock = live.view.optLong("locked_remaining_ms", 0)
    val receipt = live.view.optJSONObject("last_receipt")?.takeIf { it.optString("match_id") == snapshot.getString("match_id") }
    val status = when {
        !raceMode && lock > 0 -> "Giriş kilidi: ${(lock + 999) / 1000} saniye"
        !raceMode && time == 0L -> "Süre doldu; sunucunun sonucu bekleniyor"
        live.view.optBoolean("pending_command") -> "Sunucu onayı bekleniyor"
        receipt?.optString("code") == "wrong" -> "Yanlış hamle; puan ve seri sunucuda güncellendi"
        receipt?.optString("code") == "not_correct" -> "Çözüm henüz doğru değil"
        conflicts.isNotEmpty() -> "Tahtada çakışma var"
        else -> "Oyun devam ediyor"
    }
    Text(status, Modifier.testTag(if (lock > 0) "input-lock" else "game-status").semantics { liveRegion = LiveRegionMode.Polite }, color = if (lock > 0 || conflicts.isNotEmpty()) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface, style = MaterialTheme.typography.bodyMedium)
    val scale = LocalDensity.current.fontScale
    BoxWithConstraints(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        val width = minOf(maxWidth, (cfg.screenHeightDp - if (columns == 6) 390 else 450).coerceIn(180, 420).dp)
        val symbolSize = (minOf(24f, width.value / n * 0.55f) / scale).sp
        Column(Modifier.size(width).testTag("game-board").semantics { collectionInfo = CollectionInfo(n, n) }) {
            for (row in 0 until n) Row(Modifier.weight(1f)) {
                for (column in 0 until n) {
                    val index = row * n + column
                    val value = cells.getInt(index)
                    val given = givens.getInt(index) != 0
                    val notes = race?.optJSONArray("notes")?.optJSONArray(index)?.let { list -> (0 until list.length()).map { list.getInt(it) } }.orEmpty()
                    val line = MaterialTheme.colorScheme.onSurface
                    val selected = live.selected == index
                    Box(Modifier.weight(1f).fillMaxHeight().background(if (selected) MaterialTheme.colorScheme.primaryContainer else Color.Transparent)
                        .drawBehind {
                            drawLine(line, Offset.Zero, Offset(size.width, 0f), if (row % (if (n == 6) 2 else 3) == 0) 2.dp.toPx() else 0.5.dp.toPx())
                            drawLine(line, Offset.Zero, Offset(0f, size.height), if (column % 3 == 0) 2.dp.toPx() else 0.5.dp.toPx())
                            if (row == n - 1) drawLine(line, Offset(0f, size.height), Offset(size.width, size.height), 2.dp.toPx())
                            if (column == n - 1) drawLine(line, Offset(size.width, 0f), Offset(size.width, size.height), 2.dp.toPx())
                        }.clickable { live.selected = index }.testTag("cell-${row + 1}-${column + 1}")
                        .clearAndSetSemantics {
                            contentDescription = "Satır ${row + 1}, sütun ${column + 1}, ${if (given) "verilen" else if (!raceMode && value != 0) "kilitli" else "girilebilir"}, ${if (value == 0) "boş" else value.toString()}, notlar: ${notes.joinToString(", ").ifEmpty { "yok" }}${if (index in conflicts) ", çakışma" else ""}"
                            this.selected = selected
                            collectionItemInfo = CollectionItemInfo(row, 1, column, 1)
                            onClick("Hücreyi seç") { live.selected = index; true }
                        }, contentAlignment = Alignment.Center) {
                        if (value != 0) Text(value.toString() + if (index in conflicts) "!" else "", fontSize = symbolSize, fontWeight = if (given) FontWeight.Bold else FontWeight.Normal, color = if (index in conflicts) MaterialTheme.colorScheme.error else if (given) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.primary)
                        else if (notes.isNotEmpty()) Text("·", fontSize = symbolSize, color = MaterialTheme.colorScheme.primary)
                    }
                }
            }
        }
    }
    val selectedValue = cells.getInt(live.selected)
    val selectedNotes = race?.optJSONArray("notes")?.optJSONArray(live.selected)?.let { list -> (0 until list.length()).map { list.getInt(it) } }.orEmpty()
    Text("${live.selected / n + 1}. satır, ${live.selected % n + 1}. sütun · ${if (selectedValue == 0) "Boş" else "Değer: $selectedValue"}${if (selectedNotes.isEmpty()) "" else " · Notlar: ${selectedNotes.joinToString(", ")}"}", style = MaterialTheme.typography.bodyMedium)
    val enabled = live.canInput() && givens.getInt(live.selected) == 0 && (raceMode || selectedValue == 0)
    for (digits in (1..n).chunked(columns)) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (digit in digits) OutlinedButton(onClick = { live.enter(digit) }, enabled = enabled, modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("number-$digit"), contentPadding = PaddingValues(4.dp)) { Text(digit.toString()) }
    }
    if (raceMode && race != null) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        FilterChip(live.noteMode, { live.noteMode = !live.noteMode }, label = { Text("✎", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }, modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("notes-toggle").semantics { contentDescription = "Kalem" })
        OutlinedButton(onClick = { live.edit("erase") }, enabled = enabled, modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("erase"), contentPadding = PaddingValues(4.dp)) { Text("Sil") }
        OutlinedButton(onClick = { live.edit("undo") }, enabled = race.optBoolean("can_undo") && live.canInput(), modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("undo").semantics { contentDescription = "Geri al" }, contentPadding = PaddingValues(4.dp)) { Text("↶", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
        OutlinedButton(onClick = { live.edit("redo") }, enabled = race.optBoolean("can_redo") && live.canInput(), modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("redo").semantics { contentDescription = "Yinele" }, contentPadding = PaddingValues(4.dp)) { Text("↷", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
    }
}

/** Dialogs are separate windows; the root's testTagsAsResourceId does not reach them. */
@OptIn(ExperimentalComposeUiApi::class)
private fun Modifier.dialogTag(tag: String) = semantics { testTagsAsResourceId = true }.testTag(tag)
