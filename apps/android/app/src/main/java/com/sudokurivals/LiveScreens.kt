package com.sudokurivals

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
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
    val gameScreen = !live.protectedSave && snapshot != null && !view.optBoolean("voluntarily_left") && !lost &&
        snapshot.optString("phase") !in listOf("finished", "closed", "lobby", "countdown")
    if (gameScreen && snapshot != null) {
        Box(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)) { LiveGame(live, snapshot) }
    } else Column(
        Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing).verticalScroll(rememberScrollState()).padding(horizontal = 20.dp).padding(bottom = 28.dp),
    ) {
        Spacer(Modifier.height(10.dp))
        val inMatch = snapshot != null && !view.optBoolean("voluntarily_left") && !lost && snapshot.optString("phase") != "finished"
        BackLink(res(if (inMatch) R.string.action_leave else R.string.action_back), live::goHome)
        val messages: @Composable () -> Unit = {
            if (live.storageError) {
                Notice(res(R.string.live_storage_error), Modifier.padding(bottom = 12.dp), error = true, tag = "live-storage-status") {
                    TextButton(onClick = live::retryStorage, modifier = Modifier.testTag("live-save-retry"), contentPadding = PaddingValues(0.dp)) { Text(res(R.string.action_retry_save), color = ink.error, fontWeight = FontWeight.SemiBold) }
                }
            }
            live.message?.let { Notice(it.asString(), Modifier.padding(bottom = 12.dp), error = true, tag = "live-message") }
        }
        when {
            live.protectedSave -> {
                Eyebrow(res(R.string.home_friend_start)); Spacer(Modifier.height(10.dp))
                PageTitle(res(R.string.live_protected_title)); Spacer(Modifier.height(10.dp))
                Lede(res(R.string.live_protected_body)); Spacer(Modifier.height(16.dp))
                messages()
                PrimaryButton(res(R.string.live_protected_action), live::requestReset, Modifier.fillMaxWidth().testTag("live-recover"))
            }
            snapshot == null || view.optBoolean("voluntarily_left") -> LiveSetup(live, messages)
            snapshot.optString("phase") == "finished" && snapshot.optJSONObject("result") != null -> LiveResult(live, snapshot, messages)
            lost || snapshot.optString("phase") == "closed" -> {
                Eyebrow(matchLabel(snapshot)); Spacer(Modifier.height(10.dp))
                PageTitle(res(R.string.live_lost_title), Modifier.testTag("result-heading")); Spacer(Modifier.height(10.dp))
                Connection(live); Spacer(Modifier.height(12.dp))
                messages()
                Text(res(if (snapshot.optString("mode") == "race") R.string.live_lost_race else R.string.live_lost_duel), Modifier.testTag("result-details"), style = Type.body, color = ink.muted)
                Spacer(Modifier.height(26.dp))
                PrimaryButton(res(R.string.live_recover), live::requestReset, Modifier.fillMaxWidth().testTag("live-recover"))
                Spacer(Modifier.height(10.dp))
                SecondaryButton(res(R.string.action_home), live::goHome, Modifier.fillMaxWidth().testTag("home"))
            }
            snapshot.optString("phase") == "finished" -> LiveResult(live, snapshot, messages)
            snapshot.optString("phase") in listOf("lobby", "countdown") -> LiveLobby(live, snapshot, messages)
            else -> Unit
        }
    }
    if (live.leaveConfirmation) AlertDialog(
        onDismissRequest = { live.leaveConfirmation = false }, title = { Text(res(R.string.leave_title)) },
        text = { Text(res(if (snapshot?.optString("phase") == "playing") R.string.leave_body_playing else R.string.leave_body)) },
        confirmButton = { TextButton(onClick = live::confirmLeave, modifier = Modifier.dialogTag("leave-confirm")) { Text(res(R.string.action_leave)) } },
        dismissButton = { TextButton(onClick = { live.leaveConfirmation = false }) { Text(res(R.string.action_stay)) } },
        containerColor = ink.background, shape = RoundedCornerShape(24.dp),
    )
    if (live.resetConfirmation) AlertDialog(
        onDismissRequest = { live.resetConfirmation = false }, title = { Text(res(R.string.reset_title)) },
        text = { Text(res(R.string.reset_body)) },
        confirmButton = { TextButton(onClick = live::confirmReset, modifier = Modifier.dialogTag("live-reset-confirm")) { Text(res(R.string.action_new_session)) } },
        dismissButton = { TextButton(onClick = { live.resetConfirmation = false }) { Text(res(R.string.action_cancel)) } },
        containerColor = ink.background, shape = RoundedCornerShape(24.dp),
    )
}

/** `Classic race · 9×9 · Easy`; rooms without a level (older clients) show none. */
@Composable
private fun matchLabel(snapshot: JSONObject, short: Boolean = false): String {
    val race = snapshot.optString("mode") == "race"
    val mode = res(if (race) (if (short) R.string.mode_race_short else R.string.mode_race) else if (short) R.string.mode_duel_short else R.string.mode_duel_long)
    val level = snapshot.optString("difficulty").takeIf { it in LEVELS }?.let { levelName(it) }
    return listOfNotNull(res(R.string.lobby_mode_size, mode, snapshot.optInt("size")), level).joinToString(" · ")
}

@Composable
private fun FieldLabel(text: String) { Text(text, color = ink.muted, fontWeight = FontWeight.Medium, fontSize = 14.sp, modifier = Modifier.padding(bottom = 8.dp)) }

@Composable
private fun paperFieldColors() = OutlinedTextFieldDefaults.colors(
    focusedContainerColor = ink.background, unfocusedContainerColor = ink.background, disabledContainerColor = ink.background,
    focusedBorderColor = ink.accent, unfocusedBorderColor = ink.border,
)

@Composable
private fun LiveSetup(live: LiveState, messages: @Composable () -> Unit) {
    val p = ink
    val busy = live.view.optBoolean("pending_http")
    val join = live.tab == "join"
    val invited = join && live.inviteReceived
    Eyebrow(res(R.string.home_friend_start))
    Spacer(Modifier.height(10.dp))
    PageTitle(res(if (invited) R.string.live_invited_title else if (join) R.string.live_join_title else R.string.live_setup_title))
    Spacer(Modifier.height(10.dp))
    Lede(res(if (invited) R.string.live_invite_received else if (join) R.string.live_join_intro else R.string.live_setup_intro))
    Spacer(Modifier.height(16.dp))
    Segmented(listOf("create" to res(R.string.live_create), "join" to res(R.string.live_join)), live.tab, { live.tab = it }, Modifier.fillMaxWidth(), tagPrefix = "tab-", enabled = !busy)
    Spacer(Modifier.height(18.dp))
    messages()
    val nameField: @Composable () -> Unit = {
        FieldLabel(res(R.string.live_name_label))
        OutlinedTextField(live.name, { live.name = it.take(32) }, modifier = Modifier.fillMaxWidth().testTag("guest-name"), singleLine = true, enabled = !busy,
            shape = RoundedCornerShape(14.dp), colors = paperFieldColors(), keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words))
        Spacer(Modifier.height(16.dp))
    }
    if (join) {
        FieldLabel(res(R.string.live_invite_label))
        OutlinedTextField(live.invite, { live.invite = it }, modifier = Modifier.fillMaxWidth().testTag("room-code"), singleLine = true, enabled = !busy,
            shape = RoundedCornerShape(14.dp), colors = paperFieldColors(), placeholder = { Text(res(R.string.live_code_placeholder), color = p.border) },
            textStyle = LocalTextStyle.current.copy(fontSize = 22.sp, fontWeight = FontWeight.Bold, letterSpacing = 3.sp, textAlign = TextAlign.Center),
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters, autoCorrectEnabled = false))
        Spacer(Modifier.height(16.dp))
        nameField()
        PrimaryButton(res(if (busy) R.string.live_waiting else R.string.live_join), live::join, Modifier.fillMaxWidth().testTag("room-join"), enabled = !busy && live.isConfigured())
    } else {
        nameField()
        FieldLabel(res(R.string.live_mode_label))
        Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min).testTag("game-mode"), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            for ((mode, title, help) in listOf(Triple("race", R.string.mode_race, R.string.mode_race_help), Triple("duel", R.string.mode_duel_long, R.string.mode_duel_help))) {
                val on = live.mode == mode
                val shape = RoundedCornerShape(18.dp)
                Box(
                    Modifier.weight(1f).fillMaxHeight().clip(shape).background(if (on) p.selected else p.background).border(1.5.dp, if (on) p.accent else p.line, shape)
                        .selectable(on, enabled = !busy, role = Role.RadioButton) { live.chooseMode(mode) }.testTag("mode-$mode").padding(14.dp)
                ) {
                    Column(Modifier.padding(end = 20.dp)) {
                        Text(res(title), fontWeight = FontWeight.Bold, fontSize = 17.sp, lineHeight = 21.sp)
                        Spacer(Modifier.height(4.dp))
                        Text(res(help), color = p.muted, fontSize = 13.sp, lineHeight = 17.sp)
                    }
                    if (on) Box(Modifier.align(Alignment.TopEnd).size(22.dp).clip(CircleShape).background(p.brand), contentAlignment = Alignment.Center) {
                        Icon(Glyph.check, null, Modifier.size(14.dp), tint = p.onBrand)
                    }
                }
            }
        }
        Spacer(Modifier.height(16.dp))
        FieldLabel(res(R.string.board_size))
        Segmented(listOf(6, 9).map { it.toString() to res(R.string.size_label, it) }, live.size.toString(), { live.chooseSize(it.toInt()) }, Modifier.fillMaxWidth().testTag("board-size"), tagPrefix = "board-size-", enabled = !busy)
        Spacer(Modifier.height(16.dp))
        FieldLabel(res(R.string.difficulty))
        Segmented(levelOptions(), live.difficulty, live::chooseDifficulty, Modifier.fillMaxWidth().testTag("difficulty"), tagPrefix = "difficulty-", enabled = !busy)
        Spacer(Modifier.height(10.dp))
        Text(res(levelHelp(live.difficulty)) + " " + res(R.string.level_scope), style = Type.small, color = p.muted)
        Spacer(Modifier.height(20.dp))
        PrimaryButton(res(if (busy) R.string.live_waiting else R.string.live_create), live::create, Modifier.fillMaxWidth().testTag("room-create"), enabled = !busy && live.isConfigured())
    }
    Spacer(Modifier.height(16.dp))
    if (BuildConfig.DEBUG) {
        var server by remember(live.origin) { mutableStateOf(live.origin) }
        OutlinedTextField(server, { server = it }, label = { Text(res(R.string.live_server_debug_label)) }, modifier = Modifier.fillMaxWidth().testTag("live-server"), singleLine = true, enabled = !busy, shape = RoundedCornerShape(14.dp), colors = paperFieldColors())
        QuietButton(res(R.string.live_server_apply), { live.configureServer(server) }, Modifier.testTag("live-server-apply"))
    } else if (BuildConfig.STAGING) Text(res(R.string.live_server_fixed, BuildConfig.MATCH_SERVER_URL.removePrefix("https://")), Modifier.testTag("live-server-fixed"), style = Type.small, color = p.muted)
    else if (!live.isConfigured()) Notice(res(R.string.live_not_configured))
}

private fun levelHelp(level: String) = when (level) {
    "beginner" -> R.string.level_beginner_help; "easy" -> R.string.level_easy_help; "medium" -> R.string.level_medium_help; "hard" -> R.string.level_hard_help; else -> R.string.level_expert_help
}

/** Connection line (web `.connection-strip`): a green dot when connected, the state in words otherwise. */
@Composable
private fun Connection(live: LiveState) {
    val p = ink
    val connection = live.view.optString("connection")
    val text = res(when (connection) {
        "connected" -> R.string.conn_connected
        "reconnecting" -> R.string.conn_reconnecting
        "reconcile" -> R.string.conn_reconcile
        "session_lost" -> R.string.conn_lost
        else -> R.string.conn_disconnected
    })
    val healthy = connection == "connected"
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(8.dp).clip(CircleShape).background(if (healthy) p.ok else p.error))
        Spacer(Modifier.width(8.dp))
        Text(text, Modifier.testTag("connection-status").semantics { liveRegion = LiveRegionMode.Polite }, color = if (healthy) p.muted else p.error, fontSize = 13.sp)
    }
}

@Composable
private fun LiveLobby(live: LiveState, snapshot: JSONObject, messages: @Composable () -> Unit) {
    val p = ink
    val context = LocalContext.current
    val countdown = snapshot.getString("phase") == "countdown"
    val own = snapshot.getJSONObject("own")
    val opponent = snapshot.optJSONObject("opponent")
    val code = snapshot.getString("room_code")
    val roomCodeDescription = res(R.string.room_code_desc, code)
    Eyebrow(matchLabel(snapshot))
    Spacer(Modifier.height(10.dp))
    PageTitle(res(if (countdown) R.string.lobby_starting else R.string.lobby_waiting))
    Spacer(Modifier.height(10.dp))
    Connection(live)
    Spacer(Modifier.height(16.dp))
    messages()
    snapshot.optString("difficulty").takeIf { it in LEVELS }?.let { level ->
        Text(res(R.string.room_level, levelName(level)) + " " + res(levelHelp(level)) + " " + res(R.string.level_scope),
            Modifier.fillMaxWidth().clip(RoundedCornerShape(14.dp)).background(p.selected).padding(horizontal = 12.dp, vertical = 10.dp).testTag("room-difficulty"),
            style = Type.small, color = p.foreground)
        Spacer(Modifier.height(14.dp))
    }
    Column(Modifier.fillMaxWidth().card(p, 22.dp).padding(20.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        SectionLabel(res(R.string.room_code_label))
        Spacer(Modifier.height(12.dp))
        Row(Modifier.testTag("room-code").clearAndSetSemantics { contentDescription = roomCodeDescription; text = AnnotatedString(code) }, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            for (letter in code) Box(Modifier.size(width = 38.dp, height = 46.dp).clip(RoundedCornerShape(10.dp)).background(p.canvas).border(1.dp, p.line, RoundedCornerShape(10.dp)), contentAlignment = Alignment.Center) {
                Text(letter.toString(), fontSize = 24.sp, fontWeight = FontWeight.Bold)
            }
        }
        Spacer(Modifier.height(14.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SecondaryButton(res(R.string.invite_copy), {
                val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
                clipboard.setPrimaryClip(ClipData.newPlainText(context.getString(R.string.clip_label), code))
            }, Modifier.weight(1f).testTag("invite-copy"), icon = Glyph.copy)
            SecondaryButton(res(R.string.invite_share), {
                val invite = live.origin + "/r/" + code
                context.startActivity(Intent.createChooser(Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, invite), context.getString(R.string.invite_share)))
            }, Modifier.weight(1f).testTag("invite-share"), icon = Glyph.share)
        }
    }
    Spacer(Modifier.height(18.dp))
    Column(Modifier.testTag("players"), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        PlayerRow(own.getString("name"), res(R.string.player_you), own.getBoolean("ready"), rival = false)
        if (opponent == null) PlayerRow(null, null, false, rival = true)
        else PlayerRow(opponent.getString("name"), if (opponent.getBoolean("connected")) null else res(R.string.conn_disconnected_short), opponent.getBoolean("ready"), rival = true)
    }
    if (countdown) {
        Spacer(Modifier.height(16.dp))
        val remaining = if (live.view.isNull("estimated_server_ms")) null else (snapshot.getLong("start_ms") - live.view.getLong("estimated_server_ms")).coerceAtLeast(0)
        Text(if (remaining == null) res(R.string.countdown_sync) else if (remaining == 0L) res(R.string.countdown_wait) else plural(R.plurals.countdown_seconds, ((remaining + 999) / 1000).toInt(), ((remaining + 999) / 1000).toInt()),
            Modifier.fillMaxWidth().testTag("timer"), color = p.accent, fontWeight = FontWeight.Bold, fontSize = 18.sp, textAlign = TextAlign.Center)
    }
    Spacer(Modifier.height(18.dp))
    val ready = own.getBoolean("ready")
    val readyDescription = res(if (ready) R.string.not_ready_action else R.string.ready_action)
    PrimaryButton(readyDescription, { live.command("ready", !ready) },
        Modifier.fillMaxWidth().testTag("ready-toggle").semantics { stateDescription = readyDescription },
        enabled = opponent != null && live.view.optString("connection") == "connected" && !live.view.optBoolean("pending_command"))
}

/** A seat: avatar initial, name (with a note), ready pill. You in blue, the rival in amber, an empty seat dashed. */
@Composable
private fun PlayerRow(name: String?, note: String?, ready: Boolean, rival: Boolean) {
    val p = ink
    Row(Modifier.fillMaxWidth().heightIn(min = 64.dp).card(p, 18.dp, elevated = false).padding(start = 10.dp, end = 14.dp, top = 10.dp, bottom = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(
            Modifier.size(44.dp).clip(CircleShape)
                .then(if (name == null) Modifier.border(1.5.dp, p.border, CircleShape) else Modifier.background(if (rival) p.rivalSoft else p.selected)),
            contentAlignment = Alignment.Center,
        ) { Text(name?.trim()?.firstOrNull()?.uppercase() ?: "+", color = if (name == null) p.muted else if (rival) p.rival else p.accent, fontWeight = FontWeight.Bold, fontSize = 17.sp) }
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(name ?: res(R.string.lobby_friend_missing), fontWeight = if (name == null) FontWeight.Normal else FontWeight.SemiBold, color = if (name == null) p.muted else p.foreground, maxLines = 2, overflow = TextOverflow.Ellipsis)
            if (note != null) Text(note, color = p.muted, fontSize = 13.sp)
        }
        Spacer(Modifier.width(8.dp))
        Text(if (name == null) "–" else res(if (ready) R.string.player_ready else R.string.player_not_ready),
            Modifier.clip(RoundedCornerShape(99.dp)).background(if (ready) Color(0xff15803d) else p.track).padding(horizontal = 10.dp, vertical = 5.dp),
            color = if (ready) Color.White else p.muted, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
private fun LiveResult(live: LiveState, snapshot: JSONObject, messages: @Composable () -> Unit) {
    val p = ink
    val own = snapshot.getJSONObject("own")
    val opponent = snapshot.optJSONObject("opponent")
    val result = snapshot.optJSONObject("result")
    val reason = result?.optString("reason")
    val noContest = reason in listOf("both_disconnected", "resource_limit")
    val outcome = when {
        result == null || noContest -> "none"
        result.isNull("winner") -> "draw"
        result.getInt("winner") == own.getInt("seat") -> "win"
        else -> "loss"
    }
    Column(Modifier.fillMaxWidth().padding(top = 12.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        when (outcome) {
            "win" -> PageSymbol(Glyph.trophy)
            "loss" -> PageSymbol(Glyph.flag, p.rivalSoft, p.rival)
            else -> PageSymbol(Glyph.equal, p.track, p.muted)
        }
        Spacer(Modifier.height(20.dp))
        Eyebrow(matchLabel(snapshot))
        Spacer(Modifier.height(10.dp))
        PageTitle(res(when (outcome) { "win" -> R.string.result_win; "loss" -> R.string.result_loss; "draw" -> R.string.result_draw; else -> R.string.result_no_contest }),
            Modifier.testTag("result-heading"), Type.title.copy(textAlign = TextAlign.Center))
        Spacer(Modifier.height(10.dp))
        Connection(live)
        Spacer(Modifier.height(14.dp))
        messages()
        if (snapshot.getString("mode") == "duel") {
            val rivalScore = live.opponentScore()
            val details = res(R.string.result_duel_details, own.getInt("score"), rivalScore?.toString() ?: res(R.string.score_unavailable))
            // You on the left in blue, the rival mirrored in amber; screen readers get one sentence.
            Row(Modifier.fillMaxWidth().card(p, 18.dp).padding(horizontal = 16.dp, vertical = 14.dp).testTag("result-details").clearAndSetSemantics { text = AnnotatedString(details); contentDescription = details },
                verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(own.getString("name"), color = p.accent, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(own.getInt("score").toString(), fontSize = 34.sp, fontWeight = FontWeight.Bold)
                }
                Text("–", color = p.muted, fontSize = 20.sp)
                Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(opponent?.optString("name")?.takeIf { it.isNotEmpty() } ?: res(R.string.opponent), color = p.rival, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(rivalScore?.toString() ?: "–", fontSize = 34.sp, fontWeight = FontWeight.Bold)
                }
            }
            if (rivalScore == null) Text(res(R.string.score_unavailable), Modifier.padding(top = 6.dp), color = p.muted, fontSize = 13.sp)
        } else Text(res(R.string.result_race_done), Modifier.testTag("result-details"), color = p.muted, style = Type.body, textAlign = TextAlign.Center)
        Spacer(Modifier.height(10.dp))
        Text(res(when (reason) {
            "resigned" -> R.string.reason_resigned
            "disconnect_timeout" -> R.string.reason_disconnect_timeout
            "both_disconnected" -> R.string.reason_both_disconnected
            "resource_limit" -> R.string.reason_resource_limit
            "time" -> R.string.reason_time
            else -> R.string.reason_default
        }), color = p.muted, style = Type.small, textAlign = TextAlign.Center)
        val rematchNote: Int? = when {
            opponent == null -> R.string.rematch_no_opponent
            !opponent.optBoolean("connected") -> if (own.optBoolean("rematch")) R.string.rematch_requested_disconnected else R.string.rematch_disconnected
            own.optBoolean("rematch") -> R.string.rematch_waiting
            opponent.optBoolean("rematch") -> R.string.rematch_friend_wants
            else -> null
        }
        if (rematchNote != null) Text(res(rematchNote), Modifier.padding(top = 8.dp).testTag("rematch-status"), color = p.foreground, style = Type.small, textAlign = TextAlign.Center)
        if (live.view.optString("connection") != "connected") Text(res(R.string.rematch_needs_connection), Modifier.padding(top = 6.dp).testTag("rematch-connection"), color = p.muted, style = Type.small, textAlign = TextAlign.Center)
        Spacer(Modifier.height(24.dp))
        PrimaryButton(res(R.string.rematch_action), { live.command("rematch") }, Modifier.fillMaxWidth().testTag("rematch"),
            enabled = opponent != null && !live.view.optBoolean("voluntarily_left") && !live.view.optBoolean("pending_command") && live.view.optString("connection") == "connected" && !own.optBoolean("rematch"))
        if (live.view.optString("connection") == "session_lost") {
            Spacer(Modifier.height(10.dp))
            SecondaryButton(res(R.string.live_recover), live::requestReset, Modifier.fillMaxWidth().testTag("live-recover"))
        }
        Spacer(Modifier.height(10.dp))
        SecondaryButton(res(R.string.action_home), live::goHome, Modifier.fillMaxWidth().testTag("home"))
    }
}

@Composable
private fun LiveGame(live: LiveState, snapshot: JSONObject) {
    val p = ink
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
    val timeText = time?.let { if (raceMode) formatTime(it) else formatTime(((it + 999) / 1000) * 1000) } ?: res(R.string.time_syncing)
    val opponent = snapshot.optJSONObject("opponent")
    val lock = live.view.optLong("locked_remaining_ms", 0)
    val receipt = live.view.optJSONObject("last_receipt")?.takeIf { it.optString("match_id") == snapshot.getString("match_id") }
    fun noteList(index: Int) = race?.optJSONArray("notes")?.optJSONArray(index)?.let { list -> (0 until list.length()).map { list.getInt(it) } }.orEmpty()
    val selectedValue = cells?.getInt(live.selected) ?: 0
    val enabled = cells != null && live.canInput() && givens.getInt(live.selected) == 0 && (raceMode || selectedValue == 0)
    val context = LocalContext.current
    val value = { i: Int -> cells?.getInt(i) ?: 0 }
    val given = { i: Int -> givens.getInt(i) != 0 }
    val left = remainingCounts(n, value)
    val rivalName = opponent?.optString("name")?.takeIf { it.isNotEmpty() } ?: res(R.string.opponent)
    GameScreenLayout(
        top = {
            val compact = compactHeight()
            // Below ~700dp the match chrome would cost the board its minimum size: the status joins the score row
            // and the race meters (the percentages stay) give way, as the web does on short screens.
            val oneLine = LocalConfiguration.current.screenHeightDp < 700
            Column(verticalArrangement = Arrangement.spacedBy(if (compact) 4.dp else 6.dp)) {
                if (UiDebug.enabled) Text(res(R.string.app_title), Modifier.testTag("app-title"), style = MaterialTheme.typography.labelMedium)
                GameNavBar(
                    modeLabel = matchLabel(snapshot, short = true), timeText = timeText,
                    timeDescription = res(R.string.live_time_desc, timeText), backDescription = res(R.string.action_back), onBack = live::goHome
                ) { Spacer(Modifier.size(48.dp)) }
                // Connection dot (text only when unhealthy) and the transient state; on short screens they share the score row.
                val lockSeconds = ((lock + 999) / 1000).toInt()
                val (short, long) = when {
                    !raceMode && lock > 0 -> res(R.string.status_lock, lockSeconds) to plural(R.plurals.lock_seconds_desc, lockSeconds, lockSeconds)
                    !raceMode && time == 0L -> res(R.string.status_time_up) to res(R.string.status_time_up_desc)
                    live.view.optBoolean("pending_command") -> res(R.string.status_pending) to res(R.string.status_pending_desc)
                    conflicts.isNotEmpty() -> res(R.string.status_conflict) to res(R.string.status_conflict_desc)
                    UiDebug.enabled -> res(R.string.status_in_progress) to res(R.string.status_in_progress)
                    else -> null to null
                }
                val status: @Composable () -> Unit = {
                    val healthy = connection == "connected"
                    val connectionText = res(when (connection) {
                        "connected" -> R.string.conn_connected
                        "reconnecting" -> R.string.conn_reconnecting
                        "reconcile" -> R.string.conn_reconcile_short
                        "session_lost" -> R.string.conn_lost
                        else -> R.string.conn_disconnected_short
                    })
                    if (healthy) StatusDot(p.ok, connectionText, "connection-status")
                    else Text(connectionText, Modifier.testTag("connection-status").semantics { liveRegion = LiveRegionMode.Polite }, color = p.error, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                    val alarm = (lock > 0 && !raceMode) || conflicts.isNotEmpty()
                    if (short != null) Row(Modifier.padding(start = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                        if (conflicts.isNotEmpty() && raceMode) { Icon(Glyph.warning, null, Modifier.size(18.dp), tint = p.error); Spacer(Modifier.width(6.dp)) }
                        Text(short, Modifier.testTag(if (lock > 0 && !raceMode) "input-lock" else "game-status").semantics { contentDescription = long!!; liveRegion = LiveRegionMode.Polite },
                            color = if (alarm) p.error else p.foreground, fontSize = 13.sp, fontWeight = if (alarm) FontWeight.SemiBold else FontWeight.Normal)
                    }
                }
                // You on the left in blue, the rival mirrored on the right in amber: filled share in a race, score in a duel.
                if (raceMode) {
                    val ownShare = if (cells != null) filledShare(n, value, given) else 0f
                    val progress = opponent?.optInt("progress_percent") ?: 0
                    val progressText = res(R.string.race_progress, progress)
                    val progressDescription = res(R.string.race_progress_desc, progress)
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        if (oneLine) { status(); Spacer(Modifier.width(8.dp)) }
                        Text(res(R.string.player_you), color = p.accent, fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
                        Spacer(Modifier.width(6.dp))
                        Text(res(R.string.percent, (ownShare * 100).toInt()), fontWeight = FontWeight.Bold, fontSize = 16.sp)
                        Spacer(Modifier.weight(1f))
                        Row(Modifier.testTag("opponent-progress").clearAndSetSemantics { text = AnnotatedString(progressText); contentDescription = progressDescription }, verticalAlignment = Alignment.CenterVertically) {
                            Text(res(R.string.percent, progress), fontWeight = FontWeight.Bold, fontSize = 16.sp)
                            Spacer(Modifier.width(6.dp))
                            Text(rivalName, Modifier.widthIn(max = 140.dp), color = p.rival, fontWeight = FontWeight.SemiBold, fontSize = 13.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                    }
                    if (!oneLine) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Meter(ownShare, p.accent, Modifier.weight(1f))
                        Meter(progress / 100f, p.rival, Modifier.weight(1f), fromEnd = true)
                    }
                } else {
                    val rival = live.opponentScore()
                    val youText = res(R.string.duel_you, own.getInt("score"))
                    val youDescription = res(R.string.duel_you_desc, own.getInt("score"))
                    val rivalText = res(R.string.duel_rival, rival?.toString() ?: "–")
                    val rivalDescription = res(R.string.duel_rival_desc, rival?.toString() ?: res(R.string.duel_rival_unknown_desc))
                    val streakText = res(R.string.duel_streak, own.getInt("combo"))
                    val streakDescription = res(R.string.duel_streak_desc, own.getInt("combo"))
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        if (oneLine) { status(); Spacer(Modifier.width(8.dp)) }
                        Row(Modifier.testTag("own-score").clearAndSetSemantics { text = AnnotatedString(youText); contentDescription = youDescription }, verticalAlignment = Alignment.CenterVertically) {
                            Text(res(R.string.player_you), color = p.accent, fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
                            Spacer(Modifier.width(6.dp))
                            Text(own.getInt("score").toString(), fontWeight = FontWeight.Bold, fontSize = 20.sp)
                        }
                        Spacer(Modifier.width(8.dp))
                        Text(streakText, Modifier.clip(RoundedCornerShape(99.dp)).background(p.selected).padding(horizontal = 7.dp, vertical = 1.dp).testTag("combo").semantics { contentDescription = streakDescription },
                            color = p.accent, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                        Spacer(Modifier.weight(1f))
                        Row(Modifier.testTag("opponent-score").clearAndSetSemantics { text = AnnotatedString(rivalText); contentDescription = rivalDescription }, verticalAlignment = Alignment.CenterVertically) {
                            Text(rival?.toString() ?: "–", fontWeight = FontWeight.Bold, fontSize = 20.sp)
                            Spacer(Modifier.width(6.dp))
                            Text(rivalName, Modifier.widthIn(max = 120.dp), color = p.rival, fontWeight = FontWeight.SemiBold, fontSize = 13.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
                        }
                    }
                    val share = ((time ?: 90_000L).coerceIn(0, 90_000) / 90_000f)
                    Meter(share, if ((time ?: 90_000L) <= 10_000) p.error else p.accent)
                }
                if (!oneLine) Row(Modifier.fillMaxWidth().height(22.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    status()
                    Spacer(Modifier.weight(1f))
                    val notice = when (receipt?.optString("code")) {
                        "wrong" -> res(R.string.receipt_wrong)
                        "not_correct" -> res(R.string.receipt_not_correct)
                        else -> null
                    }
                    if (notice != null && short == null) Text(notice, Modifier.semantics { liveRegion = LiveRegionMode.Polite }, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        },
        board = { side ->
            if (cells != null) SudokuBoard(
                n = n, side = side, selected = live.selected, conflicts = conflicts, value = value, given = given, notes = ::noteList,
                label = { index ->
                    val v = cells.getInt(index)
                    cellLabel(context, index, n, if (givens.getInt(index) != 0) R.string.cell_given else if (!raceMode && v != 0) R.string.cell_locked else R.string.cell_editable,
                        v, noteList(index), index in conflicts, ", ")
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
                KeysWithOverlay(keys = {
                    DigitPad(n, enabled, raceMode && live.noteMode, { left[it] }) { live.enter(it) }
                    if (raceMode && race != null) Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        Tool(Glyph.notes, res(R.string.tool_notes), "notes-toggle", enabled = live.canInput(), checked = live.noteMode, description = res(R.string.cd_notes), onClick = live::toggleNotes)
                        Tool(Glyph.erase, res(R.string.action_erase), "erase", enabled = enabled) { live.edit("erase") }
                        Tool(Glyph.undo, res(R.string.cd_undo), "undo", enabled = race.optBoolean("can_undo") && live.canInput()) { live.edit("undo") }
                        Tool(Glyph.redo, res(R.string.cd_redo), "redo", enabled = race.optBoolean("can_redo") && live.canInput()) { live.edit("redo") }
                    }
                }, overlay = null)
                // Messages that need an answer stay in the flow; they only appear when something is wrong.
                if (live.storageError) {
                    Notice(res(R.string.live_storage_error), Modifier, error = true, tag = "live-storage-status") {
                        TextButton(onClick = live::retryStorage, modifier = Modifier.heightIn(min = 48.dp).testTag("live-save-retry"), contentPadding = PaddingValues(0.dp)) { Text(res(R.string.action_retry_save), color = p.error, fontWeight = FontWeight.SemiBold) }
                    }
                }
                live.message?.let { Notice(it.asString(), Modifier, error = true, tag = "live-message") }
                if (live.view.optBoolean("reconcile_required")) {
                    Notice(res(R.string.reconcile_message), Modifier, error = true, tag = "reconcile-message") {
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            TextButton(onClick = { live.reconcile(true) }, modifier = Modifier.heightIn(min = 48.dp).testTag("reconcile-local")) { Text(res(R.string.reconcile_keep_local), color = p.error, fontWeight = FontWeight.SemiBold) }
                            TextButton(onClick = { live.reconcile(false) }, modifier = Modifier.heightIn(min = 48.dp).testTag("reconcile-server")) { Text(res(R.string.reconcile_use_server), color = p.error) }
                        }
                    }
                }
            }
        }
    )
}
