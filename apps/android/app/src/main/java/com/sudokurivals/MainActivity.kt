package com.sudokurivals
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import uniffi.sudoku_ffi.validateBoard
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            var text by remember { mutableStateOf("1 0 3 4 5 6\n4 0 6 1 2 3\n0 3 0 5 6 1\n5 6 0 2 0 4\n3 0 5 6 0 2\n0 1 0 0 4 0") }
            var result by remember { mutableStateOf("") }
            MaterialTheme {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text("Sudoku Rivals", style = MaterialTheme.typography.headlineMedium)
                    Text("Ortak çekirdek doğrulama denemesi (6×6)")
                    OutlinedTextField(value = text, onValueChange = { text = it }, label = { Text("36 sayı; boş hücre için 0") })
                    Button(onClick = {
                        val cells = text.trim().split(Regex("\\s+"))
                        result = if (cells.size != 36 || cells.any { !it.matches(Regex("[0-6]")) }) "0 ile 6 arasında 36 sayı girin."
                        else if (validateBoard(cells.map { it.toByte() }.toByteArray(), 6u)) "Çakışma yok." else "Tahtada çakışma var."
                    }) { Text("Tahtayı denetle") }
                    Text(result)
                }
            }
        }
    }
}
