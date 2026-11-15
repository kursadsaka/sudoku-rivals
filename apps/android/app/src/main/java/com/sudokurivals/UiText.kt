package com.sudokurivals

import android.content.Context
import androidx.annotation.PluralsRes
import androidx.annotation.StringRes
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext

/**
 * A user-visible message held as a resource id, not a resolved string: state objects stay
 * language-neutral and the text is resolved when drawn, in the language the system or the
 * per-app language setting has chosen.
 */
internal class UiText(@StringRes val id: Int, vararg val args: Any) {
    fun resolve(context: Context): String = context.getString(id, *args)
}

@Composable
internal fun UiText.asString(): String {
    val context = LocalContext.current
    LocalConfiguration.current // re-resolve when the configuration (language) changes
    return resolve(context)
}

@Composable
internal fun res(@StringRes id: Int, vararg args: Any): String {
    val context = LocalContext.current
    LocalConfiguration.current
    return context.getString(id, *args)
}

/** Quantity strings (English "1 second" / "2 seconds"); Turkish uses the same form for both. */
@Composable
internal fun plural(@PluralsRes id: Int, count: Int, vararg args: Any): String {
    val context = LocalContext.current
    LocalConfiguration.current
    return context.resources.getQuantityString(id, count, *args)
}

/** Accessibility label of one board cell, built from resources so it follows the language. */
internal fun cellLabel(context: Context, index: Int, n: Int, @StringRes state: Int, value: Int, notes: List<Int>, conflict: Boolean, notesSeparator: String): String =
    context.getString(
        R.string.cell_label, index / n + 1, index % n + 1, context.getString(state),
        if (value == 0) context.getString(R.string.cell_empty) else value.toString(),
        notes.joinToString(notesSeparator).ifEmpty { context.getString(R.string.cell_notes_none) },
        if (conflict) context.getString(R.string.cell_conflict_suffix) else ""
    )
