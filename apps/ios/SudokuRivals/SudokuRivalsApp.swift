import Foundation
import SwiftUI

@main
struct SudokuRivalsApp: App {
    var body: some Scene { WindowGroup { ContentView() } }
}

struct ContentView: View {
    @State private var board = "1 0 3 4 5 6\n4 0 6 1 2 3\n0 3 0 5 6 1\n5 6 0 2 0 4\n3 0 5 6 0 2\n0 1 0 0 4 0"
    @State private var result = ""
    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Sudoku Rivals").font(.largeTitle)
            Text("Ortak çekirdek doğrulama denemesi (6×6)")
            Text("36 sayı; boş hücre için 0")
            TextEditor(text: $board).frame(height: 180).border(.secondary)
            Button("Tahtayı denetle") {
                let parts = board.split(whereSeparator: \.isWhitespace)
                let values = parts.compactMap { UInt8($0) }
                if parts.count != 36 || values.count != 36 || values.contains(where: { $0 > 6 }) {
                    result = "0 ile 6 arasında 36 sayı girin."
                } else {
                    result = validateBoard(cells: Data(values), size: 6) ? "Çakışma yok." : "Tahtada çakışma var."
                }
            }
            Text(result).accessibilityLabel(result)
            Spacer()
        }.padding()
    }
}
