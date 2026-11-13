import Foundation
import Testing
@testable import SudokuRivals

/// Exercises the app's session/store against the real Rust core (UniFFI).
final class FakeClock: @unchecked Sendable {
    var now = ContinuousClock.now
    func advance(_ ms: Int) { now = now.advanced(by: .milliseconds(ms)) }
}

func tempStore() -> SoloStore {
    SoloStore(directory: FileManager.default.temporaryDirectory.appending(path: "solo-tests-\(UUID().uuidString)"))
}

/// Seeds whose generated puzzles the core's singles hints can finish; shared with UI tests.
let singlesSeed: [Int: UInt32] = [6: 7, 9: 7]

/// Plays only advisory hints applied as ordinary sets. Returns false if singles stall.
@discardableResult
func solveWithHints(_ session: SoloSession) -> Bool {
    while !session.complete {
        guard case .hint(let hint) = session.hint() else { return false }
        #expect(session.applyHint(hint) == nil)
    }
    return true
}

@Suite struct SoloSessionTests {
    @Test(arguments: [6, 9])
    func generatedGameHasFixedGivensAndExpectedBlanks(size: Int) throws {
        let store = tempStore()
        let session = try SoloSession.start(size: size, seed: 1234, store: store)
        #expect(session.size == size)
        #expect(session.board.givens.filter { $0 == 0 }.count == Int(SoloSession.blanks(for: size)))
        #expect(session.saveState == .saved)
        #expect(session.conflicts.isEmpty)

        let given = try #require(session.board.givens.firstIndex { $0 != 0 })
        let before = session.game
        session.selected = given
        #expect(!session.canEdit)
        #expect(session.enter(1) == "GivenCell")
        #expect(session.erase() == "GivenCell")
        #expect(session.game == before)
        #expect(session.apply(#"{"type":"set","index":0,"value":1,"extra":1}"#) == "InvalidCommand")
        #expect(session.game == before)
    }

    @Test(arguments: [6, 9])
    func notesEraseUndoRedoFollowCoreRules(size: Int) throws {
        let session = try SoloSession.start(size: size, seed: 99, store: tempStore())
        let blank = try #require(session.board.givens.firstIndex(of: 0))
        session.selected = blank
        session.notesMode = true
        #expect(session.enter(1) == nil)
        #expect(session.enter(size) == nil)
        #expect(session.board.notes(blank) == [1, size])
        #expect(session.enter(1) == nil) // toggles off
        #expect(session.board.notes(blank) == [size])

        session.notesMode = false
        #expect(session.enter(2) == nil)
        #expect(session.board.cells[blank].value == 2)
        #expect(session.board.notes(blank).isEmpty) // digit clears that cell's notes only
        session.notesMode = true
        #expect(session.enter(3) == "FilledCell")

        #expect(session.undo() == nil)
        #expect(session.board.cells[blank].value == 0)
        #expect(session.board.notes(blank) == [size])
        #expect(session.redo() == nil)
        #expect(session.board.cells[blank].value == 2)
        #expect(session.erase() == nil)
        #expect(session.board.cells[blank] == .init(value: 0, notes: 0))
        #expect(session.board.redo.isEmpty)
        #expect(session.undo() == nil)
        #expect(session.board.cells[blank].value == 2)
        #expect(session.board.redo.count == 1)
    }

    @Test func visibleConflictsUseZeroBasedIndices() throws {
        let session = try SoloSession.start(size: 6, seed: 5, store: tempStore())
        let board = session.board
        // Find a blank cell and a digit already given in its row.
        let (blank, digit) = try #require((0..<36).lazy.compactMap { i -> (Int, Int)? in
            guard board.givens[i] == 0 else { return nil }
            let row = (0..<6).map { board.givens[i / 6 * 6 + $0] }
            return row.first { $0 != 0 }.map { (i, $0) }
        }.first)
        let peer = try #require((0..<6).map { blank / 6 * 6 + $0 }.first { board.givens[$0] == digit })
        session.selected = blank
        #expect(session.enter(digit) == nil)
        #expect(session.conflicts == [blank, peer])
        #expect(session.board.cells[blank].value == digit) // conflicting player digit is kept
        #expect(session.hint() == .conflicts)
        #expect(session.undo() == nil)
        #expect(session.conflicts.isEmpty)
    }

    @Test(arguments: [6, 9])
    func hintIsAdvisoryAndApplyIsUndoable(size: Int) throws {
        let session = try SoloSession.start(size: size, seed: singlesSeed[size]!, store: tempStore())
        let before = session.game
        guard case .hint(let hint) = session.hint() else { Issue.record("no hint"); return }
        #expect(session.game == before) // asking never mutates
        #expect(["naked_single", "hidden_single"].contains(hint.technique))
        #expect(session.board.givens[hint.index] == 0)
        #expect(session.applyHint(hint) == nil)
        #expect(session.board.cells[hint.index].value == hint.value)
        #expect(session.selected == hint.index)
        #expect(session.undo() == nil)
        #expect(session.game != before) // history differs (redo entry)...
        #expect(session.board.cells == (try BoardState(json: before)).cells) // ...but cells are restored
    }

    @Test(arguments: [6, 9])
    func completionStopsClockAndSurvivesRestore(size: Int) throws {
        let clock = FakeClock()
        let store = tempStore()
        let session = try SoloSession.start(size: size, seed: singlesSeed[size]!, store: store, now: { clock.now })
        clock.advance(5_000)
        #expect(solveWithHints(session))
        #expect(session.complete)
        let finished = session.elapsedMs
        #expect(finished == 5_000)
        clock.advance(60_000)
        #expect(session.elapsedMs == finished)
        #expect(session.saveState == .saved)

        guard case .ready(let envelope, let complete) = store.read(size) else { Issue.record("not saved"); return }
        #expect(complete)
        #expect(envelope.elapsedMs == finished)
        let restored = try SoloSession.restore(envelope, store: store)
        #expect(restored.complete)
        #expect(restored.game == session.game)
    }

    @Test func timerStopsWhilePausedAndResumes() throws {
        let clock = FakeClock()
        let session = try SoloSession.start(size: 6, seed: 3, store: tempStore(), now: { clock.now })
        clock.advance(1_500)
        #expect(session.elapsedMs == 1_500)
        session.pause()
        clock.advance(10_000)
        #expect(session.elapsedMs == 1_500)
        #expect(session.enter(1) == "NotPlaying")
        session.resume()
        clock.advance(500)
        #expect(session.elapsedMs == 2_000)
    }

    @Test func coldRestoreKeepsBoardNotesHistoryAndMetadata() throws {
        let clock = FakeClock()
        let store = tempStore()
        let session = try SoloSession.start(size: 9, seed: 11, store: store, now: { clock.now })
        let blanks = session.board.givens.indices.filter { session.board.givens[$0] == 0 }
        session.selected = blanks[0]
        #expect(session.enter(4) == nil)
        session.selected = blanks[1]
        session.notesMode = true
        #expect(session.enter(7) == nil)
        clock.advance(3_210)
        session.pause()

        guard case .ready(let envelope, _) = store.read(9) else { Issue.record("not saved"); return }
        let restored = try SoloSession.restore(envelope, store: store, now: { clock.now })
        #expect(restored.game == session.game)
        #expect(restored.elapsedMs == 3_210)
        #expect(restored.selected == blanks[1])
        #expect(restored.notesMode)
        #expect(restored.paused)
        #expect(restored.board.notes(blanks[1]) == [7])
        #expect(restored.board.undo.count == 2)
        #expect(store.read(6) == .missing) // sizes are independent
    }

    @Test func sizesAreIndependentSlots() throws {
        let store = tempStore()
        let six = try SoloSession.start(size: 6, seed: 1, store: store)
        let nine = try SoloSession.start(size: 9, seed: 1, store: store)
        let nineBytes = try Data(contentsOf: store.url(9))
        six.selected = try #require(six.board.givens.firstIndex(of: 0))
        #expect(six.enter(1) == nil)
        #expect(try Data(contentsOf: store.url(9)) == nineBytes)
        guard case .ready(let saved6, _) = store.read(6), case .ready(let saved9, _) = store.read(9) else {
            Issue.record("missing slot"); return
        }
        #expect(saved6.game == six.game)
        #expect(saved9.game == nine.game)
    }

    @Test func failedWriteKeepsPreviousDiskAndLiveBoardThenRetries() throws {
        let store = tempStore()
        let session = try SoloSession.start(size: 6, seed: 21, store: store)
        let onDisk = try Data(contentsOf: store.url(6))
        session.setFailWrites(true)
        session.selected = try #require(session.board.givens.firstIndex(of: 0))
        #expect(session.enter(1) == nil)
        #expect(session.saveState == .failed)
        #expect(session.board.cells[session.selected].value == 1) // live board kept
        #expect(try Data(contentsOf: store.url(6)) == onDisk) // previous save intact
        #expect(!session.save())
        session.setFailWrites(false)
        #expect(session.save())
        #expect(session.saveState == .saved)
        guard case .ready(let envelope, _) = store.read(6) else { Issue.record("not saved"); return }
        #expect(envelope.game == session.game)
    }

    @Test func corruptAndFutureSavesAreReportedAndNeverRewritten() throws {
        let store = tempStore()
        try FileManager.default.createDirectory(at: store.directory, withIntermediateDirectories: true)
        let garbage = Data(#"{"version":1,"game":"oops"#.utf8)
        try garbage.write(to: store.url(6))
        #expect(store.read(6) == .corrupt)
        #expect(try Data(contentsOf: store.url(6)) == garbage)

        let future = Data(#"{"version":7,"whatever":true}"#.utf8)
        try future.write(to: store.url(9))
        #expect(store.read(9) == .unsupportedVersion)

        // A valid envelope wrapping a future core save is also a version problem, not corruption.
        let game = try newGeneratedSolo(size: 9, seed: 1, blanks: 43).replacingOccurrences(of: #""version":1"#, with: #""version":2"#)
        let wrapped = try JSONEncoder().encode(SoloEnvelope(game: game, elapsedMs: 0, paused: true, selected: 0, notesMode: false))
        try wrapped.write(to: store.url(9))
        #expect(store.read(9) == .unsupportedVersion)

        // Wrong size inside a slot, or a history that does not replay, is corrupt.
        let six = try newGeneratedSolo(size: 6, seed: 1, blanks: 18)
        try JSONEncoder().encode(SoloEnvelope(game: six, elapsedMs: 0, paused: true, selected: 0, notesMode: false))
            .write(to: store.url(9))
        #expect(store.read(9) == .corrupt)

        // Confirmed new game: the unreadable file is moved aside byte-for-byte first.
        let archived = try store.archiveUnreadable(6)
        #expect(try Data(contentsOf: archived) == garbage)
        #expect(store.read(6) == .missing)
    }

    @Test func archiveFailureIsReported() {
        let store = tempStore()
        #expect(throws: (any Error).self) { try store.archiveUnreadable(6) }
    }
}
