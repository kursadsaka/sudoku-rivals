import Foundation
import Observation
import Security

/// Presentation-only mirror of the Rust client's `view_json`. It carries no
/// credentials: guest, token and request bodies never appear in the view.
struct LiveView: Decodable, Equatable {
    struct Own: Decodable, Equatable {
        let seat: Int
        let name: String
        let ready: Bool
        let connected: Bool
        let boardId: UInt32
        let givens: [Int]
        let cells: [Int]
        let score: Int
        let combo: Int
        let wrong: Int
        let rematch: Bool
    }
    struct Opponent: Decodable, Equatable {
        let name: String
        let ready: Bool
        let connected: Bool
        let progressPercent: Int?
        let score: Int?
        let rematch: Bool
    }
    struct Outcome: Decodable, Equatable {
        let winner: Int?
        let reason: String
    }
    struct Snapshot: Decodable, Equatable {
        let roomCode: String
        let matchId: String
        let generation: UInt64
        let mode: String
        let size: Int
        /// Room level fixed at creation ("beginner"…"expert"); nil for rooms created without one.
        let difficulty: String?
        let phase: String
        let startMs: UInt64?
        let endMs: UInt64?
        let own: Own
        let opponent: Opponent?
        let result: Outcome?
    }
    struct Race: Decodable, Equatable {
        let cells: [Int]
        let notes: [[Int]]
        let conflicts: [Int]
        let canUndo: Bool
        let canRedo: Bool
        let dirty: Bool
    }
    struct Receipt: Decodable, Equatable {
        let matchId: String
        let code: String
    }

    var connection: String
    var snapshot: Snapshot?
    var race: Race?
    var lastReceipt: Receipt?
    var lastError: String?
    var pendingCommand: Bool
    var pendingLeave: Bool
    var pendingHttp: Bool
    var reconcileRequired: Bool
    var retryAtMs: UInt64?
    var pollDue: Bool
    var estimatedServerMs: UInt64?
    var remainingMs: UInt64?
    var lockedRemainingMs: UInt64?
    var voluntarilyLeft: Bool
}

extension LiveView {
    init(json: String) throws {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        self = try decoder.decode(Self.self, from: Data(json.utf8))
    }

    static let idle = LiveView(connection: "disconnected", snapshot: nil, race: nil, lastReceipt: nil, lastError: nil,
                               pendingCommand: false, pendingLeave: false, pendingHttp: false, reconcileRequired: false, retryAtMs: nil,
                               pollDue: false, estimatedServerMs: nil, remainingMs: nil, lockedRemainingMs: nil,
                               voluntarilyLeft: false)
}

/// One HTTP request the Rust dispatcher asks the platform to perform.
struct LiveRequest: Decodable, Equatable {
    let id: Int64
    let method: String
    let path: String
    let body: String
}

/// The opaque Rust save holds seat credentials. It lives in one file that is
/// excluded from backups, written atomically, and never logged or shown.
struct LiveStore {
    struct Record: Codable {
        var version = 1
        let origin: String
        let client: String
    }

    let directory: URL
    var failWrites = false

    static var standard: LiveStore {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return LiveStore(directory: base.appending(path: "LiveSession", directoryHint: .isDirectory))
    }

    var url: URL { directory.appending(path: "live-session.json") }
    var exists: Bool { FileManager.default.fileExists(atPath: url.path) }

    func load() throws -> Record {
        let data = try Data(contentsOf: url)
        // Core caps the client save at 1 MiB; JSON string escaping can double it.
        guard data.count <= 2_200_000 else { throw CocoaError(.fileReadTooLarge) }
        let record = try JSONDecoder().decode(Record.self, from: data)
        guard record.version == 1 else { throw CocoaError(.fileReadCorruptFile) }
        return record
    }

    func save(_ record: Record) throws {
        if failWrites { throw CocoaError(.fileWriteUnknown) }
        var folder = directory
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        // Set on the folder: an atomic write replaces the file and would drop a per-file flag.
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        try folder.setResourceValues(values)
        let data = try JSONEncoder().encode(record)
        try data.write(to: url, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        guard try Data(contentsOf: url) == data else { throw CocoaError(.fileWriteUnknown) }
    }

    /// Copies the current file aside before a reset replaces it. Never deletes.
    func preserve(now: Date = .now) throws {
        guard exists else { return }
        let stamp = Int(now.timeIntervalSince1970 * 1000)
        try FileManager.default.copyItem(at: url, to: directory.appending(path: "live-session-recovery-\(stamp).json"))
    }
}

/// HTTP and storage adapter around the shared Rust `MultiplayerClient`. The
/// Rust object owns rules, sequences, retries, clock and drafts; this type
/// only performs the requests it is handed and persists the opaque save.
@MainActor
@Observable
final class LiveSession {
    typealias Transport = (URLRequest) async throws -> (Data, Int)

    #if DEBUG
    nonisolated static let allowsLoopbackHTTP = true
    #else
    nonisolated static let allowsLoopbackHTTP = false
    #endif
    /// Fixed G16 Render Free staging origin. Only the `Staging` build configuration (compilation
    /// condition `STAGING`) uses it; Debug keeps the loopback test origin and Release stays unconfigured
    /// until a production server exists. HTTPS only: there is no ATS exception anywhere.
    nonisolated static let stagingOrigin = "https://sudoku-rivals-g16-staging.onrender.com"
    #if STAGING
    static let releaseOrigin: String? = stagingOrigin
    /// The Staging build talks to exactly this origin; a saved record naming any other one is preserved, not loaded.
    nonisolated static let pinsOrigin = true
    #else
    // shortcut: no production match server yet; Release shows "not configured" until one is set here.
    static let releaseOrigin: String? = nil
    nonisolated static let pinsOrigin = false
    #endif

    private(set) var origin: String?
    private(set) var view: LiveView = .idle
    var name = L10n.t("live.defaultName")
    var mode = "race"
    var size = 6
    /// Sent with `create` only when set; nil keeps the v1 request shape (an unrated room).
    var difficulty: String?
    var invite = ""
    var selected = 0
    var notesMode = false
    private(set) var message: String?
    private(set) var storageError = false
    private(set) var protectedSave = false
    private(set) var visible = false
    var leaveConfirmation = false
    var resetConfirmation = false

    @ObservationIgnored var onHome: () -> Void = {}
    @ObservationIgnored var store: LiveStore
    @ObservationIgnored private var client: MultiplayerClient
    @ObservationIgnored private var guest = LiveSession.secureGuest()
    @ObservationIgnored private let transport: Transport
    @ObservationIgnored private let now: () -> UInt64
    @ObservationIgnored private let defaultOrigin: String?
    @ObservationIgnored private var task: Task<Void, Never>?
    @ObservationIgnored private var epoch = 0
    @ObservationIgnored private var activeId: Int64?
    @ObservationIgnored private var foreground = false
    @ObservationIgnored private var resumeRequired = false
    @ObservationIgnored private var deferred: LiveRequest?
    @ObservationIgnored private var leaving = false

    init(store: LiveStore, origin: String?, pinOrigin: Bool = LiveSession.pinsOrigin,
         transport: @escaping Transport = LiveSession.urlSessionTransport,
         now: @escaping () -> UInt64 = LiveSession.monotonicMs) {
        self.store = store
        self.transport = transport
        self.now = now
        self.defaultOrigin = origin.flatMap { Self.normalize($0) }
        self.origin = defaultOrigin
        client = MultiplayerClient(seed: .random(in: .min ... .max))
        if store.exists {
            do {
                let record = try store.load()
                guard let saved = Self.normalize(record.origin), !pinOrigin || saved == defaultOrigin
                else { throw CocoaError(.fileReadCorruptFile) }
                client = try MultiplayerClient.restore(saved: record.client)
                self.origin = saved
            } catch {
                protectedSave = true
                message = L10n.t("protected.detail")
            }
        }
        refresh()
        leaving = view.pendingLeave
        visible = view.pendingHttp || (view.snapshot != nil && !view.voluntarilyLeft)
        resumeRequired = visible && !view.pendingHttp && view.connection != "session_lost"
    }

    // MARK: Derived state

    var snapshot: LiveView.Snapshot? { view.snapshot }
    var isConfigured: Bool { origin != nil }
    /// A seat the server still holds for us; leaving it needs confirmation.
    var holdsSeat: Bool { snapshot != nil && !view.voluntarilyLeft && view.connection != "session_lost" }
    var inviteURL: URL? {
        guard let origin, let code = snapshot?.roomCode else { return nil }
        return URL(string: "\(origin)/r/\(code)")
    }

    func canInput() -> Bool {
        guard let s = snapshot, s.phase == "playing", !view.reconcileRequired, view.connection != "session_lost" else { return false }
        if s.mode == "race" { return view.race != nil }
        return view.connection == "connected" && !view.pendingCommand
            && (view.lockedRemainingMs ?? 1) == 0 && (view.remainingMs ?? 0) > 0
    }

    // MARK: Lifecycle

    func open() {
        visible = true
        foreground = true
        tick()
    }

    func setForeground(_ active: Bool) {
        foreground = active && visible
        if !active {
            if holdsSeat { resumeRequired = true }
            cancelTransport()
            // A new room setup after leaving is still ours to recover; the departed room is not.
            if view.pendingHttp || (!view.voluntarilyLeft && snapshot != nil) {
                dispatch(["type": "network_failed"], quiet: true)
            }
            persist()
        } else if visible {
            if !resumeRequired && !storageError, let request = deferred {
                deferred = nil
                send(request)
            }
            tick()
        }
    }

    func goHome() {
        if holdsSeat { leaveConfirmation = true; return }
        visible = false
        foreground = false
        cancelTransport()
        if view.pendingHttp { dispatch(["type": "network_failed"], quiet: true) }
        persist()
        onHome()
    }

    func confirmLeave() {
        leaveConfirmation = false
        leaving = true
        message = L10n.t("live.leaving")
        dispatch(["type": "leave"])
    }

    func requestReset() { resetConfirmation = true }

    /// Keeps a copy of the old record, then starts a fresh client and guest.
    func confirmReset() {
        resetConfirmation = false
        do { try store.preserve() } catch {
            message = L10n.t("live.error.resetFailed")
            return
        }
        cancelTransport()
        client = MultiplayerClient(seed: .random(in: .min ... .max))
        guest = Self.secureGuest()
        origin = origin ?? defaultOrigin
        protectedSave = false
        resumeRequired = false
        deferred = nil
        leaving = false
        selected = 0
        notesMode = false
        message = nil
        refresh()
        persist()
    }

    // MARK: Player actions

    func create() {
        guard isConfigured else { message = L10n.t("live.notConfigured"); return }
        var action: [String: Any] = ["type": "create", "guest": guest, "name": name, "mode": mode, "size": size]
        if let difficulty { action["difficulty"] = difficulty }
        dispatch(action)
    }

    func join() {
        guard isConfigured else { message = L10n.t("live.notConfigured"); return }
        dispatch(["type": "join", "guest": guest, "invite": invite, "name": name])
    }

    func command(_ type: String, ready: Bool? = nil) {
        var command: [String: Any] = ["type": type]
        if let ready { command["ready"] = ready }
        dispatch(["type": "command", "command": command])
    }

    func edit(_ type: String, value: Int? = nil) {
        var command: [String: Any] = ["type": type]
        if ["set", "toggle_note", "erase"].contains(type) { command["index"] = selected }
        if let value { command["value"] = value }
        dispatch(["type": "edit", "command": command])
    }

    func enter(_ value: Int) {
        guard let s = snapshot, canInput() else { return }
        if s.mode == "race" {
            edit(notesMode ? "toggle_note" : "set", value: value)
        } else {
            dispatch(["type": "command", "command": [
                "type": "move", "board_id": s.own.boardId, "index": selected, "value": value,
            ] as [String: Any]])
        }
    }

    func moveSelection(rows: Int, columns: Int) {
        guard let n = snapshot?.size else { return }
        selected = min(max(selected / n + rows, 0), n - 1) * n + min(max(selected % n + columns, 0), n - 1)
    }

    func reconcile(keepLocal: Bool) { dispatch(["type": "reconcile", "keep_local": keepLocal]) }

    func retryStorage() {
        guard !protectedSave, persist() else { return }
        let request = deferred
        deferred = nil
        if resumeRequired && snapshot != nil { tick() } else if let request { send(request) }
    }

    // MARK: Engine loop

    /// Called every 250 ms while the live screen is on screen.
    func tick() {
        // After leaving, only a pending new create/join may run; the old room is never polled or resumed.
        guard foreground, !protectedSave, visible, !view.voluntarilyLeft || view.pendingHttp else { return }
        refresh()
        let time = now()
        if activeId != nil || deferred != nil || storageError { return }
        if resumeRequired, !leaving, snapshot != nil, view.connection != "session_lost" {
            if view.retryAtMs.map({ time >= $0 }) ?? true, dispatch(["type": "resume"], quiet: true) {
                resumeRequired = false
            }
            return
        }
        if let retryAt = view.retryAtMs, time >= retryAt {
            if !dispatch(["type": "retry"], quiet: true), snapshot != nil { dispatch(["type": "resume"], quiet: true) }
            return
        }
        if view.pendingHttp { return }
        if view.connection == "reconcile" {
            if view.pollDue { dispatch(["type": "poll"], quiet: true) }
            return
        }
        guard view.connection == "connected" else { return }
        if view.race?.dirty == true, snapshot?.phase == "playing", !view.pendingCommand, !view.reconcileRequired {
            dispatch(["type": "sync"], quiet: true)
        } else if view.pollDue {
            dispatch(["type": "poll"], quiet: true)
        }
    }

    private func refresh() {
        guard let next = try? LiveView(json: client.view(nowMs: now())), next != view else { return }
        let old = snapshot
        if let board = next.snapshot, !board.own.givens.isEmpty,
           old?.matchId != board.matchId || old?.own.boardId != board.own.boardId || old?.own.givens.isEmpty == true {
            // Lobby and countdown snapshots hide the board, even when its ID is already assigned.
            selected = board.own.givens.firstIndex(of: 0) ?? 0
            notesMode = false
        }
        view = next
    }

    @discardableResult
    private func persist() -> Bool {
        guard !protectedSave else { return false }
        guard let origin else { return true } // nothing can be stored before a server exists
        do {
            try store.save(.init(origin: origin, client: client.save()))
            storageError = false
            return true
        } catch {
            storageError = true
            return false
        }
    }

    @discardableResult
    private func dispatch(_ action: [String: Any], quiet: Bool = false) -> Bool {
        guard !protectedSave else { return false }
        struct Effect: Decodable { let request: LiveRequest?; let changed: Bool }
        do {
            let json = String(decoding: try JSONSerialization.data(withJSONObject: action), as: UTF8.self)
            let effect = try JSONDecoder().decode(Effect.self, from: Data(try client.dispatch(action: json, nowMs: now()).utf8))
            if !quiet { message = nil }
            // Credential-bearing requests go out only once the state that issued them is durable.
            let saved = effect.changed ? persist() : !storageError
            refresh()
            if let request = effect.request {
                if saved { send(request) } else { deferred = request }
            }
            if leaving, action["type"] as? String == "receive", view.voluntarilyLeft {
                leaving = false
                visible = false
                foreground = false
                cancelTransport()
                onHome()
            }
            return true
        } catch {
            if !quiet { message = L10n.t("live.error.action") }
            return false
        }
    }

    private func cancelTransport() {
        epoch += 1
        activeId = nil
        task?.cancel()
        task = nil
    }

    private func send(_ request: LiveRequest) {
        guard activeId != request.id else { return }
        guard foreground else { deferred = request; return }
        guard request.method == "POST", request.path.hasPrefix("/v1/"),
              !request.path.contains("?"), !request.path.contains("#"),
              let origin, let url = URL(string: origin + request.path) else {
            message = L10n.t("live.error.unsafe")
            return
        }
        cancelTransport()
        activeId = request.id
        let ticket = epoch
        var http = URLRequest(url: url)
        http.httpMethod = "POST"
        http.setValue("application/json", forHTTPHeaderField: "Content-Type")
        http.setValue("no-store", forHTTPHeaderField: "Cache-Control")
        http.httpBody = Data(request.body.utf8) // exact opaque body: never rebuilt, logged or shown
        let transport = self.transport
        task = Task {
            let outcome = try? await transport(http)
            // A newer request, a reset or backgrounding invalidated this callback.
            guard ticket == self.epoch, self.activeId == request.id else { return }
            self.activeId = nil
            self.task = nil
            guard let (data, status) = outcome, data.count <= 131_072 else {
                // Only a race keeps offline edits; duel input is disabled while disconnected.
                self.message = L10n.t(self.view.race != nil ? "live.error.unreachableRace" : "live.error.unreachable")
                self.dispatch(["type": "network_failed", "request_id": request.id], quiet: true)
                return
            }
            let text = String(decoding: data, as: UTF8.self)
            if !self.dispatch(["type": "receive", "request_id": request.id, "status": status, "response": text], quiet: true) {
                self.message = L10n.t("live.error.invalidResponse")
                self.dispatch(["type": "network_failed", "request_id": request.id], quiet: true)
            } else {
                self.message = (200..<300).contains(status) ? nil : Self.serverMessage(text)
            }
        }
    }

    // MARK: Platform pieces

    /// Same clock domain for every call in one process; includes device sleep.
    nonisolated static func monotonicMs() -> UInt64 { clock_gettime_nsec_np(CLOCK_MONOTONIC_RAW) / 1_000_000 }

    /// 256-bit guest secret from the system CSPRNG. Never falls back to a weak value.
    static func secureGuest() -> String {
        var bytes = [UInt8](repeating: 0, count: 32)
        let status = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        precondition(status == errSecSuccess, "System random generator unavailable")
        return bytes.map { String(format: "%02x", $0) }.joined()
    }

    /// HTTPS only; plain HTTP to this machine's loopback in debug builds.
    nonisolated static func normalize(_ raw: String, allowLoopbackHTTP: Bool = allowsLoopbackHTTP) -> String? {
        guard let parts = URLComponents(string: raw.trimmingCharacters(in: .whitespacesAndNewlines)),
              let scheme = parts.scheme?.lowercased(), let host = parts.host?.lowercased(), !host.isEmpty,
              parts.user == nil, parts.password == nil, parts.query == nil, parts.fragment == nil,
              parts.path.isEmpty || parts.path == "/" else { return nil }
        guard scheme == "https" || (allowLoopbackHTTP && scheme == "http" && ["localhost", "127.0.0.1"].contains(host))
        else { return nil }
        if let port = parts.port, !(1...65535).contains(port) { return nil }
        return "\(scheme)://\(host)" + (parts.port.map { ":\($0)" } ?? "")
    }

    nonisolated static func serverMessage(_ raw: String) -> String {
        struct Failure: Decodable { let error: String }
        switch (try? JSONDecoder().decode(Failure.self, from: Data(raw.utf8)))?.error {
        case "room_full": return L10n.t("server.roomFull")
        case "room_missing", "room_closed": return L10n.t("server.roomMissing")
        case "unauthorized": return L10n.t("server.unauthorized")
        case "rate_limited", "capacity": return L10n.t("server.busy")
        case "stale_generation", "stale_match", "sequence_gap", "sequence_conflict": return L10n.t("server.syncing")
        default: return L10n.t("server.generic")
        }
    }

    /// Ephemeral session: no cookies, no cache, no redirects, bounded time.
    nonisolated static let urlSessionTransport: Transport = { request in
        let (data, response) = try await httpSession.data(for: request, delegate: NoRedirects.shared)
        return (data, (response as? HTTPURLResponse)?.statusCode ?? 0)
    }

    nonisolated private static let httpSession: URLSession = {
        let config = URLSessionConfiguration.ephemeral
        config.httpCookieStorage = nil
        config.httpShouldSetCookies = false
        config.urlCache = nil
        config.requestCachePolicy = .reloadIgnoringLocalCacheData
        config.timeoutIntervalForRequest = 4
        config.timeoutIntervalForResource = 10
        config.waitsForConnectivity = false
        return URLSession(configuration: config)
    }()

    private final class NoRedirects: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
        static let shared = NoRedirects()
        func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse,
                        newRequest request: URLRequest) async -> URLRequest? { nil }
    }
}
