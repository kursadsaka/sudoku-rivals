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
    val backLabel = res(R.string.action_back)
    val gameScreen = !live.protectedSave && snapshot != null && !view.optBoolean("voluntarily_left") && !lost &&
        snapshot.optString("phase") !in listOf("finished", "closed", "lobby", "countdown")
    if (gameScreen && snapshot != null) {
        Box(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)) { LiveGame(live, snapshot) }
    } else Column(
        Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing).verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = live::goHome, modifier = Modifier.width(48.dp).testTag("back").semantics { contentDescription = backLabel }, contentPadding = PaddingValues(4.dp)) { Text("‹", Modifier.clearAndSetSemantics { }, fontSize = 24.sp) }
            Text(res(R.string.app_title), Modifier.weight(1f).testTag("app-title").semantics { heading() }, style = MaterialTheme.typography.titleMedium)
        }
        if (live.storageError) {
            Text(res(R.string.live_storage_error), Modifier.testTag("live-storage-status").semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error)
            OutlinedButton(onClick = live::retryStorage, modifier = Modifier.testTag("live-save-retry")) { Text(res(R.string.action_retry_save)) }
        }
        live.message?.let { Text(it.asString(), Modifier.testTag("live-message").semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error) }
        when {
            live.protectedSave -> {
                Text(res(R.string.live_protected_title), style = MaterialTheme.typography.titleLarge)
                Text(res(R.string.live_protected_body))
                Button(onClick = live::requestReset, modifier = Modifier.testTag("live-recover")) { Text(res(R.string.live_protected_action)) }
            }
            snapshot == null || view.optBoolean("voluntarily_left") -> LiveSetup(live)
            snapshot.optString("phase") == "finished" && snapshot.optJSONObject("result") != null -> LiveResult(live, snapshot)
            lost || snapshot.optString("phase") == "closed" -> {
                Connection(live)
                Text(res(R.string.live_lost_title), Modifier.testTag("result-heading").semantics { heading() }, style = MaterialTheme.typography.titleLarge)
                Text(res(if (snapshot.optString("mode") == "race") R.string.live_lost_race else R.string.live_lost_duel), Modifier.testTag("result-details"))
                Button(onClick = live::requestReset, modifier = Modifier.testTag("live-recover")) { Text(res(R.string.live_recover)) }
                OutlinedButton(onClick = live::goHome, modifier = Modifier.testTag("home")) { Text(res(R.string.action_home)) }
            }
            snapshot.optString("phase") == "finished" -> LiveResult(live, snapshot)
            snapshot.optString("phase") in listOf("lobby", "countdown") -> LiveLobby(live, snapshot)
            else -> Unit
        }
    }
    if (live.leaveConfirmation) AlertDialog(
        onDismissRequest = { live.leaveConfirmation = false }, title = { Text(res(R.string.leave_title)) },
        text = { Text(res(if (snapshot?.optString("phase") == "playing") R.string.leave_body_playing else R.string.leave_body)) },
        confirmButton = { TextButton(onClick = live::confirmLeave, modifier = Modifier.dialogTag("leave-confirm")) { Text(res(R.string.action_leave)) } },
        dismissButton = { TextButton(onClick = { live.leaveConfirmation = false }) { Text(res(R.string.action_cancel)) } }
    )
    if (live.resetConfirmation) AlertDialog(
        onDismissRequest = { live.resetConfirmation = false }, title = { Text(res(R.string.reset_title)) },
        text = { Text(res(R.string.reset_body)) },
        confirmButton = { TextButton(onClick = live::confirmReset, modifier = Modifier.dialogTag("live-reset-confirm")) { Text(res(R.string.action_new_session)) } },
        dismissButton = { TextButton(onClick = { live.resetConfirmation = false }) { Text(res(R.string.action_cancel)) } }
    )
}

@Composable
private fun LiveSetup(live: LiveState) {
    val busy = live.view.optBoolean("pending_http")
    Text(res(R.string.home_friend_start), style = MaterialTheme.typography.titleLarge)
    if (live.inviteReceived) Text(res(R.string.live_invite_received))
    if (BuildConfig.DEBUG) {
        var server by remember(live.origin) { mutableStateOf(live.origin) }
        OutlinedTextField(server, { server = it }, label = { Text(res(R.string.live_server_debug_label)) }, modifier = Modifier.fillMaxWidth().testTag("live-server"), singleLine = true, enabled = !busy)
        TextButton(onClick = { live.configureServer(server) }, modifier = Modifier.testTag("live-server-apply"), enabled = !busy) { Text(res(R.string.live_server_apply)) }
    } else if (BuildConfig.STAGING) Text(res(R.string.live_server_fixed, BuildConfig.MATCH_SERVER_URL.removePrefix("https://")), Modifier.testTag("live-server-fixed"))
    else if (!live.isConfigured()) Text(res(R.string.live_not_configured))
    OutlinedTextField(live.name, { live.name = it }, label = { Text(res(R.string.live_name_label)) }, modifier = Modifier.fillMaxWidth().testTag("guest-name"), singleLine = true, enabled = !busy)
    Row(Modifier.testTag("game-mode"), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        FilterChip(live.mode == "race", { live.mode = "race" }, label = { Text(res(R.string.mode_race)) }, enabled = !busy, modifier = Modifier.testTag("mode-race").heightIn(min = 48.dp))
        FilterChip(live.mode == "duel", { live.mode = "duel" }, label = { Text(res(R.string.mode_duel_chip)) }, enabled = !busy, modifier = Modifier.testTag("mode-duel").heightIn(min = 48.dp))
    }
    Row(Modifier.testTag("board-size"), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (size in listOf(6, 9)) FilterChip(live.size == size, { live.size = size }, label = { Text("$size×$size") }, enabled = !busy, modifier = Modifier.testTag("board-size-$size").heightIn(min = 48.dp))
    }
    Button(onClick = live::create, enabled = !busy && live.isConfigured(), modifier = Modifier.fillMaxWidth().testTag("room-create")) { Text(res(if (busy) R.string.live_waiting else R.string.live_create)) }
    OutlinedTextField(live.invite, { live.invite = it }, label = { Text(res(R.string.live_invite_label)) }, modifier = Modifier.fillMaxWidth().testTag("room-code"), singleLine = true, enabled = !busy)
    OutlinedButton(onClick = live::join, enabled = !busy && live.isConfigured(), modifier = Modifier.fillMaxWidth().testTag("room-join")) { Text(res(R.string.live_join)) }
}

@Composable
private fun Connection(live: LiveState) {
    val connection = live.view.optString("connection")
    val text = res(when (connection) {
        "connected" -> R.string.conn_connected
        "reconnecting" -> R.string.conn_reconnecting
        "reconcile" -> R.string.conn_reconcile
        "session_lost" -> R.string.conn_lost
        else -> R.string.conn_disconnected
    })
    Text(text, Modifier.testTag("connection-status").semantics { liveRegion = LiveRegionMode.Polite }, color = if (connection == "connected") MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
}

@Composable
private fun LiveLobby(live: LiveState, snapshot: JSONObject) {
    val context = LocalContext.current
    val countdown = snapshot.getString("phase") == "countdown"
    val own = snapshot.getJSONObject("own")
    val opponent = snapshot.optJSONObject("opponent")
    val code = snapshot.getString("room_code")
    val roomCodeDescription = res(R.string.room_code_desc, code)
    Text(res(if (countdown) R.string.lobby_starting else R.string.lobby_waiting), style = MaterialTheme.typography.titleLarge)
    Text(res(R.string.lobby_mode_size, res(if (snapshot.getString("mode") == "race") R.string.mode_race else R.string.mode_duel_long), snapshot.getInt("size")))
    Connection(live)
    Text(code, Modifier.testTag("room-code").semantics { contentDescription = roomCodeDescription }, style = MaterialTheme.typography.headlineMedium)
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(onClick = {
            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText(context.getString(R.string.clip_label), code))
        }, modifier = Modifier.testTag("invite-copy")) { Text(res(R.string.invite_copy)) }
        OutlinedButton(onClick = {
            val invite = live.origin + "/r/" + code
            context.startActivity(Intent.createChooser(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, invite), context.getString(R.string.invite_share)))
        }, modifier = Modifier.testTag("invite-share")) { Text(res(R.string.invite_share)) }
    }
    Column(Modifier.testTag("players")) {
        Text(res(R.string.lobby_you, own.getString("name"), res(if (own.getBoolean("ready")) R.string.player_ready else R.string.player_not_ready)))
        Text(if (opponent == null) res(R.string.lobby_friend_missing) else res(if (opponent.getBoolean("connected")) R.string.lobby_friend else R.string.lobby_friend_disconnected, opponent.getString("name"), res(if (opponent.getBoolean("ready")) R.string.player_ready else R.string.player_not_ready)))
    }
    if (countdown) {
        val remaining = if (live.view.isNull("estimated_server_ms")) null else (snapshot.getLong("start_ms") - live.view.getLong("estimated_server_ms")).coerceAtLeast(0)
        Text(if (remaining == null) res(R.string.countdown_sync) else if (remaining == 0L) res(R.string.countdown_wait) else plural(R.plurals.countdown_seconds, ((remaining + 999) / 1000).toInt(), ((remaining + 999) / 1000).toInt()), Modifier.testTag("timer"), style = MaterialTheme.typography.titleLarge)
    }
    Button(onClick = { live.command("ready", !own.getBoolean("ready")) }, enabled = opponent != null && live.view.optString("connection") == "connected" && !live.view.optBoolean("pending_command"), modifier = Modifier.fillMaxWidth().testTag("ready-toggle")) { Text(res(if (own.getBoolean("ready")) R.string.not_ready_action else R.string.ready_action)) }
}

@Composable
private fun LiveResult(live: LiveState, snapshot: JSONObject) {
    val own = snapshot.getJSONObject("own")
    val opponent = snapshot.optJSONObject("opponent")
    val result = snapshot.optJSONObject("result")
    val reason = result?.optString("reason")
    val noContest = reason in listOf("both_disconnected", "resource_limit")
    val title = res(when {
        result == null || noContest -> R.string.result_no_contest
        result.isNull("winner") -> R.string.result_draw
        result.getInt("winner") == own.getInt("seat") -> R.string.result_win
        else -> R.string.result_loss
    })
    Text(title, Modifier.testTag("result-heading").semantics { heading() }, style = MaterialTheme.typography.titleLarge)
    Connection(live)
    Text(if (snapshot.getString("mode") == "duel") res(R.string.result_duel_details, own.getInt("score"), live.opponentScore()?.toString() ?: res(R.string.score_unavailable)) else res(R.string.result_race_done), Modifier.testTag("result-details"))
    val detail = res(when (reason) {
        "resigned" -> R.string.reason_resigned
        "disconnect_timeout" -> R.string.reason_disconnect_timeout
        "both_disconnected" -> R.string.reason_both_disconnected
        "resource_limit" -> R.string.reason_resource_limit
        "time" -> R.string.reason_time
        else -> R.string.reason_default
    })
    Text(detail)
    if (opponent == null) Text(res(R.string.rematch_no_opponent), Modifier.testTag("rematch-status"))
    else if (!opponent.optBoolean("connected")) Text(res(if (own.optBoolean("rematch")) R.string.rematch_requested_disconnected else R.string.rematch_disconnected), Modifier.testTag("rematch-status"))
    else if (own.optBoolean("rematch")) Text(res(R.string.rematch_waiting))
    else if (opponent?.optBoolean("rematch") == true) Text(res(R.string.rematch_friend_wants))
    Button(onClick = { live.command("rematch") }, enabled = opponent != null && !live.view.optBoolean("voluntarily_left") && !live.view.optBoolean("pending_command") && live.view.optString("connection") == "connected" && !own.optBoolean("rematch"), modifier = Modifier.fillMaxWidth().testTag("rematch")) { Text(res(R.string.rematch_action)) }
    if (live.view.optString("connection") != "connected") Text(res(R.string.rematch_needs_connection), Modifier.testTag("rematch-connection"))
    if (live.view.optString("connection") == "session_lost") {
        Button(onClick = live::requestReset, modifier = Modifier.testTag("live-recover")) { Text(res(R.string.live_recover)) }
    }
    OutlinedButton(onClick = live::goHome, modifier = Modifier.testTag("home")) { Text(res(R.string.action_home)) }
}

@Composable
private fun LiveGame(live: LiveState, snapshot: JSONObject) {
    val raceMode = snapshot.getString("mode") == "race"
    val own = snapshot.getJSONObject("own")
    val race = live.view.optJSONObject("race")
    val cells: org.json.JSONArray? = if (raceMode) race?.optJSONArray("cells") else own.getJSONArray("cells")
    val givens = own.getJSONArray("givens")
    val n = snapshot.getInt("size")
    val conflicts = if (raceMode) race?.optJSONArray("conflicts")?.let { list -> (0 until list.length()).map { list.getInt(it) }.toSet() }.orEmpty() else emptySet()
    val connection = live.view.optString("connection")
    val estimate = if (live.view.isNull("estimated_server_ms")) null else live.view.getLong("estimated_server_ms")
    val time = if (raceMode) estimate?.let { (it - snapshot.getLong("start_ms")).coerceAtLeast(0) } else if (live.view.isNull("remaining_ms")) null else live.view.getLong("remaining_ms")
    val timeText = time?.let { "%02d:%02d".format(it / 60000, (it / 1000) % 60) } ?: res(R.string.time_syncing)
    val opponent = snapshot.optJSONObject("opponent")
    val lock = live.view.optLong("locked_remaining_ms", 0)
    val receipt = live.view.optJSONObject("last_receipt")?.takeIf { it.optString("match_id") == snapshot.getString("match_id") }
    fun noteList(index: Int) = race?.optJSONArray("notes")?.optJSONArray(index)?.let { list -> (0 until list.length()).map { list.getInt(it) } }.orEmpty()
    val selectedValue = cells?.getInt(live.selected) ?: 0
    val enabled = cells != null && live.canInput() && givens.getInt(live.selected) == 0 && (raceMode || selectedValue == 0)
    val context = LocalContext.current
    LocalConfiguration.current
    val notesDescription = res(R.string.cd_notes)
    val undoDescription = res(R.string.cd_undo)
    val redoDescription = res(R.string.cd_redo)
    GameScreenLayout(
        top = {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                if (UiDebug.enabled) Text(res(R.string.app_title), Modifier.testTag("app-title"), style = MaterialTheme.typography.labelMedium)
                GameNavBar(
                    modeLabel = res(R.string.lobby_mode_size, res(if (raceMode) R.string.mode_race else R.string.mode_duel_short), n), timeText = timeText,
                    timeDescription = res(R.string.live_time_desc, timeText), backDescription = res(R.string.action_back), onBack = live::goHome
                ) {
                    val name = opponent?.optString("name").orEmpty()
                    val nameDescription = res(R.string.opponent_name_desc, name)
                    if (name.isNotEmpty()) Text(name, Modifier.widthIn(max = 96.dp).semantics { contentDescription = nameDescription }, maxLines = 1, overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis, style = MaterialTheme.typography.bodyMedium)
                }
                // One status row: connection dot (text only when unhealthy) · mode value · transient state.
                Row(Modifier.fillMaxWidth().heightIn(min = 24.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    val healthy = connection == "connected"
                    val connectionText = res(when (connection) {
                        "connected" -> R.string.conn_connected
                        "reconnecting" -> R.string.conn_reconnecting
                        "reconcile" -> R.string.conn_reconcile_short
                        "session_lost" -> R.string.conn_lost
                        else -> R.string.conn_disconnected_short
                    })
                    if (healthy) StatusDot(MaterialTheme.colorScheme.primary, connectionText, "connection-status")
                    else Text(connectionText, Modifier.testTag("connection-status").semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                    if (raceMode) {
                        val progress = opponent?.optInt("progress_percent") ?: 0
                        val progressDescription = res(R.string.race_progress_desc, progress)
                        Text(res(R.string.race_progress, progress), Modifier.testTag("opponent-progress").semantics { contentDescription = progressDescription }, style = MaterialTheme.typography.bodyMedium)
                    } else {
                        val rival = live.opponentScore()
                        val youDescription = res(R.string.duel_you_desc, own.getInt("score"))
                        val rivalDescription = res(R.string.duel_rival_desc, rival?.toString() ?: res(R.string.duel_rival_unknown_desc))
                        val streakDescription = res(R.string.duel_streak_desc, own.getInt("combo"))
                        Text(res(R.string.duel_you, own.getInt("score")), Modifier.testTag("own-score").semantics { contentDescription = youDescription }, style = MaterialTheme.typography.bodyMedium)
                        Text(res(R.string.duel_rival, rival?.toString() ?: "–"), Modifier.testTag("opponent-score").semantics { contentDescription = rivalDescription }, style = MaterialTheme.typography.bodyMedium)
                        Text(res(R.string.duel_streak, own.getInt("combo")), Modifier.testTag("combo").semantics { contentDescription = streakDescription }, style = MaterialTheme.typography.bodyMedium)
                    }
                    Spacer(Modifier.weight(1f))
                    val lockSeconds = ((lock + 999) / 1000).toInt()
                    val (short, long) = when {
                        !raceMode && lock > 0 -> res(R.string.status_lock, lockSeconds) to plural(R.plurals.lock_seconds_desc, lockSeconds, lockSeconds)
                        !raceMode && time == 0L -> res(R.string.status_time_up) to res(R.string.status_time_up_desc)
                        live.view.optBoolean("pending_command") -> res(R.string.status_pending) to res(R.string.status_pending_desc)
                        conflicts.isNotEmpty() -> res(R.string.status_conflict) to res(R.string.status_conflict_desc)
                        UiDebug.enabled -> res(R.string.status_in_progress) to res(R.string.status_in_progress)
                        else -> null to null
                    }
                    if (short != null) Text(short, Modifier.testTag(if (lock > 0 && !raceMode) "input-lock" else "game-status").semantics { contentDescription = long!!; liveRegion = LiveRegionMode.Polite },
                        color = if ((lock > 0 && !raceMode) || conflicts.isNotEmpty()) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface, style = MaterialTheme.typography.bodyMedium)
                }
            }
        },
        board = { side ->
            if (cells != null) SudokuBoard(
                n = n, side = side, selected = live.selected, conflicts = conflicts,
                value = { cells.getInt(it) }, given = { givens.getInt(it) != 0 }, notes = ::noteList,
                label = { index ->
                    val value = cells.getInt(index)
                    cellLabel(context, index, n, if (givens.getInt(index) != 0) R.string.cell_given else if (!raceMode && value != 0) R.string.cell_locked else R.string.cell_editable,
                        value, noteList(index), index in conflicts, ", ")
                },
                onSelect = { live.selected = it }
            ) else Spacer(Modifier.size(side))
        },
        bottom = {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (UiDebug.enabled && cells != null) {
                    val selectedNotes = noteList(live.selected)
                    Text(res(R.string.selected_cell_line, live.selected / n + 1, live.selected % n + 1,
                        (if (selectedValue == 0) res(R.string.selected_cell_empty) else res(R.string.selected_cell_value, selectedValue)) +
                            if (selectedNotes.isEmpty()) "" else res(R.string.selected_cell_notes_suffix, selectedNotes.joinToString(", "))),
                        Modifier.testTag("selected-cell"), style = MaterialTheme.typography.bodyMedium)
                }
                DigitPad(n, enabled) { live.enter(it) }
                if (raceMode && race != null) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    FilterChip(live.noteMode, { live.noteMode = !live.noteMode }, label = { Text("✎", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }, modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("notes-toggle").semantics { contentDescription = notesDescription })
                    OutlinedButton(onClick = { live.edit("erase") }, enabled = enabled, modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("erase"), contentPadding = PaddingValues(4.dp)) { Text(res(R.string.action_erase)) }
                    OutlinedButton(onClick = { live.edit("undo") }, enabled = race.optBoolean("can_undo") && live.canInput(), modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("undo").semantics { contentDescription = undoDescription }, contentPadding = PaddingValues(4.dp)) { Text("↶", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
                    OutlinedButton(onClick = { live.edit("redo") }, enabled = race.optBoolean("can_redo") && live.canInput(), modifier = Modifier.weight(1f).heightIn(min = 48.dp).testTag("redo").semantics { contentDescription = redoDescription }, contentPadding = PaddingValues(4.dp)) { Text("↷", Modifier.clearAndSetSemantics { }, fontSize = 20.sp) }
                }
                // Transient notices only.
                if (live.storageError) {
                    Text(res(R.string.live_storage_error), Modifier.testTag("live-storage-status").semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
                    OutlinedButton(onClick = live::retryStorage, modifier = Modifier.heightIn(min = 48.dp).testTag("live-save-retry")) { Text(res(R.string.action_retry_save)) }
                }
                live.message?.let { Text(it.asString(), Modifier.testTag("live-message").semantics { liveRegion = LiveRegionMode.Polite }, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium) }
                if (live.view.optBoolean("reconcile_required")) {
                    Text(res(R.string.reconcile_message), Modifier.testTag("reconcile-message").semantics { liveRegion = LiveRegionMode.Polite }, style = MaterialTheme.typography.bodyMedium)
                    OutlinedButton(onClick = { live.reconcile(true) }, modifier = Modifier.heightIn(min = 48.dp).testTag("reconcile-local")) { Text(res(R.string.reconcile_keep_local)) }
                    OutlinedButton(onClick = { live.reconcile(false) }, modifier = Modifier.heightIn(min = 48.dp).testTag("reconcile-server")) { Text(res(R.string.reconcile_use_server)) }
                }
                val notice = when (receipt?.optString("code")) {
                    "wrong" -> res(R.string.receipt_wrong)
                    "not_correct" -> res(R.string.receipt_not_correct)
                    else -> null
                }
                notice?.let { Text(it, Modifier.semantics { liveRegion = LiveRegionMode.Polite }, style = MaterialTheme.typography.bodyMedium) }
            }
        }
    )
}

/** Dialogs are separate windows; the root's testTagsAsResourceId does not reach them. */
@OptIn(ExperimentalComposeUiApi::class)
private fun Modifier.dialogTag(tag: String) = semantics { testTagsAsResourceId = true }.testTag(tag)
