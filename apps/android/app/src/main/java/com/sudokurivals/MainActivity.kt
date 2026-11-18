package com.sudokurivals

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Vibrator
import android.provider.Settings
import android.view.KeyEvent
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.*
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import org.json.JSONObject

class MainActivity : ComponentActivity() {
    private lateinit var preferences: AppPreferences
    private lateinit var feedback: Feedback
    private lateinit var solo: SoloState
    private lateinit var live: LiveState
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        preferences = AppPreferences(getSharedPreferences("preferences", Context.MODE_PRIVATE))
        feedback = Feedback(applicationContext, preferences)
        solo = SoloState(filesDir, preferences, feedback::cue)
        live = LiveState(filesDir, getString(R.string.guest_name), preferences, feedback::cue)
        live.onHome = { if (solo.game != null) solo.exit() else solo.screen = SoloScreen.Home }
        if (BuildConfig.DEBUG) applyDebugExtras(intent)
        intent.dataString?.let { solo.pause(); live.openInvite(it) }
        setContent { SoloApp(solo, live, preferences, feedback) }
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
        if (::solo.isInitialized) solo.close()
        if (::feedback.isInitialized) feedback.release()
        super.onDestroy()
    }
    override fun onStop() {
        if (::solo.isInitialized) solo.pause()
        if (::live.isInitialized) live.setForeground(false)
        super.onStop()
    }
    /** Edge-to-edge with status/navigation icons that suit the chosen theme. */
    internal fun systemBars(dark: Boolean) {
        val transparent = android.graphics.Color.TRANSPARENT
        val style = if (dark) SystemBarStyle.dark(transparent) else SystemBarStyle.light(transparent, transparent)
        enableEdgeToEdge(style, style)
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
                KeyEvent.KEYCODE_N -> if (race && event.repeatCount == 0) live.toggleNotes()
                KeyEvent.KEYCODE_DEL, KeyEvent.KEYCODE_FORWARD_DEL -> if (race && live.canInput()) live.edit("erase")
                KeyEvent.KEYCODE_Z -> if (event.isCtrlPressed || event.isMetaPressed) { if (race && live.canInput()) live.edit(if (event.isShiftPressed) "redo" else "undo") } else return super.dispatchKeyEvent(event)
                KeyEvent.KEYCODE_ESCAPE -> live.goHome()
                else -> return super.dispatchKeyEvent(event)
            }
            return true
        }
        if (::solo.isInitialized && solo.screen == SoloScreen.Game && event.action == KeyEvent.ACTION_DOWN) {
            // Escape first closes a hint or notice, as on the web; the keys under a hint stay inert.
            if (keyCode == KeyEvent.KEYCODE_ESCAPE && (solo.hint != null || solo.hintMessage != null || solo.inputError != null)) { solo.dismissNotices(); return true }
            if (solo.hint != null) return super.dispatchKeyEvent(event)
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
                KeyEvent.KEYCODE_N -> if (event.repeatCount == 0) solo.toggleNotes()
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
private fun SoloApp(solo: SoloState, live: LiveState, preferences: AppPreferences, feedback: Feedback) {
    SudokuTheme(preferences.appearance) {
        val p = ink
        val activity = LocalContext.current as? MainActivity
        val view = LocalView.current
        LaunchedEffect(view) { feedback.view = view }
        LaunchedEffect(p.dark) {
            activity?.systemBars(p.dark)
            activity?.window?.decorView?.setBackgroundColor(p.canvas.toArgb())
        }
        Surface(Modifier.fillMaxSize().semantics { testTagsAsResourceId = true }, color = p.canvas, contentColor = p.foreground) {
            if (live.visible) {
                LiveScreens(live)
            } else {
                BackHandler(enabled = solo.screen != SoloScreen.Home) {
                    when (solo.screen) {
                        SoloScreen.Game -> solo.pause()
                        SoloScreen.Paused, SoloScreen.Result -> solo.exit()
                        SoloScreen.Settings -> solo.screen = solo.settingsReturn
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
                        .verticalScroll(rememberScrollState()).padding(horizontal = 20.dp).padding(bottom = 28.dp),
                ) {
                    when (solo.screen) {
                        SoloScreen.Home -> Home(solo, live)
                        SoloScreen.Setup -> Setup(solo)
                        SoloScreen.Game -> Unit
                        SoloScreen.Paused -> Paused(solo)
                        SoloScreen.Result -> Result(solo)
                        SoloScreen.Settings -> SettingsScreen(solo, preferences, feedback)
                    }
                }
                if (solo.replaceRequested) AlertDialog(
                    onDismissRequest = { solo.replaceRequested = false },
                    title = { Text(res(R.string.replace_title)) },
                    text = { Text(res(if (solo.corruptReplacement) R.string.replace_body_corrupt else R.string.replace_body)) },
                    confirmButton = { TextButton(onClick = solo::newGame, modifier = Modifier.dialogTag("replace-confirm")) { Text(res(R.string.setup_start)) } },
                    dismissButton = { TextButton(onClick = { solo.replaceRequested = false }) { Text(res(R.string.action_cancel)) } },
                    containerColor = p.background, shape = RoundedCornerShape(24.dp),
                )
            }
        }
    }
}

/** Dialogs are separate windows; the root's testTagsAsResourceId does not reach them. */
@OptIn(ExperimentalComposeUiApi::class)
internal fun Modifier.dialogTag(tag: String) = semantics { testTagsAsResourceId = true }.testTag(tag)

/** Site header: the mark and wordmark, Settings on the right (home only). */
@Composable
private fun Header(onSettings: (() -> Unit)?) {
    val p = ink
    val settings = res(R.string.home_settings)
    Row(Modifier.fillMaxWidth().padding(top = 10.dp, bottom = 4.dp).heightIn(min = 48.dp), verticalAlignment = Alignment.CenterVertically) {
        BrandMark()
        Spacer(Modifier.width(10.dp))
        val title = res(R.string.app_title)
        Text(title.substringBefore(" ") + " ", fontSize = 18.sp, color = p.foreground, modifier = Modifier.testTag("app-title").clearAndSetSemantics { heading(); contentDescription = title })
        Text(title.substringAfter(" "), fontSize = 18.sp, fontWeight = FontWeight.ExtraBold, color = p.foreground, modifier = Modifier.clearAndSetSemantics { })
        Spacer(Modifier.weight(1f))
        if (onSettings != null) SquareIconButton(Glyph.settings, settings, onSettings, Modifier.testTag("settings-open"))
    }
}

@Composable
private fun Home(solo: SoloState, live: LiveState) {
    val p = ink
    Header { solo.settingsReturn = SoloScreen.Home; solo.screen = SoloScreen.Settings }
    Spacer(Modifier.height(16.dp))
    Eyebrow(res(R.string.home_eyebrow))
    Spacer(Modifier.height(10.dp))
    PageTitle(res(R.string.home_title), style = Type.hero)
    Spacer(Modifier.height(12.dp))
    Lede(res(R.string.home_subtitle))
    Spacer(Modifier.height(20.dp))
    ModeCard(primary = true, friends = false, title = res(R.string.home_solo_start), description = res(R.string.home_solo_description), tag = "solo-start", onClick = solo::openSetup)
    Spacer(Modifier.height(12.dp))
    ModeCard(primary = false, friends = true, title = res(R.string.home_friend_start), description = res(R.string.home_friend_description), tag = "friend-start", onClick = live::open)
    if (live.snapshot() != null && !live.view.optBoolean("voluntarily_left")) {
        Spacer(Modifier.height(12.dp))
        val text = res(R.string.home_friend_return)
        Row(
            Modifier.fillMaxWidth().heightIn(min = 64.dp).card(p, 18.dp, stroke = p.rival, elevated = false).clickable(role = androidx.compose.ui.semantics.Role.Button, onClick = live::open)
                .testTag("live-resume").padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(text, Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
            Icon(Glyph.arrow, null, Modifier.size(20.dp), tint = p.accent)
        }
    }
    val saved = solo.slots.filterValues { it.status == "saved" }
    if (saved.isNotEmpty()) {
        Spacer(Modifier.height(28.dp))
        SectionLabel(res(R.string.home_continue))
        Spacer(Modifier.height(10.dp))
        for ((size, slot) in saved) {
            ResumeCard(solo, size, slot)
            Spacer(Modifier.height(10.dp))
        }
    }
    for ((size, slot) in solo.slots) if (slot.status == "corrupt") {
        Spacer(Modifier.height(12.dp))
        Notice(res(R.string.solo_corrupt_slot, size), Modifier, error = true, tag = "storage-status") {
            TextButton(onClick = solo::refreshSlots, modifier = Modifier.testTag("retry-$size"), contentPadding = PaddingValues(0.dp)) { Text(res(R.string.action_retry), color = p.error, fontWeight = FontWeight.SemiBold) }
        }
    }
    solo.storageError?.let { Spacer(Modifier.height(12.dp)); Notice(it.asString(), Modifier, error = true, tag = "storage-status") }
    Spacer(Modifier.height(28.dp))
    Text(res(R.string.home_offline), style = Type.small, color = p.muted)
}

@Composable
private fun ModeCard(primary: Boolean, friends: Boolean, title: String, description: String, tag: String, onClick: () -> Unit) {
    val p = ink
    val fill = if (primary) p.brand else p.background
    val content = if (primary) p.onBrand else p.foreground
    Row(
        Modifier.fillMaxWidth().heightIn(min = 104.dp).card(p, 22.dp, fill, if (primary) p.brand else p.line)
            .clickable(role = androidx.compose.ui.semantics.Role.Button, onClick = onClick).testTag(tag).padding(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(64.dp).clip(RoundedCornerShape(16.dp)).background(if (primary) Color.White.copy(alpha = .16f) else p.canvas), contentAlignment = Alignment.Center) {
            ModeArt(friends, if (primary) p.onBrand else p.accent)
        }
        Spacer(Modifier.width(16.dp))
        Column(Modifier.weight(1f)) {
            Text(title, style = Type.subtitle, color = content)
            Spacer(Modifier.height(4.dp))
            Text(description, fontSize = 14.sp, lineHeight = 19.sp, color = if (primary) content.copy(alpha = .86f) else p.muted)
        }
        Spacer(Modifier.width(12.dp))
        Box(Modifier.size(36.dp).clip(CircleShape).background(if (primary) p.onBrand else p.selected), contentAlignment = Alignment.Center) {
            Icon(Glyph.arrow, null, Modifier.size(20.dp), tint = if (primary) p.brand else p.accent)
        }
    }
}

@Composable
private fun ResumeCard(solo: SoloState, size: Int, slot: Slot) {
    val p = ink
    Row(
        Modifier.fillMaxWidth().heightIn(min = 64.dp).card(p, 18.dp, elevated = false)
            .clickable(role = androidx.compose.ui.semantics.Role.Button) { solo.resumeSaved(size) }.testTag("solo-resume-$size")
            .padding(start = 10.dp, end = 14.dp, top = 10.dp, bottom = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(44.dp).clip(RoundedCornerShape(12.dp)).background(p.selected), contentAlignment = Alignment.Center) {
            Text(res(R.string.size_label, size), color = p.accent, fontWeight = FontWeight.Bold, fontSize = 14.sp)
        }
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(res(if (slot.complete) R.string.home_completed else R.string.home_saved_game), fontWeight = FontWeight.SemiBold)
            Text(listOfNotNull(slot.level?.let { levelName(it) }, formatTime(slot.elapsedMillis)).joinToString(" · "), color = p.muted, style = Type.small)
        }
        if (!slot.complete) Text(res(R.string.action_resume), color = p.accent, fontWeight = FontWeight.SemiBold, fontSize = 14.sp)
        Spacer(Modifier.width(4.dp))
        Icon(if (slot.complete) Glyph.arrow else Glyph.play, null, Modifier.size(18.dp), tint = p.accent)
    }
}

@Composable
internal fun levelName(level: String): String = res(when (level) {
    "beginner" -> R.string.level_beginner; "easy" -> R.string.level_easy; "medium" -> R.string.level_medium; "hard" -> R.string.level_hard; else -> R.string.level_expert
})

@Composable
internal fun levelOptions(): List<Pair<String, String>> = LEVELS.map { it to levelName(it) }

/** A selectable size card (web `.size-option`): the picture, `N×N`, a short line, a check when chosen. */
@Composable
internal fun SizeCard(size: Int, selected: Boolean, caption: String, enabled: Boolean = true, modifier: Modifier = Modifier, onClick: () -> Unit) {
    val p = ink
    val shape = RoundedCornerShape(18.dp)
    Box(
        modifier.clip(shape).background(if (selected) p.selected else p.background).border(1.5.dp, if (selected) p.accent else p.line, shape)
            .selectable(selected, enabled = enabled, role = androidx.compose.ui.semantics.Role.RadioButton, onClick = onClick).testTag("board-size-$size").padding(16.dp)
    ) {
        Column {
            SizeArt(size, if (selected) p.accent else p.muted)
            Spacer(Modifier.height(10.dp))
            Text(res(R.string.size_label, size), fontSize = 24.sp, fontWeight = FontWeight.Bold, letterSpacing = (-0.5).sp)
            Text(caption, color = p.muted, style = Type.small)
        }
        if (selected) Box(Modifier.align(Alignment.TopEnd).size(22.dp).clip(CircleShape).background(p.brand), contentAlignment = Alignment.Center) {
            Icon(Glyph.check, null, Modifier.size(14.dp), tint = p.onBrand)
        }
    }
}

@Composable
private fun Setup(solo: SoloState) {
    Spacer(Modifier.height(10.dp))
    BackLink(res(R.string.action_back), { solo.screen = SoloScreen.Home })
    Eyebrow(res(R.string.home_solo_start))
    Spacer(Modifier.height(10.dp))
    PageTitle(res(R.string.setup_title))
    Spacer(Modifier.height(10.dp))
    Lede(res(R.string.setup_intro))
    Spacer(Modifier.height(20.dp))
    val sizeLabel = res(R.string.board_size)
    Text(sizeLabel, color = ink.muted, fontWeight = FontWeight.Medium, fontSize = 14.sp)
    Spacer(Modifier.height(8.dp))
    Row(Modifier.fillMaxWidth().semantics { contentDescription = sizeLabel }.testTag("board-size"), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        SizeCard(6, solo.size == 6, res(R.string.size_six_caption), !solo.preparing, Modifier.weight(1f)) { solo.selectSize(6) }
        SizeCard(9, solo.size == 9, res(R.string.size_nine_caption), !solo.preparing, Modifier.weight(1f)) { solo.selectSize(9) }
    }
    Spacer(Modifier.height(20.dp))
    Text(res(R.string.difficulty), color = ink.muted, fontWeight = FontWeight.Medium, fontSize = 14.sp)
    Spacer(Modifier.height(8.dp))
    Segmented(levelOptions(), solo.difficulty, solo::selectDifficulty, Modifier.fillMaxWidth().testTag("difficulty"), tagPrefix = "difficulty-", enabled = !solo.preparing)
    Spacer(Modifier.height(24.dp))
    PrimaryButton(res(if (solo.preparing) R.string.setup_preparing else R.string.setup_start), solo::prepareNew, Modifier.fillMaxWidth().testTag("game-start"), enabled = !solo.preparing)
    Spacer(Modifier.height(12.dp))
    Text(res(R.string.setup_note), style = Type.small, color = ink.muted)
    solo.storageError?.let {
        Spacer(Modifier.height(12.dp))
        Notice(it.asString(), Modifier, error = true, tag = "storage-status") {
            TextButton(onClick = solo::prepareNew, modifier = Modifier.testTag("retry"), contentPadding = PaddingValues(0.dp)) { Text(res(R.string.action_retry), color = ink.error, fontWeight = FontWeight.SemiBold) }
        }
    }
}

/** The centred symbol of the pause and result pages. */
@Composable
internal fun PageSymbol(icon: androidx.compose.ui.graphics.vector.ImageVector, fill: Color = ink.selected, tint: Color = ink.accent) {
    Box(Modifier.size(76.dp).clip(RoundedCornerShape(24.dp)).background(fill), contentAlignment = Alignment.Center) { Icon(icon, null, Modifier.size(34.dp), tint = tint) }
}

/** `9×9 · Easy   04:12` box. */
@Composable
internal fun StatBox(caption: String, value: String, modifier: Modifier = Modifier) {
    val p = ink
    Row(modifier.card(p, 16.dp, elevated = false).padding(horizontal = 18.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(caption, color = p.muted, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
        Spacer(Modifier.width(10.dp))
        Text(value, fontSize = 24.sp, fontWeight = FontWeight.Bold, color = p.foreground)
    }
}

private fun soloCaption(size: Int, level: String?, levelText: String?): String = listOfNotNull("$size×$size", levelText).joinToString(" · ")

@Composable
private fun CentredPage(content: @Composable ColumnScope.() -> Unit) {
    Column(Modifier.fillMaxWidth().padding(top = 48.dp), horizontalAlignment = Alignment.CenterHorizontally, content = content)
}

@Composable
private fun Paused(solo: SoloState) {
    val level = solo.levelOf(solo.game)
    CentredPage {
        PageSymbol(Glyph.pause)
        Spacer(Modifier.height(20.dp))
        PageTitle(res(R.string.paused_title), style = Type.title.copy(textAlign = TextAlign.Center))
        Spacer(Modifier.height(10.dp))
        Text(res(R.string.paused_body), style = Type.body, color = ink.muted, textAlign = TextAlign.Center)
        Spacer(Modifier.height(18.dp))
        StatBox(soloCaption(solo.size, level, level?.let { levelName(it) }), formatTime(solo.elapsedDisplay))
        solo.storageError?.let { Spacer(Modifier.height(12.dp)); StorageNotice(solo, it) }
        Spacer(Modifier.height(26.dp))
        PrimaryButton(res(R.string.action_resume), solo::resume, Modifier.fillMaxWidth().testTag("resume"))
        Spacer(Modifier.height(10.dp))
        SecondaryButton(res(R.string.action_save_exit), { solo.exit() }, Modifier.fillMaxWidth().testTag("save-exit"))
    }
}

@Composable
private fun Result(solo: SoloState) {
    val level = solo.levelOf(solo.game)
    CentredPage {
        PageSymbol(Glyph.check)
        Spacer(Modifier.height(20.dp))
        Eyebrow(res(R.string.home_completed))
        Spacer(Modifier.height(10.dp))
        PageTitle(res(R.string.solo_result_title), Modifier.testTag("result-heading"), Type.title.copy(textAlign = TextAlign.Center))
        Spacer(Modifier.height(10.dp))
        Text(res(R.string.solo_result_body), style = Type.body, color = ink.muted, textAlign = TextAlign.Center)
        Spacer(Modifier.height(18.dp))
        StatBox(soloCaption(solo.size, level, level?.let { levelName(it) }), formatTime(solo.elapsedDisplay), Modifier.testTag("result-details"))
        solo.storageError?.let { Spacer(Modifier.height(12.dp)); StorageNotice(solo, it) }
        Spacer(Modifier.height(26.dp))
        PrimaryButton(res(R.string.action_new_game), { if (solo.exit()) solo.openSetup() }, Modifier.fillMaxWidth().testTag("new-game"))
        Spacer(Modifier.height(10.dp))
        SecondaryButton(res(R.string.action_home), { solo.exit() }, Modifier.fillMaxWidth().testTag("home"))
    }
}

@Composable
private fun StorageNotice(solo: SoloState, error: UiText) {
    Notice(error.asString(), Modifier, error = true, tag = "storage-status") {
        TextButton(onClick = { solo.save() }, modifier = Modifier.testTag("retry"), contentPadding = PaddingValues(0.dp)) { Text(res(R.string.action_retry), color = ink.error, fontWeight = FontWeight.SemiBold) }
    }
}

@Composable
private fun SettingsRow(title: String, note: String? = null, modifier: Modifier = Modifier, trailing: @Composable () -> Unit) {
    Row(modifier.fillMaxWidth().heightIn(min = 60.dp).padding(vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(title, fontWeight = FontWeight.Medium)
            if (note != null) Text(note, color = ink.muted, fontSize = 13.sp, lineHeight = 17.sp)
        }
        Spacer(Modifier.width(16.dp))
        trailing()
    }
}

@Composable
private fun SettingsScreen(solo: SoloState, preferences: AppPreferences, feedback: Feedback) {
    val p = ink
    val context = LocalContext.current
    val canVibrate = remember { (context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator)?.hasVibrator() == true }
    val switchColors = SwitchDefaults.colors(checkedTrackColor = p.brand, checkedThumbColor = Color.White, uncheckedTrackColor = p.border, uncheckedThumbColor = Color.White, uncheckedBorderColor = p.border)
    Spacer(Modifier.height(10.dp))
    BackLink(res(R.string.action_back), { solo.screen = solo.settingsReturn })
    PageTitle(res(R.string.home_settings))
    Spacer(Modifier.height(12.dp))
    Column(Modifier.fillMaxWidth().card(p, 20.dp, elevated = false).padding(horizontal = 16.dp)) {
        SettingsRow(res(R.string.settings_theme)) {}
        Segmented(listOf("system" to res(R.string.theme_system), "light" to res(R.string.theme_light), "dark" to res(R.string.theme_dark)), preferences.appearance,
            { preferences.appearance = (it); feedback.cue(Cue.Select) }, Modifier.fillMaxWidth().padding(bottom = 14.dp).testTag("theme"), tagPrefix = "theme-", compact = true)
        HorizontalDivider(color = p.line)
        val soundLabel = res(R.string.settings_sound)
        SettingsRow(soundLabel, res(R.string.settings_sound_note)) {
            Switch(preferences.sound, { preferences.sound = (it); if (it) feedback.cue(Cue.Toggle) }, Modifier.testTag("sound-toggle").semantics { contentDescription = soundLabel }, colors = switchColors)
        }
        if (canVibrate) {
            HorizontalDivider(color = p.line)
            val hapticsLabel = res(R.string.settings_haptics)
            SettingsRow(hapticsLabel, res(R.string.settings_haptics_note)) {
                Switch(preferences.haptics, { preferences.haptics = (it); if (it) feedback.cue(Cue.Toggle) }, Modifier.testTag("haptics-toggle").semantics { contentDescription = hapticsLabel }, colors = switchColors)
            }
        }
        HorizontalDivider(color = p.line)
        // Language is Android's per-app language (Android 13+), the same place every app keeps it.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            SettingsRow(res(R.string.settings_language), res(R.string.settings_language_note), Modifier.clickable(role = androidx.compose.ui.semantics.Role.Button) {
                runCatching { context.startActivity(Intent(Settings.ACTION_APP_LOCALE_SETTINGS, Uri.fromParts("package", context.packageName, null))) }
            }.testTag("language")) { Icon(Glyph.arrow, null, Modifier.size(20.dp), tint = p.muted) }
        } else SettingsRow(res(R.string.settings_language), res(R.string.settings_language_system)) {}
        HorizontalDivider(color = p.line)
        SettingsRow(res(R.string.settings_motion), res(R.string.settings_body_system)) {}
    }
    Spacer(Modifier.height(28.dp))
    SectionLabel(res(R.string.settings_storage))
    Spacer(Modifier.height(8.dp))
    Text(res(R.string.settings_body_storage), style = Type.small, color = p.muted)
    Spacer(Modifier.height(8.dp))
    Column(Modifier.testTag("storage-list")) {
        for ((size, slot) in solo.slots) Text(
            res(R.string.size_label, size) + " · " + when (slot.status) {
                "saved" -> res(if (slot.complete) R.string.home_completed else R.string.home_saved_game) + " · " + formatTime(slot.elapsedMillis)
                "corrupt" -> res(R.string.solo_corrupt_short)
                else -> "–"
            }, style = Type.small, color = p.muted
        )
    }
}

@Composable
private fun Game(solo: SoloState) {
    val p = ink
    val state = JSONObject(solo.game ?: return)
    val cells = state.getJSONArray("cells")
    val givens = state.getJSONArray("givens")
    val n = solo.size
    val conflicts = solo.conflicts()
    val valid = conflicts.isEmpty()
    val editable = givens.getInt(solo.selected) == 0
    val context = LocalContext.current
    val value = { i: Int -> cells.getJSONObject(i).getInt("value") }
    val given = { i: Int -> givens.getInt(i) != 0 }
    val left = remainingCounts(n, value)
    val level = solo.levelOf(solo.game)
    val levelText = level?.let { levelName(it) }
    val statusDescription = res(if (valid) R.string.status_in_progress else R.string.status_conflict_desc)
    val hint = solo.hint
    val marked = hint?.optJSONArray("eliminations")?.let { steps -> (0 until steps.length()).flatMap { s -> steps.getJSONObject(s).optJSONArray("cells")?.let { c -> (0 until c.length()).map(c::getInt) }.orEmpty() } }?.toSet().orEmpty()
    GameScreenLayout(
        top = {
            val compact = compactHeight()
            Column(verticalArrangement = Arrangement.spacedBy(if (compact) 4.dp else 6.dp)) {
                if (UiDebug.enabled) Text(res(R.string.app_title), Modifier.testTag("app-title"), style = MaterialTheme.typography.labelMedium)
                GameNavBar(
                    modeLabel = listOfNotNull(res(R.string.solo_mode_label, n), levelText).joinToString(" · "), timeText = formatTime(solo.elapsedDisplay),
                    timeDescription = res(R.string.elapsed_description, formatTime(solo.elapsedDisplay)), backDescription = res(R.string.action_back), onBack = solo::pause,
                    notice = (solo.inputError ?: solo.hintMessage)?.asString(), onDismissNotice = solo::dismissNotices,
                ) { SquareIconButton(Glyph.pause, res(R.string.action_pause), solo::pause, Modifier.testTag("pause")) }
                Meter(filledShare(n, value, given), p.accent)
                // One fixed-height status row. Healthy play shows no text (screen readers still get it).
                Row(Modifier.fillMaxWidth().height(if (compact) 18.dp else 22.dp).testTag("game-status").semantics { liveRegion = LiveRegionMode.Polite; contentDescription = statusDescription }, verticalAlignment = Alignment.CenterVertically) {
                    if (!valid) {
                        Icon(Glyph.warning, null, Modifier.size(18.dp), tint = p.error)
                        Spacer(Modifier.width(6.dp))
                        Text(res(R.string.status_conflict), Modifier.clearAndSetSemantics { }, color = p.error, fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
                    } else if (UiDebug.enabled) Text(res(R.string.status_in_progress), Modifier.clearAndSetSemantics { }, style = MaterialTheme.typography.bodyMedium)
                }
            }
        },
        board = { side ->
            SudokuBoard(
                n = n, side = side, selected = solo.selected, conflicts = conflicts, value = value, given = given,
                notes = { i -> val bits = cells.getJSONObject(i).getInt("notes"); (1..n).filter { bits and (1 shl (it - 1)) != 0 } },
                label = { index ->
                    val cell = cells.getJSONObject(index)
                    cellLabel(context, index, n, if (givens.getInt(index) != 0) R.string.cell_given else R.string.cell_editable, cell.getInt("value"),
                        (1..n).filter { cell.getInt("notes") and (1 shl (it - 1)) != 0 }, index in conflicts, " ")
                },
                onSelect = solo::select, marked = marked,
            )
        },
        bottom = {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                if (UiDebug.enabled) {
                    val selectedCell = cells.getJSONObject(solo.selected)
                    val selectedNotes = selectedCell.getInt("notes")
                    Text(res(R.string.selected_cell_line, solo.selected / n + 1, solo.selected % n + 1,
                        (if (selectedCell.getInt("value") == 0) res(R.string.selected_cell_empty) else res(R.string.selected_cell_value, selectedCell.getInt("value"))) +
                            if (selectedNotes == 0) "" else res(R.string.selected_cell_notes_suffix, (1..n).filter { selectedNotes and (1 shl (it - 1)) != 0 }.joinToString(", "))),
                        Modifier.testTag("selected-cell"), style = MaterialTheme.typography.bodyMedium)
                }
                KeysWithOverlay(
                    keys = {
                        DigitPad(n, editable, solo.noteMode, { left[it] }, { context.getString(R.string.enter_digit, it) }) { solo.enter(it) }
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                            Tool(Glyph.notes, res(R.string.tool_notes), "notes-toggle", checked = solo.noteMode, description = res(R.string.cd_notes), onClick = solo::toggleNotes)
                            Tool(Glyph.erase, res(R.string.action_erase), "erase", enabled = editable) { solo.apply("erase") }
                            Tool(Glyph.undo, res(R.string.cd_undo), "undo", enabled = state.getJSONArray("undo").length() > 0) { solo.apply("undo") }
                            Tool(Glyph.redo, res(R.string.cd_redo), "redo", enabled = state.getJSONArray("redo").length() > 0) { solo.apply("redo") }
                            Tool(Glyph.hint, res(R.string.cd_hint), "hint", onClick = solo::requestHint)
                        }
                    },
                    overlay = hint?.let { { HintCard(it, n, onDismiss = solo::dismissNotices, onApply = solo::acceptHint) } },
                )
                // A failed save stays in the flow until it is fixed; passing notices float over the heading.
                solo.storageError?.let { StorageNotice(solo, it) }
                if (UiDebug.enabled && solo.storageError == null) Text(res(R.string.storage_saved), Modifier.testTag("storage-status"), style = MaterialTheme.typography.bodySmall)
            }
        }
    )
}

/** One sentence per elimination the placement depends on, then the placement itself (web `hintLines`). */
@Composable
private fun hintLines(hint: JSONObject, n: Int): List<String> {
    val context = LocalContext.current
    androidx.compose.ui.platform.LocalConfiguration.current
    fun unit(o: JSONObject) = context.getString(when (o.getString("kind")) { "row" -> R.string.unit_row; "column" -> R.string.unit_column; else -> R.string.unit_box }, o.getInt("index") + 1)
    val steps = hint.optJSONArray("eliminations")?.let { list -> (0 until list.length()).map { list.getJSONObject(it) } }.orEmpty().map { step ->
        val digits = step.getJSONArray("digits")
        if (step.getString("technique") == "locked_candidates") context.getString(R.string.hint_locked, unit(step.getJSONObject("unit")), unit(step.getJSONObject("target")), digits.getInt(0))
        else context.getString(if (step.getString("technique") == "naked_pair") R.string.hint_naked_pair else R.string.hint_hidden_pair, unit(step.getJSONObject("unit")), digits.getInt(0), digits.getInt(1))
    }
    val index = hint.getInt("index")
    val row = index / n + 1
    val column = index % n + 1
    val value = hint.getInt("value")
    val last = if (hint.getString("technique") == "naked_single")
        context.getString(if (steps.isEmpty()) R.string.hint_naked else R.string.hint_naked_then, row, column, value)
    else context.getString(if (steps.isEmpty()) R.string.hint_hidden else R.string.hint_hidden_then, unit(hint.getJSONObject("unit")), value, row, column)
    return steps + last
}

@Composable
private fun HintCard(hint: JSONObject, n: Int, onDismiss: () -> Unit, onApply: () -> Unit) {
    val p = ink
    val lines = hintLines(hint, n)
    val title = res(R.string.cd_hint)
    Column(
        Modifier.fillMaxSize().card(p, 16.dp, stroke = p.accent).testTag("hint-card").semantics { paneTitle = title }.padding(horizontal = 12.dp, vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.weight(1f).verticalScroll(rememberScrollState()).semantics { liveRegion = LiveRegionMode.Polite }) {
            Icon(Glyph.hint, null, Modifier.padding(top = 1.dp).size(20.dp), tint = p.accent)
            Spacer(Modifier.width(8.dp))
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) { for (line in lines) Text(line, fontSize = 14.sp, lineHeight = 19.sp, color = p.foreground) }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = onDismiss, modifier = Modifier.heightIn(min = 44.dp).testTag("hint-dismiss"), shape = RoundedCornerShape(12.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, p.border), colors = ButtonDefaults.outlinedButtonColors(contentColor = p.foreground)) { Text(res(R.string.action_dismiss)) }
            Button(onClick = onApply, modifier = Modifier.weight(1f).heightIn(min = 44.dp).testTag("hint-apply"), shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(containerColor = p.brand, contentColor = p.onBrand)) { Text(res(R.string.action_apply), fontWeight = FontWeight.SemiBold) }
        }
    }
}

internal fun formatTime(milliseconds: Long): String {
    val seconds = milliseconds / 1000
    return "%d:%02d".format(seconds / 60, seconds % 60)
}
