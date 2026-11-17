import Foundation
import Observation

/// Read-only view of the core's version-1 save JSON, for rendering only.
struct BoardState: Decodable, Equatable {
    struct Cell: Decodable, Equatable { let value: Int; let notes: Int }
    struct Edit: Decodable, Equatable {}
    let size: Int
    let givens: [Int]
    let cells: [Cell]
    let undo: [Edit]
    let redo: [Edit]

    init(json: String) throws { self = try JSONDecoder().decode(Self.self, from: Data(json.utf8)) }

    func notes(_ index: Int) -> [Int] { (1...size).filter { cells[index].notes & (1 << ($0 - 1)) != 0 } }
    func isGiven(_ index: Int) -> Bool { givens[index] != 0 }
    /// 6×6 boxes are 2 rows × 3 columns; 9×9 boxes are 3 × 3.
    var boxRows: Int { size == 6 ? 2 : 3 }
}

/// A placement the player can make now (`solo_technique_hint`): a single, preceded by the candidate
/// eliminations (locked candidates, pairs) that justify it when singles alone do not.
struct LogicalHint: Decodable, Equatable {
    struct Unit: Decodable, Equatable { let kind: String; let index: Int }
    struct Elimination: Decodable, Equatable {
        let technique: String
        let digits: [Int]
        let unit: Unit
        let target: Unit?
        let cells: [Int]
    }
    let index: Int
    let value: Int
    let technique: String
    let unit: Unit?
    /// Absent from the singles-only `solo_hint` answer.
    var eliminations: [Elimination]? = nil

    /// Cells that form the elimination patterns, highlighted on the board while the hint is open.
    var patternCells: Set<Int> { Set((eliminations ?? []).flatMap(\.cells)) }
}

enum HintOutcome: Equatable {
    case hint(LogicalHint)
    case noneWithSingles
    case conflicts
    case unsolvable
    case searchLimit
}

enum SaveState: Equatable { case saved, failed }

/// The open solo game. Every rule decision goes through the Rust core;
/// this type owns the timer, selection, and durable persistence.
@Observable
final class SoloSession {
    static func blanks(for size: Int) -> UInt8 { size == 6 ? 18 : 43 }

    private(set) var store: SoloStore
    private(set) var game: String
    private(set) var board: BoardState
    private(set) var conflicts: Set<Int> = []
    private(set) var complete = false
    private(set) var paused: Bool
    private(set) var saveState: SaveState = .saved
    /// The puzzle's level as rated by the engine from its givens ("beginner"…"expert"); nil for older,
    /// unrated puzzles, which then simply show no level.
    let level: String?
    var selected: Int
    var notesMode: Bool

    private var elapsedBase: Duration
    private var runningSince: ContinuousClock.Instant?
    @ObservationIgnored private let now: () -> ContinuousClock.Instant

    var size: Int { board.size }

    private init(store: SoloStore, game: String, elapsed: Duration, paused: Bool,
                 selected: Int, notesMode: Bool, now: @escaping () -> ContinuousClock.Instant) throws {
        self.store = store
        self.game = game
        self.board = try BoardState(json: game)
        self.level = Self.rate(game)
        self.elapsedBase = elapsed
        self.paused = paused
        self.selected = selected
        self.notesMode = notesMode
        self.now = now
        try refresh()
        if !paused && !complete { runningSince = now() }
    }

    /// Generates a fresh puzzle. The caller has already obtained confirmation
    /// and archived any unreadable save for this size.
    static func start(size: Int, seed: UInt32, store: SoloStore,
                      now: @escaping () -> ContinuousClock.Instant = { .now }) throws -> SoloSession {
        let game = try newGeneratedSolo(size: UInt8(size), seed: seed, blanks: blanks(for: size))
        let first = try BoardState(json: game).givens.firstIndex(of: 0) ?? 0
        let session = try SoloSession(store: store, game: game, elapsed: .zero, paused: false,
                                      selected: first, notesMode: false, now: now)
        session.save()
        return session
    }

    /// The level of a saved game's puzzle, or nil when it is unrated or cannot be read.
    static func rate(_ game: String) -> String? {
        struct Rating: Decodable { let level: String? }
        guard let json = try? soloDifficulty(saved: game) else { return nil }
        return (try? JSONDecoder().decode(Rating.self, from: Data(json.utf8)))?.level
    }

    /// Digs a puzzle that needs exactly the techniques of `difficulty`. Synchronous and CPU-bound (it can
    /// take a moment for the harder 9×9 levels), so callers run it off the main actor. A seed that cannot
    /// reach the level is retried with the next seed, as on the web.
    static func generate(size: Int, seed: UInt32, difficulty: String) throws -> String {
        var lastError: Error = SoloError.InvalidInput(reason: "CannotReachDifficulty")
        for attempt in 0..<3 {
            do { return try newRatedSolo(size: UInt8(size), seed: seed &+ UInt32(attempt), difficulty: difficulty) }
            catch let SoloError.InvalidInput(reason) where reason.contains("CannotReachDifficulty") {
                lastError = SoloError.InvalidInput(reason: reason)
            }
        }
        throw lastError
    }

    /// Opens a freshly generated game and saves it. The caller has already obtained confirmation and
    /// archived any unreadable save for this size.
    static func begin(game: String, store: SoloStore,
                      now: @escaping () -> ContinuousClock.Instant = { .now }) throws -> SoloSession {
        let first = try BoardState(json: game).givens.firstIndex(of: 0) ?? 0
        let session = try SoloSession(store: store, game: game, elapsed: .zero, paused: false,
                                      selected: first, notesMode: false, now: now)
        session.save()
        return session
    }

    static func restore(_ envelope: SoloEnvelope, store: SoloStore,
                        now: @escaping () -> ContinuousClock.Instant = { .now }) throws -> SoloSession {
        try SoloSession(store: store, game: envelope.game, elapsed: .milliseconds(envelope.elapsedMs),
                        paused: envelope.paused, selected: envelope.selected,
                        notesMode: envelope.notesMode, now: now)
    }

    var elapsed: Duration { elapsedBase + (runningSince.map { now() - $0 } ?? .zero) }
    var elapsedMs: Int {
        let (seconds, atto) = elapsed.components
        return Int(seconds) * 1000 + Int(atto / 1_000_000_000_000_000)
    }
    var canEdit: Bool { !paused && !complete && !board.isGiven(selected) }

    private func refresh() throws {
        board = try BoardState(json: game)
        conflicts = Set(try soloConflicts(saved: game).map(Int.init))
        complete = try soloIsComplete(saved: game)
    }

    private func stopClock() {
        elapsedBase = elapsed
        runningSince = nil
    }

    func pause() {
        guard !paused else { save(); return }
        stopClock()
        paused = true
        save()
    }

    func resume() {
        guard paused, !complete else { return }
        paused = false
        runningSince = now()
    }

    /// Applies a tagged core command. Returns a core error reason, or nil.
    @discardableResult
    func apply(_ command: String) -> String? {
        guard !paused, !complete else { return "NotPlaying" }
        let next: String
        do { next = try updateSolo(saved: game, command: command) }
        catch let SoloError.InvalidInput(reason) { return reason }
        catch { return "\(error)" }
        let changed = next != game
        game = next
        do { try refresh() } catch { return "\(error)" }
        if complete { stopClock() }
        if changed || complete { save() }
        return nil
    }

    func enter(_ digit: Int) -> String? {
        apply(notesMode
              ? #"{"type":"toggle_note","index":\#(selected),"value":\#(digit)}"#
              : #"{"type":"set","index":\#(selected),"value":\#(digit)}"#)
    }
    func erase() -> String? { apply(#"{"type":"erase","index":\#(selected)}"#) }
    func undo() -> String? { apply(#"{"type":"undo"}"#) }
    func redo() -> String? { apply(#"{"type":"redo"}"#) }

    /// Advisory only: the core never mutates state for a hint.
    func hint() -> HintOutcome {
        if !conflicts.isEmpty { return .conflicts }
        do {
            let json = try soloTechniqueHint(saved: game)
            guard let hint = try JSONDecoder().decode(LogicalHint?.self, from: Data(json.utf8)) else {
                return .noneWithSingles
            }
            return .hint(hint)
        } catch let SoloError.InvalidInput(reason) where reason == "SearchLimit" {
            return .searchLimit
        } catch {
            return .unsolvable
        }
    }

    /// An applied hint is an ordinary, undoable set.
    func applyHint(_ hint: LogicalHint) -> String? {
        selected = hint.index
        return apply(#"{"type":"set","index":\#(hint.index),"value":\#(hint.value)}"#)
    }

    func move(rowDelta: Int, columnDelta: Int) {
        let n = size
        let row = min(max(selected / n + rowDelta, 0), n - 1)
        let column = min(max(selected % n + columnDelta, 0), n - 1)
        selected = row * n + column
    }

    var envelope: SoloEnvelope {
        SoloEnvelope(game: game, elapsedMs: elapsedMs, paused: paused || complete,
                     selected: selected, notesMode: notesMode)
    }

    @discardableResult
    func save() -> Bool {
        do {
            try store.write(envelope, size: size)
            saveState = .saved
            return true
        } catch {
            // Atomic write: the previous file is intact and the live board is kept.
            saveState = .failed
            return false
        }
    }

    #if DEBUG
    func setFailWrites(_ fail: Bool) { store.failWrites = fail }
    #endif
}
