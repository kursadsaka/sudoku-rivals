import Foundation
import Testing
@testable import SudokuRivals

func tempLiveStore() -> LiveStore {
    LiveStore(directory: FileManager.default.temporaryDirectory.appending(path: "live-tests-\(UUID().uuidString)"))
}

/// Records every request the adapter sends and can fail or drop them on demand.
@MainActor
final class FaultTransport {
    struct Call { let path: String; let body: String }
    var calls: [Call] = []
    var offline = false
    /// Forwards the request but loses the response once, like a dropped connection.
    var dropNextCommand: String?
    let upstream: LiveSession.Transport?

    init(upstream: LiveSession.Transport? = nil) { self.upstream = upstream }

    var transport: LiveSession.Transport {
        { @MainActor request in
            let body = String(decoding: request.httpBody ?? Data(), as: UTF8.self)
            if self.offline { throw URLError(.notConnectedToInternet) }
            self.calls.append(Call(path: request.url?.path ?? "", body: body))
            guard let upstream = self.upstream else { throw URLError(.cannotConnectToHost) }
            let result = try await upstream(request)
            if let drop = self.dropNextCommand, body.contains(#""type":"\#(drop)""#) {
                self.dropNextCommand = nil
                throw URLError(.networkConnectionLost)
            }
            return result
        }
    }

    func commands(_ type: String) -> [Call] { calls.filter { $0.body.contains(#""command":{"#) && $0.body.contains(#""type":"\#(type)""#) } }
}

@MainActor
final class ManualClock {
    var ms: UInt64 = 1_000
    var now: () -> UInt64 { { self.ms } }
}

// MARK: Hermetic adapter behaviour

@MainActor
@Suite struct LiveAdapterTests {
    @Test func originsAllowOnlyHTTPSOrDebugLoopback() {
        #expect(LiveSession.normalize("https://play.example.com/", allowLoopbackHTTP: false) == "https://play.example.com")
        #expect(LiveSession.normalize("https://play.example.com:8443", allowLoopbackHTTP: false) == "https://play.example.com:8443")
        #expect(LiveSession.normalize("http://127.0.0.1:3000", allowLoopbackHTTP: true) == "http://127.0.0.1:3000")
        for bad in ["http://127.0.0.1:3000", "http://example.com", "https://user:pw@example.com", "https://example.com/api",
                    "https://example.com?x=1", "https://example.com#f", "ftp://example.com", "", "example.com"] {
            #expect(LiveSession.normalize(bad, allowLoopbackHTTP: false) == nil, "\(bad)")
        }
        #expect(LiveSession.normalize("http://192.168.1.2:3000", allowLoopbackHTTP: true) == nil)
    }

    @Test func guestSecretsAreRandom256Bit() {
        let a = LiveSession.secureGuest(), b = LiveSession.secureGuest()
        #expect(a.count == 64 && b.count == 64 && a != b)
        #expect(a.allSatisfy { $0.isHexDigit })
    }

    @Test func noServerMeansNotConfiguredAndNoRequest() {
        let fault = FaultTransport()
        let live = LiveSession(store: tempLiveStore(), origin: nil, transport: fault.transport)
        live.open()
        live.create()
        #expect(!live.isConfigured)
        #expect(live.message == L10n.t("live.notConfigured"))
        #expect(fault.calls.isEmpty)
        #expect(!live.store.exists)
    }

    @Test func corruptRecordIsProtectedThenPreservedOnReset() throws {
        let store = tempLiveStore()
        try FileManager.default.createDirectory(at: store.directory, withIntermediateDirectories: true)
        try Data("{broken-live-record".utf8).write(to: store.url)
        let fault = FaultTransport()
        let live = LiveSession(store: store, origin: "http://127.0.0.1:9", transport: fault.transport)
        #expect(live.protectedSave)
        live.open()
        live.create()
        #expect(fault.calls.isEmpty)
        #expect(try Data(contentsOf: store.url) == Data("{broken-live-record".utf8), "unreadable record overwritten")

        live.confirmReset()
        #expect(!live.protectedSave)
        let copies = try FileManager.default.contentsOfDirectory(atPath: store.directory.path)
            .filter { $0.hasPrefix("live-session-recovery-") }
        #expect(copies.count == 1)
        #expect(try Data(contentsOf: store.directory.appending(path: copies[0])) == Data("{broken-live-record".utf8))
        #expect(try store.load().origin == "http://127.0.0.1:9")
    }

    @Test func futureOrForeignOriginRecordIsNotLoaded() throws {
        let store = tempLiveStore()
        try FileManager.default.createDirectory(at: store.directory, withIntermediateDirectories: true)
        try Data(#"{"version":2,"origin":"http://127.0.0.1:9","client":"{}"}"#.utf8).write(to: store.url)
        #expect(LiveSession(store: store, origin: "http://127.0.0.1:9").protectedSave)
        try Data(#"{"version":1,"origin":"http://evil.example","client":"{}"}"#.utf8).write(to: store.url)
        #expect(LiveSession(store: store, origin: "http://127.0.0.1:9").protectedSave)
    }

    @Test func credentialRequestWaitsForDurableSaveAndRecordIsNotBackedUp() async throws {
        var store = tempLiveStore()
        store.failWrites = true
        let fault = FaultTransport()
        let live = LiveSession(store: store, origin: "http://127.0.0.1:9", transport: fault.transport)
        live.open()
        live.create()
        #expect(live.storageError)
        try await Task.sleep(for: .milliseconds(100))
        #expect(fault.calls.isEmpty, "credential-bearing request sent before durable state")
        #expect(!store.exists)

        live.store.failWrites = false
        live.retryStorage()
        #expect(!live.storageError)
        try await waitFor { !fault.calls.isEmpty }
        #expect(fault.calls.count == 1 && fault.calls[0].path == "/v1/rooms")
        let values = try store.directory.resourceValues(forKeys: [.isExcludedFromBackupKey])
        #expect(values.isExcludedFromBackup == true)
        #expect(try store.load().version == 1)
    }

    @Test func lostCreateIsRetriedWithIdenticalBodyAfterBackoff() async throws {
        let clock = ManualClock()
        let fault = FaultTransport()
        let live = LiveSession(store: tempLiveStore(), origin: "http://127.0.0.1:9", transport: fault.transport, now: clock.now)
        live.open()
        live.create()
        try await waitFor { live.view.retryAtMs != nil }
        #expect(live.message == L10n.t("live.error.unreachable"))
        let retryAt = try #require(live.view.retryAtMs)
        live.tick()
        #expect(fault.calls.count == 1, "retried before backoff")
        clock.ms = retryAt
        live.tick()
        try await waitFor { fault.calls.count == 2 }
        #expect(fault.calls[0].body == fault.calls[1].body)
        #expect(fault.calls[0].body.contains(#""guest":""#))
    }

    @Test func backgroundInvalidatesInFlightCallback() async throws {
        let gate = Gate()
        let live = LiveSession(store: tempLiveStore(), origin: "http://127.0.0.1:9", transport: { _ in
            await gate.wait()
            return (Data(#"{"protocol":1,"error":"room_full"}"#.utf8), 409)
        })
        live.open()
        live.create()
        #expect(live.view.pendingHttp)
        live.setForeground(false)
        await gate.open()
        try await Task.sleep(for: .milliseconds(100))
        // The stale 409 never reached the core: no error surfaced, request stays retryable.
        #expect(live.view.lastError == nil)
        #expect(live.message != L10n.t("server.roomFull"))
    }
}

actor Gate {
    private var waiters: [CheckedContinuation<Void, Never>] = []
    private var isOpen = false
    func wait() async { if !isOpen { await withCheckedContinuation { waiters.append($0) } } }
    func open() { isOpen = true; waiters.forEach { $0.resume() }; waiters = [] }
}

@MainActor
func waitFor(_ timeout: Duration = .seconds(5), _ condition: () -> Bool) async throws {
    let end = ContinuousClock.now + timeout
    while !condition() {
        guard ContinuousClock.now < end else { Issue.record("condition timed out"); throw CancellationError() }
        try await Task.sleep(for: .milliseconds(20))
    }
}

// MARK: Two real adapters against a local sudoku-server

/// Set by tests/ios-multiplayer.sh (TEST_RUNNER_LIVE_TEST_SERVER); skipped otherwise.
let liveServer = ProcessInfo.processInfo.environment["LIVE_TEST_SERVER"]

/// Plain backtracking; both seats get the same puzzle, so either can solve it.
func solve(_ givens: [Int], size: Int) -> [Int]? {
    var board = givens
    let boxRows = size == 6 ? 2 : 3
    func allowed(_ index: Int, _ value: Int) -> Bool {
        let row = index / size, column = index % size
        for i in 0..<size where board[row * size + i] == value || board[i * size + column] == value { return false }
        let top = row / boxRows * boxRows, left = column / 3 * 3
        for r in top..<top + boxRows { for c in left..<left + 3 where board[r * size + c] == value { return false } }
        return true
    }
    func search() -> Bool {
        guard let index = board.firstIndex(of: 0) else { return true }
        for value in 1...size where allowed(index, value) {
            board[index] = value
            if search() { return true }
        }
        board[index] = 0
        return false
    }
    return search() ? board : nil
}

@MainActor
@Suite(.enabled(if: liveServer != nil, "needs LIVE_TEST_SERVER"), .serialized)
struct LiveServerTests {
    let origin = liveServer ?? ""

    func player(_ fault: FaultTransport) -> LiveSession {
        let live = LiveSession(store: tempLiveStore(), origin: origin, transport: fault.transport)
        live.open()
        return live
    }

    func pump(_ players: [LiveSession], timeout: Duration = .seconds(20), until condition: () -> Bool) async throws {
        let end = ContinuousClock.now + timeout
        while !condition() {
            guard ContinuousClock.now < end else { Issue.record("live condition timed out"); throw CancellationError() }
            players.forEach { $0.tick() }
            try await Task.sleep(for: .milliseconds(40))
        }
    }

    /// Host creates, guest joins by invite link, both ready, match is playing.
    func startMatch(mode: String, size: Int) async throws -> (LiveSession, LiveSession, FaultTransport, FaultTransport) {
        let hostFault = FaultTransport(upstream: LiveSession.urlSessionTransport)
        let guestFault = FaultTransport(upstream: LiveSession.urlSessionTransport)
        let host = player(hostFault), guest = player(guestFault)
        host.name = "Host"; host.mode = mode; host.size = size
        host.create()
        try await pump([host]) { host.snapshot != nil }
        guest.name = "Guest"
        guest.invite = try #require(host.inviteURL).absoluteString
        guest.join()
        try await pump([host, guest]) { guest.snapshot != nil && host.snapshot?.opponent != nil }
        host.command("ready", ready: true)
        try await pump([host, guest]) { !host.view.pendingCommand }
        guest.command("ready", ready: true)
        try await pump([host, guest]) { host.snapshot?.phase == "playing" && guest.snapshot?.phase == "playing" }
        #expect(host.snapshot?.matchId == guest.snapshot?.matchId)
        #expect(host.snapshot?.own.givens == guest.snapshot?.own.givens)
        return (host, guest, hostFault, guestFault)
    }

    @Test(arguments: [6, 9])
    func raceOfflineEditsSyncOnceAndFirstCorrectFinishWins(size: Int) async throws {
        let (host, guest, hostFault, _) = try await startMatch(mode: "race", size: size)
        let givens = try #require(host.snapshot?.own.givens)
        let solution = try #require(solve(givens, size: size))
        let blanks = givens.indices.filter { givens[$0] == 0 }

        hostFault.offline = true
        try await pump([host, guest]) { host.view.connection != "connected" }
        for index in blanks.prefix(4) {
            host.selected = index
            host.enter(solution[index])
        }
        host.notesMode = true
        host.selected = blanks[4]
        host.enter(1)
        host.notesMode = false
        #expect(host.view.race?.dirty == true)
        #expect(host.view.race?.notes[blanks[4]] == [1])
        let syncsBefore = hostFault.commands("sync").count
        hostFault.offline = false
        try await pump([host, guest]) { host.view.connection == "connected" && host.view.race?.dirty == false }
        #expect(hostFault.commands("sync").count - syncsBefore == 1, "offline batch was not one sync")
        #expect(host.view.race?.notes[blanks[4]] == [1], "notes must stay local")
        try await pump([host, guest]) { (guest.snapshot?.opponent?.progressPercent ?? 0) > 0 }

        for index in blanks.dropFirst(4) {
            host.selected = index
            host.enter(solution[index])
        }
        try await pump([host, guest]) { host.snapshot?.phase == "finished" && guest.snapshot?.phase == "finished" }
        #expect(host.snapshot?.result?.winner == host.snapshot?.own.seat)
        #expect(guest.snapshot?.result?.winner == host.snapshot?.own.seat)

        // Mutual rematch opens a new match id for both.
        let old = try #require(host.snapshot?.matchId)
        host.command("rematch")
        try await pump([host, guest]) { !host.view.pendingCommand }
        guest.command("rematch")
        try await pump([host, guest]) { host.snapshot?.matchId != old && guest.snapshot?.matchId != old }
        try await leave(host, guest)
    }

    @Test func duelLostWrongMoveRetriesExactlyThenLockThenScore() async throws {
        let (host, guest, hostFault, _) = try await startMatch(mode: "duel", size: 6)
        let givens = try #require(host.snapshot?.own.givens)
        let solution = try #require(solve(givens, size: 6))
        let blank = try #require(givens.firstIndex(of: 0))
        host.selected = blank
        hostFault.dropNextCommand = "move"
        host.enter(solution[blank] % 6 + 1)
        try await pump([host, guest]) { host.snapshot?.own.wrong == 1 && !host.view.pendingCommand }
        let moves = hostFault.commands("move")
        #expect(moves.count == 2 && moves[0].body == moves[1].body, "retry changed the command body")
        #expect(host.snapshot?.own.score == 0)
        #expect((host.view.lockedRemainingMs ?? 0) > 0)
        #expect(!host.canInput())
        try await pump([host, guest], timeout: .seconds(5)) { host.canInput() }
        host.enter(solution[blank])
        try await pump([host, guest]) { host.snapshot?.own.score == 10 && guest.snapshot?.opponent?.score == 10 }
        guest.command("resign")
        try await pump([host, guest]) { host.snapshot?.phase == "finished" }
        #expect(host.snapshot?.result?.winner == host.snapshot?.own.seat)
        #expect(host.snapshot?.result?.reason == "resigned")
        try await leave(host, guest)
    }

    @Test func backgroundThenForegroundResumesWithNewGenerationAndColdRestoreKeepsDraft() async throws {
        let (host, guest, _, _) = try await startMatch(mode: "race", size: 6)
        let givens = try #require(host.snapshot?.own.givens)
        let blank = try #require(givens.firstIndex(of: 0))
        let generation = try #require(host.snapshot?.generation)
        host.setForeground(false)
        #expect(host.view.connection != "connected")
        host.setForeground(true)
        try await pump([host, guest]) { (host.snapshot?.generation ?? 0) > generation && host.view.connection == "connected" }

        // Cold start: a second adapter restores the opaque record, keeps the draft and resumes.
        host.selected = blank
        host.edit("toggle_note", value: 2)
        host.setForeground(false)
        let restored = LiveSession(store: host.store, origin: origin)
        #expect(restored.visible)
        #expect(restored.view.race?.notes[blank] == [2])
        restored.open()
        try await pump([restored, guest]) { restored.view.connection == "connected" && (restored.snapshot?.generation ?? 0) > generation + 1 }
        #expect(restored.view.race?.notes[blank] == [2])
        try await leave(restored, guest)
    }

    @Test func viewAndMessagesNeverCarryCredentials() async throws {
        let (host, guest, _, _) = try await startMatch(mode: "duel", size: 9)
        let record = try host.store.load()
        struct Saved: Decodable { struct Auth: Decodable { let token: String; let guest: String }; let auth: Auth }
        let auth = try JSONDecoder().decode(Saved.self, from: Data(record.client.utf8)).auth
        #expect(auth.token.count >= 32 && auth.guest.count == 64)
        let client = try MultiplayerClient.restore(saved: record.client)
        let view = try client.view(nowMs: LiveSession.monotonicMs())
        #expect(!view.contains(auth.token) && !view.contains(auth.guest))
        for text in [host.message, guest.message, host.inviteURL?.absoluteString].compactMap({ $0 }) {
            #expect(!text.contains(auth.token) && !text.contains(auth.guest))
        }
        try await leave(host, guest)
    }

    func leave(_ a: LiveSession, _ b: LiveSession) async throws {
        var homes = 0
        a.onHome = { homes += 1 }
        a.confirmLeave()
        try await pump([a, b]) { a.view.voluntarilyLeft }
        #expect(homes == 1 && !a.visible)
        b.confirmLeave()
        try await pump([b]) { b.view.voluntarilyLeft }
    }
}
