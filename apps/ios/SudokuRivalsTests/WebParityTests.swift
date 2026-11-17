import Foundation
import Testing
@testable import SudokuRivals

/// The iOS side of the web parity work: five rated levels from the shared core, technique hints that
/// finish a rated puzzle, remembered preferences, and the synthesized feedback sounds.
@Suite struct WebParityTests {
    @Test(arguments: [6, 9], AppPreferences.levels)
    func ratedPuzzleHasRequestedLevel(size: Int, level: String) throws {
        let game = try SoloSession.generate(size: size, seed: 42, difficulty: level)
        #expect(SoloSession.rate(game) == level)
        let session = try SoloSession.begin(game: game, store: tempStore())
        #expect(session.level == level)
        #expect(session.size == size)
        #expect(!session.board.isGiven(session.selected)) // opens on the first empty cell
    }

    @Test func unknownLevelIsRejected() {
        #expect(throws: (any Error).self) { try SoloSession.generate(size: 9, seed: 1, difficulty: "master") }
    }

    @Test(arguments: ["hard", "expert"])
    func techniqueHintsFinishHarderPuzzles(level: String) throws {
        let session = try SoloSession.begin(game: try SoloSession.generate(size: 9, seed: 7, difficulty: level), store: tempStore())
        var sawElimination = false
        while !session.complete {
            guard case .hint(let hint) = session.hint() else { Issue.record("technique hints stalled"); return }
            if !(hint.eliminations ?? []).isEmpty { sawElimination = true }
            #expect(session.applyHint(hint) == nil)
        }
        #expect(sawElimination, "a \(level) puzzle needs at least one elimination step")
    }

    @Test func unratedSavesShowNoLevel() throws {
        let session = try SoloSession.start(size: 9, seed: 1, store: tempStore())
        // Blank-count puzzles may happen to rate; older saves must simply never crash the label.
        #expect(session.level == nil || AppPreferences.levels.contains(session.level!))
    }

    @MainActor @Test func preferencesDefaultToNineEasyAndPersist() throws {
        let suite = "sudoku-rivals-tests-\(UUID().uuidString)"
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let prefs = AppPreferences(defaults: defaults)
        #expect(prefs.soloSize == 9 && prefs.liveSize == 9)
        #expect(prefs.soloDifficulty == "easy" && prefs.liveDifficulty == "easy")
        #expect(prefs.liveMode == "race" && prefs.appearance == .system)
        #expect(!prefs.sound && prefs.haptics)
        prefs.soloSize = 6
        prefs.soloDifficulty = "expert"
        prefs.appearance = .dark
        prefs.sound = true
        let again = AppPreferences(defaults: defaults)
        #expect(again.soloSize == 6 && again.soloDifficulty == "expert" && again.appearance == .dark && again.sound)
    }

    @MainActor @Test func feedbackToneIsAValidWave() {
        let data = Feedback.wave([(880, 0.035)])
        #expect(data.prefix(4) == Data("RIFF".utf8))
        #expect(data.count == 44 + Int(0.035 * 44_100) * 2)
    }
}
