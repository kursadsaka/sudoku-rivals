import XCTest

/// Drives the real app against a local sudoku-server through the fault proxy of
/// tests/ios-multiplayer.py. The opponent is an in-process LiveSession: the same
/// Swift adapter and Rust client as the app, talking straight to the server.
@MainActor
final class LiveUITests: XCTestCase {
    let env = ProcessInfo.processInfo.environment
    var proxy: String { env["LIVE_UI_PROXY"] ?? "" }
    var server: String { env["LIVE_TEST_SERVER"] ?? "" }
    var app: XCUIApplication!
    var bot: LiveSession!

    override func setUp() async throws {
        try XCTSkipIf(env["LIVE_UI_PROXY"] == nil || env["LIVE_TEST_SERVER"] == nil, "run via tests/ios-multiplayer.py")
        try await harness("offline", ["offline": false])
        newBot()
    }

    override func tearDown() async throws {
        if let bot, bot.holdsSeat { self.bot.confirmLeave(); try? await until(10, "bot leave") { self.bot.view.voluntarilyLeft } }
        try? await harness("offline", ["offline": false])
    }

    func newBot() {
        let store = LiveStore(directory: FileManager.default.temporaryDirectory.appending(path: "bot-\(UUID().uuidString)"))
        bot = LiveSession(store: store, origin: server)
        bot.name = "Bot"
        bot.open()
    }

    // MARK: Helpers
    // XCUITest's own waits block the main actor, which also runs the bot's network
    // tasks. Every wait here is an async poll that yields and ticks the bot instead.

    func launch(reset: Bool) {
        app = XCUIApplication()
        app.launchArguments = ["-AppleLanguages", "(en)", "-AppleLocale", "en_US"]
        app.launchEnvironment = ["LIVE_TEST_ORIGIN": proxy, "LIVE_TEST_RESET": reset ? "1" : "0"]
        app.launch()
    }

    func el(_ id: String) -> XCUIElement { app.descendants(matching: .any)[id].firstMatch }

    func until(_ timeout: TimeInterval = 20, _ what: String, _ condition: @escaping () -> Bool) async throws {
        let end = Date.now.addingTimeInterval(timeout)
        while true {
            bot?.tick()
            if condition() { return }
            guard Date.now < end else { XCTFail("timed out: \(what)"); throw CancellationError() }
            try await Task.sleep(for: .milliseconds(150))
        }
    }

    /// Lets time pass while the bot keeps polling.
    func pause(_ seconds: TimeInterval) async throws {
        let end = Date.now.addingTimeInterval(seconds)
        try await until(seconds + 1, "pause") { Date.now >= end }
    }

    func exists(_ id: String, timeout: TimeInterval = 15) async throws {
        let element = el(id)
        try await until(timeout, "missing \(id)") { element.exists }
    }

    func tap(_ id: String, timeout: TimeInterval = 15) async throws {
        let element = el(id)
        try await until(timeout, "enabled \(id)") { element.exists && element.isEnabled }
        element.tap()
    }

    func label(_ id: String, timeout: TimeInterval = 20, _ condition: @escaping (String) -> Bool) async throws {
        let element = el(id)
        try await until(timeout, "\(id) label") { element.exists && condition(element.label) }
    }

    var connected: (String) -> Bool { { $0.contains("Connected") && !$0.contains("Disconnected") } }

    @discardableResult
    func harness(_ path: String, _ body: [String: Any] = [:]) async throws -> Data {
        var request = URLRequest(url: URL(string: "\(proxy)/__harness/\(path)")!)
        request.httpMethod = body.isEmpty && path == "calls" ? "GET" : "POST"
        if request.httpMethod == "POST" { request.httpBody = try JSONSerialization.data(withJSONObject: body) }
        request.timeoutInterval = 20
        return try await URLSession.shared.data(for: request).0
    }

    struct Call: Decodable { let path: String; let type: String?; let hash: String }
    func calls() async throws -> [Call] { try JSONDecoder().decode([Call].self, from: try await harness("calls")) }

    func enter(_ index: Int, _ value: Int, size: Int) async throws {
        try await tap("cell-\(index / size + 1)-\(index % size + 1)")
        try await tap("number-\(value)")
    }

    func openLiveSetup(mode: String, size: Int) async throws {
        try await tap("friend-start")
        try await exists("game-mode")
        app.segmentedControls["game-mode"].buttons[mode == "race" ? "Classic race" : "90 s duel"].tap()
        app.segmentedControls["board-size"].buttons["\(size)×\(size)"].tap()
    }

    /// App creates the room; the bot joins with the code shown in the lobby.
    func appHosts(mode: String, size: Int) async throws {
        try await openLiveSetup(mode: mode, size: size)
        try await tap("room-create")
        let code = el("room-code")
        try await until(15, "lobby code") { code.exists && (code.value as? String)?.count == 6 }
        bot.invite = try XCTUnwrap(code.value as? String)
        bot.join()
        try await until(15, "bot joined") { self.bot.snapshot != nil }
        try await label("players") { $0.contains("Bot") }
    }

    /// Both ready → countdown → playing. Returns the shared puzzle's solution.
    func startMatch(size: Int) async throws -> (givens: [Int], solution: [Int]) {
        try await tap("ready-toggle")
        try await until(15, "app ready seen by bot") { self.bot.snapshot?.opponent?.ready == true }
        bot.command("ready", ready: true)
        try await exists("game-board", timeout: 25)
        try await until(15, "bot playing") { self.bot.snapshot?.phase == "playing" }
        let givens = try XCTUnwrap(bot.snapshot?.own.givens)
        XCTAssertEqual(app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'cell-'")).count, size * size)
        // Same one-screen order as solo/web/Android: race has four tools, duel none.
        // (The mode comes from the bot's snapshot: tests that create the room in the app never set `bot.mode`.)
        let race = (bot.snapshot?.mode ?? bot.mode) == "race"
        if race { XCTAssertTrue(app.buttons["undo"].waitForExistence(timeout: 10), "race tools missing") }
        assertOneScreenGameLayout(app, size: size, tools: race ? ["notes-toggle", "erase", "undo", "redo"] : [])
        return (givens, try XCTUnwrap(solve(givens, size: size)))
    }

    func leaveToHome() async throws {
        try await tap("back")
        let confirm = app.alerts.buttons["Leave"].firstMatch
        try await until(5, "leave alert") { confirm.exists }
        confirm.tap()
        try await exists("friend-start", timeout: 15)
    }

    // MARK: Scenarios

    func testRaceSixOfflineColdRestartSingleSyncWinRematchLeave() async throws {
        launch(reset: true)
        try await appHosts(mode: "race", size: 6)
        let (givens, solution) = try await startMatch(size: 6)
        let blanks = givens.indices.filter { givens[$0] == 0 }
        try await label("connection-status", connected)

        try await harness("offline", ["offline": true])
        try await label("connection-status") { $0.contains("Disconnected") || $0.contains("Reconnecting") }
        for index in blanks.prefix(3) { try await enter(index, solution[index], size: 6) }
        let syncsBefore = try await calls().filter { $0.type == "sync" }.count
        add(screenshot("race-offline"))

        // Cold restart while offline: the board and seat come back from the protected record.
        app.terminate()
        launch(reset: false)
        try await exists("game-board")
        for index in blanks.prefix(3) {
            XCTAssertTrue(app.buttons["cell-\(index / 6 + 1)-\(index % 6 + 1)"].label.contains(", \(solution[index]), notes"))
        }
        try await harness("offline", ["offline": false])
        try await label("connection-status", timeout: 30, connected)
        try await until(20, "bot sees progress") { (self.bot.snapshot?.opponent?.progressPercent ?? 0) > 0 }
        let afterRestart = try await calls()
        XCTAssertEqual(afterRestart.filter { $0.type == "sync" }.count - syncsBefore, 1, "offline edits were not one sync")
        XCTAssertTrue(afterRestart.contains { $0.path.hasSuffix("/resume") }, "cold start did not resume")

        for index in blanks.dropFirst(3) { try await enter(index, solution[index], size: 6) }
        try await label("result-heading") { $0 == "You won the match" }
        try await until(20, "bot finished") { self.bot.snapshot?.phase == "finished" }
        XCTAssertEqual(bot.snapshot?.result?.winner, 1 - (bot.snapshot?.own.seat ?? 1))
        add(screenshot("race-won"))

        try await tap("rematch")
        try await until(20, "bot sees rematch") { self.bot.snapshot?.opponent?.rematch == true }
        bot.command("rematch")
        let heading = el("result-heading")
        try await until(20, "rematch left result") { !heading.exists }
        try await leaveToHome()
    }

    func testDuelNineInviteLinkLostWrongMoveLockBackgroundAndTimeUp() async throws {
        launch(reset: true)
        bot.mode = "duel"
        bot.size = 9
        bot.create()
        try await until(15, "bot room") { self.bot.snapshot != nil }
        let code = try XCTUnwrap(bot.snapshot?.roomCode)
        try await tap("friend-start")
        try await tap("room-code")
        el("room-code").typeText("\(proxy)/r/\(code)")
        try await tap("room-join")
        try await label("players") { $0.contains("Bot") }
        let (givens, solution) = try await startMatch(size: 9)
        let blank = try XCTUnwrap(givens.firstIndex(of: 0))

        try await harness("drop", ["type": "move"])
        let before = try await calls().count
        try await enter(blank, solution[blank] % 9 + 1, size: 9)
        try await label("input-lock", timeout: 10) { $0.hasPrefix("Input locked") }
        let moves = try await calls().dropFirst(before).filter { $0.type == "move" }
        XCTAssertGreaterThanOrEqual(moves.count, 2)
        XCTAssertEqual(moves.first?.hash, moves.dropFirst().first?.hash, "retry changed the move body")
        try await label("own-score") { $0 == "You: 0" }
        XCTAssertFalse(app.buttons["number-1"].isEnabled, "input not locked")
        add(screenshot("duel-lock"))

        try await exists("game-status", timeout: 6)
        try await tap("number-\(solution[blank])")
        try await label("own-score") { $0 == "You: 10" }
        XCTAssertTrue(app.buttons["cell-\(blank / 9 + 1)-\(blank % 9 + 1)"].label.contains("locked"))

        let resumes = try await calls().filter { $0.path.hasSuffix("/resume") }.count
        XCUIDevice.shared.press(.home)
        try await pause(2)
        app.activate()
        try await label("connection-status", connected)
        let afterBackground = try await calls().filter { $0.path.hasSuffix("/resume") }.count
        XCTAssertGreaterThan(afterBackground, resumes, "no resume after background")

        // Only the server ends the duel: 90 s after start, higher score wins.
        try await label("result-heading", timeout: 100) { $0 == "You won the match" }
        XCTAssertTrue(el("result-details").label.contains("Your score: 10"))
        XCTAssertTrue(app.staticTexts["The 90 seconds ran out."].exists)
        add(screenshot("duel-time-up"))
        try await leaveToHome()
    }

    func testRaceNineOpponentWinsThenServerRestartIsLostWithoutResult() async throws {
        launch(reset: true)
        try await appHosts(mode: "race", size: 9)
        let (_, solution) = try await startMatch(size: 9)
        // The bot syncs the whole solution; the app must show the server's verdict.
        for index in solution.indices where bot.snapshot?.own.givens[index] == 0 {
            bot.selected = index
            bot.enter(solution[index])
        }
        try await label("result-heading", timeout: 30) { $0 == "Your opponent won" }
        try await leaveToHome()
        bot.confirmLeave()
        try await until(15, "bot left") { self.bot.view.voluntarilyLeft }

        newBot()
        try await appHosts(mode: "duel", size: 6)
        _ = try await startMatch(size: 6)
        try await harness("restart")
        try await label("result-heading", timeout: 30) { $0 == "The match cannot continue" }
        XCTAssertTrue(el("result-details").label.contains("No win or loss was declared"))
        add(screenshot("server-restart"))
        try await tap("live-recover")
        let confirm = app.alerts.buttons["New session"].firstMatch
        try await until(5, "reset alert") { confirm.exists }
        confirm.tap()
        try await exists("room-create")
    }

    func testExpiredRaceResultCanPreserveSaveAndCreateNewRoom() async throws {
        try await recoverKnownResultAndCreateNewRoom(mode: "race")
    }

    func testExpiredDuelResultCanPreserveSaveAndCreateNewRoom() async throws {
        try await recoverKnownResultAndCreateNewRoom(mode: "duel")
    }

    func recoverKnownResultAndCreateNewRoom(mode: String) async throws {
        launch(reset: true)
        try await appHosts(mode: mode, size: 6)
        _ = try await startMatch(size: 6)
        let oldCode = try XCTUnwrap(bot.snapshot?.roomCode)
        bot.command("resign")
        try await label("result-heading") { $0 == "You won the match" }
        XCTAssertFalse(el("live-recover").exists, "connected results should keep the normal rematch flow")
        // Restart produces the same terminal session loss as expiry/resume 410.
        try await harness("restart")
        try await label("connection-status", timeout: 30) { $0.contains("Cannot return to the session") }
        XCTAssertEqual(el("result-heading").label, "You won the match")
        XCTAssertFalse(el("rematch").isEnabled)
        app.terminate()
        launch(reset: false)
        try await label("result-heading") { $0 == "You won the match" }
        try await tap("live-recover")
        let cancel = app.alerts.buttons["Cancel"].firstMatch
        try await until(5, "reset cancellation") { cancel.exists }
        cancel.tap()
        XCTAssertEqual(el("result-heading").label, "You won the match")
        try await tap("live-recover")
        let confirm = app.alerts.buttons["New session"].firstMatch
        try await until(5, "reset confirmation") { confirm.exists }
        confirm.tap()
        try await exists("room-create")
        try await tap("room-create")
        try await until(15, "new room after preserved result") {
            let code = self.el("room-code").value as? String
            return code?.count == 6 && code != oldCode
        }
        try await leaveToHome()
    }

    func screenshot(_ name: String) -> XCTAttachment {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        return attachment
    }
}

/// Plain backtracking; both seats share the puzzle, so the bot's givens suffice.
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
