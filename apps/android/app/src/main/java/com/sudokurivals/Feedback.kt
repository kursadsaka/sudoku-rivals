package com.sudokurivals

import android.content.Context
import android.content.SharedPreferences
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioTrack
import android.os.Build
import android.view.HapticFeedbackConstants
import android.view.View
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.math.PI
import kotlin.math.min
import kotlin.math.sin

internal val LEVELS = listOf("beginner", "easy", "medium", "hard", "expert")

/**
 * Settings and remembered choices (preferences only; games have their own files). Board size defaults to
 * 9×9, difficulty to Easy and the friend mode to the classic race, as on the web and iOS, until the player
 * picks something else; the last pick is remembered.
 */
internal class AppPreferences(private val store: SharedPreferences) {
    private fun level(key: String) = store.getString(key, null)?.takeIf { it in LEVELS } ?: "easy"
    private fun size(key: String) = store.getInt(key, 9).takeIf { it == 6 || it == 9 } ?: 9

    /** system | light | dark */
    private var appearanceState by mutableStateOf(store.getString("appearance", null)?.takeIf { it in listOf("system", "light", "dark") } ?: "system")
    var appearance: String
        get() = appearanceState
        set(value) { appearanceState = value; store.edit().putString("appearance", appearanceState).apply() }
    /** Interaction sounds: off by default, as on the web and iOS. */
    private var soundState by mutableStateOf(store.getBoolean("sound", false))
    var sound: Boolean
        get() = soundState
        set(value) { soundState = value; store.edit().putBoolean("sound", soundState).apply() }
    /** Touch feedback: on by default, like Android's own keyboard; the system "Touch feedback" setting still applies. */
    private var hapticsState by mutableStateOf(store.getBoolean("haptics", true))
    var haptics: Boolean
        get() = hapticsState
        set(value) { hapticsState = value; store.edit().putBoolean("haptics", hapticsState).apply() }
    private var soloSizeState by mutableStateOf(size("soloSize"))
    var soloSize: Int
        get() = soloSizeState
        set(value) { soloSizeState = value; store.edit().putInt("soloSize", soloSizeState).apply() }
    private var soloDifficultyState by mutableStateOf(level("soloDifficulty"))
    var soloDifficulty: String
        get() = soloDifficultyState
        set(value) { soloDifficultyState = value; store.edit().putString("soloDifficulty", soloDifficultyState).apply() }
    private var liveModeState by mutableStateOf(store.getString("liveMode", null)?.takeIf { it == "race" || it == "duel" } ?: "race")
    var liveMode: String
        get() = liveModeState
        set(value) { liveModeState = value; store.edit().putString("liveMode", liveModeState).apply() }
    private var liveSizeState by mutableStateOf(size("liveSize"))
    var liveSize: Int
        get() = liveSizeState
        set(value) { liveSizeState = value; store.edit().putInt("liveSize", liveSizeState).apply() }
    private var liveDifficultyState by mutableStateOf(level("liveDifficulty"))
    var liveDifficulty: String
        get() = liveDifficultyState
        set(value) { liveDifficultyState = value; store.edit().putString("liveDifficulty", liveDifficultyState).apply() }
    private var displayNameState by mutableStateOf(store.getString("displayName", null)?.take(32).orEmpty())
    var displayName: String
        get() = displayNameState
        set(value) { displayNameState = value.take(32); store.edit().putString("displayName", displayNameState).apply() }

}

/** The web's cue names (apps/web/sound.js); each maps to touch feedback and an optional short tone. */
internal enum class Cue { Tap, Type, Select, Toggle, Open, Close, Ready, Attention, Error, Success }

/**
 * Touch feedback through the view (follows the system "Touch feedback" switch) and short tones synthesized
 * in code: no audio files, nothing plays while the ringer is silenced or set to vibrate, and the audio
 * focus is never taken, so other audio keeps playing.
 */
internal class Feedback(private val context: Context, private val preferences: AppPreferences) {
    var view: View? = null
    private val tracks = HashMap<Cue, AudioTrack>()

    fun cue(cue: Cue) {
        if (preferences.haptics) haptic(cue)
        if (preferences.sound) runCatching { tone(cue) }
    }

    private fun haptic(cue: Cue) {
        val target = view ?: return
        val modern = Build.VERSION.SDK_INT >= Build.VERSION_CODES.R
        val constant = when (cue) {
            Cue.Tap, Cue.Select -> HapticFeedbackConstants.KEYBOARD_TAP
            Cue.Type, Cue.Toggle -> HapticFeedbackConstants.CLOCK_TICK
            Cue.Ready -> HapticFeedbackConstants.VIRTUAL_KEY
            Cue.Error -> if (modern) HapticFeedbackConstants.REJECT else HapticFeedbackConstants.LONG_PRESS
            Cue.Success, Cue.Attention -> if (modern) HapticFeedbackConstants.CONFIRM else HapticFeedbackConstants.LONG_PRESS
            Cue.Open, Cue.Close -> return
        }
        target.performHapticFeedback(constant)
    }

    /** Tone recipe per cue: (frequency Hz, milliseconds, gain). Quiet and short, like the web's Cuelume set. */
    private fun recipe(cue: Cue): List<Triple<Double, Int, Double>> = when (cue) {
        Cue.Tap -> listOf(Triple(880.0, 28, .18))
        Cue.Type -> listOf(Triple(1320.0, 18, .12))
        Cue.Select -> listOf(Triple(740.0, 24, .15))
        Cue.Toggle -> listOf(Triple(660.0, 26, .15), Triple(990.0, 30, .15))
        Cue.Open -> listOf(Triple(587.0, 30, .12), Triple(880.0, 40, .12))
        Cue.Close -> listOf(Triple(880.0, 30, .12), Triple(587.0, 40, .12))
        Cue.Ready -> listOf(Triple(784.0, 40, .16), Triple(1047.0, 60, .16))
        Cue.Attention -> listOf(Triple(659.0, 70, .2), Triple(988.0, 110, .2))
        Cue.Error -> listOf(Triple(233.0, 60, .22), Triple(0.0, 30, 0.0), Triple(208.0, 80, .22))
        Cue.Success -> listOf(Triple(523.0, 70, .18), Triple(659.0, 70, .18), Triple(784.0, 140, .18))
    }

    private fun tone(cue: Cue) {
        val audio = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager
        if (audio.ringerMode != AudioManager.RINGER_MODE_NORMAL) return
        val track = tracks.getOrPut(cue) { build(recipe(cue)) }
        if (track.playState == AudioTrack.PLAYSTATE_PLAYING) track.stop()
        track.reloadStaticData()
        track.play()
    }

    private fun build(parts: List<Triple<Double, Int, Double>>): AudioTrack {
        val rate = 44_100
        val samples = ArrayList<Short>()
        for ((frequency, ms, gain) in parts) {
            val count = rate * ms / 1000
            val edge = min(count / 4, rate * 6 / 1000).coerceAtLeast(1)
            for (i in 0 until count) {
                val envelope = min(1.0, min(i, count - 1 - i).toDouble() / edge)
                samples += (sin(2 * PI * frequency * i / rate) * gain * envelope * Short.MAX_VALUE).toInt().toShort()
            }
        }
        val data = samples.toShortArray()
        val track = AudioTrack.Builder()
            .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME).setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION).build())
            .setAudioFormat(AudioFormat.Builder().setSampleRate(rate).setEncoding(AudioFormat.ENCODING_PCM_16BIT).setChannelMask(AudioFormat.CHANNEL_OUT_MONO).build())
            .setTransferMode(AudioTrack.MODE_STATIC).setBufferSizeInBytes(data.size * 2).build()
        track.write(data, 0, data.size)
        return track
    }

    fun release() { tracks.values.forEach { runCatching { it.release() } }; tracks.clear() }
}
