import Foundation

/// TR/EN texts. Keys and templates follow docs/ui-contract.json where it has
/// them; the rest match the web client's wording.
enum L10n {
    static let turkish = Locale.preferredLanguages.first?.hasPrefix("tr") ?? false

    static func t(_ key: String, _ args: [String: CustomStringConvertible] = [:]) -> String {
        var text = (turkish ? tr[key] : en[key]) ?? key
        for (name, value) in args { text = text.replacingOccurrences(of: "{\(name)}", with: value.description) }
        return text
    }

    static func time(_ ms: Int) -> String {
        let seconds = ms / 1000, hours = seconds / 3600, minutes = seconds / 60 % 60, rest = seconds % 60
        return hours > 0 ? String(format: "%d:%02d:%02d", hours, minutes, rest) : String(format: "%d:%02d", minutes, rest)
    }

    private static let tr: [String: String] = [
        "app.title": "Sudoku Rivals",
        "home.solo": "Tek başına oyna", "home.soloDetail": "6×6 veya 9×9. İnternetsiz de oynanır.",
        "home.friends": "Arkadaşınla yarış", "home.friendsPending": "Arkadaş maçları bu sürümde henüz hazır değil.",
        "home.resume": "Oyuna devam et", "home.continue": "Kaldığın yerden",
        "home.savedGame": "Kayıtlı oyun", "home.completed": "Tamamlandı",
        "storage.corrupt": "{size} kaydı açılamadı. Kayıt korunuyor; tekrar deneyebilir veya yeni oyun başlatabilirsin.",
        "storage.unsupported": "{size} kaydı bu sürümün açamadığı bir biçimde. Kayıt değiştirilmeden korunuyor.",
        "storage.unavailable": "{size} kaydına şu an erişilemiyor. Kayıt korunuyor; tekrar deneyebilirsin.",
        "setup.title": "Tahtanı seç", "setup.size": "Tahta boyutu", "setup.start": "Oyunu başlat",
        "setup.six": "Daha küçük tahta, 2×3 kutular", "setup.nine": "Klasik Sudoku, 3×3 kutular",
        "setup.generator": "Her oyun tek çözümlü yeni bir bulmacayla başlar ve ağ gerektirmez.",
        "setup.difficulty": "Zorluk seçimi yok: boş hücre sayısı (6×6: 18, 9×9: 43) zorluk derecesi değildir ve insan zorluk seviyesi henüz kalibre edilmedi.",
        "setup.hasSave": "Bu boyutta kayıtlı bir oyunun var. Yeni oyun onun yerini alır.",
        "setup.hasUnreadable": "Bu boyuttaki kayıt açılamıyor. Yeni oyun başlatırsan o kayıt silinmez, ayrı bir dosyaya taşınır.",
        "setup.failed": "Oyun başlatılamadı. Mevcut kayıtların değişmedi.",
        "setup.archiveFailed": "Açılamayan kayıt güvenle taşınamadı; üzerine yazılmadı. Yeni oyun başlatılmadı.",
        "confirm.replace": "Yeni oyun bu boyuttaki kaydının yerini alacak. Devam edilsin mi?",
        "confirm.replaceUnreadable": "Açılamayan kayıt ayrı bir dosyaya taşınacak ve yeni oyun başlayacak. Devam edilsin mi?",
        "confirm.continue": "Yeni oyun başlat", "confirm.cancel": "Vazgeç",
        "mode.solo": "Tek başına",
        "game.pause": "Duraklat", "game.resume": "Devam et", "game.exit": "Kaydedip çık",
        "game.discardExit": "Kaydetmeden çık",
        "game.confirmDiscard": "Kaydedilemeyen değişiklikler kaybolacak. Önceki kayıt korunacak. Yine de çıkılsın mı?",
        "game.erase": "Sil", "game.notes": "Kalem", "game.undo": "Geri al", "game.redo": "Yinele", "game.hint": "İpucu",
        "game.elapsed": "Geçen süre: {time}",
        "game.conflict": "Tahtada çakışma var", "game.incomplete": "Oyun devam ediyor", "game.complete": "Sudoku tamamlandı",
        "game.given": "verilen", "game.editable": "girilebilir", "game.empty": "boş", "game.noNotes": "yok",
        "cell.label": "Satır {row}, sütun {column}, {kind}, {value}, notlar: {notes}{conflict}",
        "cell.conflict": ", çakışma var",
        "digit.label": "{digit} yaz", "numbers": "Sayılar",
        "selection": "{row}. satır · {column}. sütun", "selection.given": "Verilen hücre · değiştirilemez",
        "saved": "Bu cihazda kaydedildi",
        "error.save": "Oyun kaydedilemedi. Açık oyunun korunuyor; tekrar deneyebilirsin.", "error.retry": "Tekrar dene",
        "error.given": "Verilen hücreler değiştirilemez.", "error.filled": "Not eklemek için önce hücredeki sayıyı sil.",
        "error.generic": "Bu işlem tamamlanamadı. Açık oyunun korunuyor.",
        "hint.naked": "{row}. satır, {column}. sütunda yalnızca {value} kullanılabilir. Aynı satır, sütun ve kutudaki sayılar diğer adayları eliyor.",
        "hint.hidden": "{unit} içinde {value} yalnızca {row}. satır, {column}. sütuna yerleşebilir.",
        "hint.row": "{number}. satır", "hint.column": "{number}. sütun", "hint.box": "{number}. kutu",
        "hint.apply": "Bu sayıyı yaz", "hint.dismiss": "Kapat",
        "hint.none": "Bu konumda tekli tekniklerle açıklanabilir bir ipucu bulunamadı.",
        "hint.conflicts": "İpucu için önce tahtadaki çakışmaları düzelt.",
        "hint.unsolvable": "Bu tahta için açıklanabilir ipucu bulunamadı.",
        "hint.searchLimit": "İpucu araması sınırına ulaşıldı; tahta çözümsüz sayılmadı.",
        "paused.title": "Kısa bir mola.", "paused.detail": "Tahtan ve notların burada. Hazır olduğunda devam et.",
        "result.title": "Tahta tamamlandı.", "result.detail": "Her satır, sütun ve kutu yerli yerinde.",
        "result.newGame": "Yeni oyun", "nav.home": "Ana sayfa", "nav.back": "Geri",
        "keyboard": "Klavye: oklar · 1–{size} · N kalem · Delete sil · ⌘Z geri al · ⇧⌘Z yinele",
    ]

    private static let en: [String: String] = [
        "app.title": "Sudoku Rivals",
        "home.solo": "Play solo", "home.soloDetail": "6×6 or 9×9. Works offline.",
        "home.friends": "Play with a friend", "home.friendsPending": "Friend matches are not available in this version yet.",
        "home.resume": "Resume game", "home.continue": "Pick up where you left off",
        "home.savedGame": "Saved game", "home.completed": "Completed",
        "storage.corrupt": "The {size} save could not be opened. It is kept; you can retry or start a new game.",
        "storage.unsupported": "The {size} save uses a format this version cannot open. It is kept unchanged.",
        "storage.unavailable": "The {size} save cannot be accessed right now. It is kept; you can retry.",
        "setup.title": "Choose your board", "setup.size": "Board size", "setup.start": "Start game",
        "setup.six": "Smaller board, 2×3 boxes", "setup.nine": "Classic Sudoku, 3×3 boxes",
        "setup.generator": "Every game starts from a new puzzle with one solution, no network needed.",
        "setup.difficulty": "No difficulty choice: the blank count (6×6: 18, 9×9: 43) is not a difficulty rating, and human difficulty is not calibrated yet.",
        "setup.hasSave": "You have a saved game of this size. A new game replaces it.",
        "setup.hasUnreadable": "The save for this size cannot be opened. Starting a new game moves it to a separate file instead of deleting it.",
        "setup.failed": "Could not start the game. Your saves are unchanged.",
        "setup.archiveFailed": "The unreadable save could not be moved safely, so it was not overwritten. No new game was started.",
        "confirm.replace": "A new game will replace the saved game of this size. Continue?",
        "confirm.replaceUnreadable": "The unreadable save will be moved to a separate file and a new game will start. Continue?",
        "confirm.continue": "Start new game", "confirm.cancel": "Cancel",
        "mode.solo": "Solo",
        "game.pause": "Pause", "game.resume": "Resume", "game.exit": "Save and exit",
        "game.discardExit": "Exit without saving",
        "game.confirmDiscard": "Unsaved changes will be lost. The previous save will remain. Exit anyway?",
        "game.erase": "Erase", "game.notes": "Notes", "game.undo": "Undo", "game.redo": "Redo", "game.hint": "Hint",
        "game.elapsed": "Elapsed: {time}",
        "game.conflict": "The board has conflicts", "game.incomplete": "Game in progress", "game.complete": "Sudoku complete",
        "game.given": "given", "game.editable": "editable", "game.empty": "empty", "game.noNotes": "none",
        "cell.label": "Row {row}, column {column}, {kind}, {value}, notes: {notes}{conflict}",
        "cell.conflict": ", conflict",
        "digit.label": "Enter {digit}", "numbers": "Numbers",
        "selection": "Row {row} · column {column}", "selection.given": "Given cell · cannot change",
        "saved": "Saved on this device",
        "error.save": "Could not save. Your open game is preserved; you can retry.", "error.retry": "Retry",
        "error.given": "Given cells cannot be changed.", "error.filled": "Erase the number before adding notes.",
        "error.generic": "That action could not be completed. Your open game is preserved.",
        "hint.naked": "Only {value} can go in row {row}, column {column}. The numbers in its row, column and box rule out the other candidates.",
        "hint.hidden": "In {unit}, {value} can only go in row {row}, column {column}.",
        "hint.row": "row {number}", "hint.column": "column {number}", "hint.box": "box {number}",
        "hint.apply": "Enter this number", "hint.dismiss": "Dismiss",
        "hint.none": "No explainable hint using singles was found here.",
        "hint.conflicts": "Resolve the board conflicts before asking for a hint.",
        "hint.unsolvable": "No explainable hint was found for this board.",
        "hint.searchLimit": "The hint search hit its limit; the board was not judged unsolvable.",
        "paused.title": "Take your time.", "paused.detail": "Your board and notes are here. Continue whenever you are ready.",
        "result.title": "Board complete.", "result.detail": "Every row, column and box is in place.",
        "result.newGame": "New game", "nav.home": "Home", "nav.back": "Back",
        "keyboard": "Keyboard: arrows · 1–{size} · N notes · Delete erase · ⌘Z undo · ⇧⌘Z redo",
    ]
}
