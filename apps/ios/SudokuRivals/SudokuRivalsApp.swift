import SwiftUI

@main
struct SudokuRivalsApp: App {
    @UIApplicationDelegateAdaptor private var delegate: AppDelegate
    @State private var app = AppModel()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView(app: app)
                .preferredColorScheme(app.prefs.colorScheme)
        }
        .onChange(of: scenePhase) { _, phase in
            // Local solo time stops in the background; the game is saved there too.
            if phase == .background, app.screen == .game { app.session?.pause() }
            // Live: drop the transport and save in the background; resume with a new generation on return.
            if phase == .background || phase == .active { app.live.setForeground(phase == .active) }
        }
    }
}

final class AppDelegate: UIResponder, UIApplicationDelegate {
    /// The system Undo/Redo menu would claim ⌘Z/⇧⌘Z before the game's own
    /// shortcuts; solo history lives in the core, so the game handles them.
    override func buildMenu(with builder: UIMenuBuilder) {
        super.buildMenu(with: builder)
        builder.remove(menu: .undoRedo)
    }
}

@MainActor
@Observable
final class AppModel {
    enum Screen { case home, setup, game, live, settings }
    var screen: Screen = .home
    var session: SoloSession?
    private(set) var slots: [Int: SlotStatus] = [:]
    /// Rated level of each saved puzzle (derived from its givens by the engine; saves carry no extra field).
    private(set) var levels: [Int: String] = [:]
    private(set) var store = SoloStore.standard
    var setupError: String?
    /// True while a rated puzzle is being dug (off the main actor).
    var preparing: Bool { preparation != nil }
    private var preparation: Task<Void, Never>?
    private var seedOverride: UInt32?
    let live: LiveSession
    let prefs: AppPreferences
    /// Where Settings returns to.
    private var settingsReturn: Screen = .home

    init() {
        #if DEBUG
        if ProcessInfo.processInfo.environment["SOLO_TEST_RESET"] == "1" || ProcessInfo.processInfo.environment["LIVE_TEST_RESET"] == "1" {
            AppPreferences.reset()
        }
        #endif
        prefs = AppPreferences()
        var liveStore = LiveStore.standard
        var liveOrigin = LiveSession.releaseOrigin
        #if DEBUG && STAGING
        #error("The Staging configuration must not be built with DEBUG: it never allows loopback or test hooks.")
        #elseif DEBUG
        let env = ProcessInfo.processInfo.environment
        // Simulator reaches the Mac's loopback; `cargo run -p sudoku-server` listens on :3000.
        liveOrigin = env["LIVE_TEST_ORIGIN"] ?? "http://127.0.0.1:3000"
        if env["LIVE_TEST_RESET"] == "1" { try? FileManager.default.removeItem(at: liveStore.directory) }
        liveStore.failWrites = env["LIVE_TEST_FAIL_WRITES"] == "1"
        #endif
        live = LiveSession(store: liveStore, origin: liveOrigin)
        #if DEBUG
        applyTestHooks(env)
        #endif
        refreshSlots()
        // Friend setup opens on what the player chose last (9×9, classic race, Easy by default).
        live.mode = prefs.liveMode
        live.size = prefs.liveSize
        live.difficulty = prefs.liveDifficulty
        if !prefs.displayName.isEmpty { live.name = prefs.displayName }
        Feedback.shared.haptics = prefs.haptics
        Feedback.shared.sound = prefs.sound
        live.onHome = { [unowned self] in screen = .home }
        // A match that still holds our seat reopens after a cold start.
        if live.visible { screen = .live }
    }

    func openLive() { screen = .live }

    func openSettings() {
        settingsReturn = screen == .game ? .game : .home
        // Settings is not part of play time: a running solo game pauses (and saves) first.
        if screen == .game { session?.pause() }
        screen = .settings
    }

    func closeSettings() { screen = settingsReturn == .game && session != nil ? .game : .home }

    func cue(_ cue: Feedback.Cue) { Feedback.shared.play(cue) }

    func refreshSlots() {
        slots = Dictionary(uniqueKeysWithValues: SoloStore.sizes.map { ($0, store.read($0)) })
        var rated: [Int: String] = [:]
        for (size, status) in slots {
            if case .ready(let envelope, _) = status, let level = SoloSession.rate(envelope.game) { rated[size] = level }
        }
        levels = rated
    }

    func level(_ size: Int) -> String? { levels[size] }

    func status(_ size: Int) -> SlotStatus { slots[size] ?? .missing }

    func resume(_ size: Int) {
        refreshSlots()
        guard case .ready(let envelope, _) = status(size),
              let session = try? SoloSession.restore(envelope, store: store) else { return }
        session.resume() // the player chose to continue; a completed game stays complete
        self.session = session
        screen = .game
    }

    func cancelSetup() {
        preparation?.cancel()
        preparation = nil
        setupError = nil
        screen = .home
    }

    /// Call only after the user confirmed replacing an existing slot. Digging a rated puzzle can take a
    /// moment, so it runs off the main actor while the start button says "Preparing your puzzle…".
    func startNew(size: Int, difficulty: String) {
        guard !preparing else { return }
        setupError = nil
        let seed = seedOverride ?? .random(in: .min ... .max)
        preparation = Task {
            let result = await Task.detached(priority: .userInitiated) {
                Result { try SoloSession.generate(size: size, seed: seed, difficulty: difficulty) }
            }.value
            // Returning to setup does not give an earlier, cancelled request permission to save.
            guard !Task.isCancelled else { return }
            preparation = nil
            guard screen == .setup else { return }
            do {
                let game = try result.get()
                let current = store.read(size)
                if current.exists && !current.isReadable {
                    do { try store.archiveUnreadable(size) } catch {
                        setupError = L10n.t("setup.archiveFailed")
                        refreshSlots()
                        return
                    }
                }
                session = try SoloSession.begin(game: game, store: store)
                screen = .game
                cue(.ready)
            } catch {
                setupError = L10n.t("setup.failed")
            }
            refreshSlots()
        }
    }

    /// Leaves the game only once it is safely on disk, unless the user chose to discard.
    func leaveGame(discard: Bool = false, to next: Screen = .home) {
        guard let session else { screen = next; return }
        if !discard {
            session.pause()
            guard session.saveState == .saved else { return }
        }
        self.session = nil
        if next == .setup { setupError = nil }
        screen = next
        refreshSlots()
    }

    #if DEBUG
    /// UI-test hooks; compiled out of release builds.
    private func applyTestHooks(_ env: [String: String]) {
        if env["SOLO_TEST_RESET"] == "1" { try? FileManager.default.removeItem(at: store.directory) }
        seedOverride = env["SOLO_TEST_SEED"].flatMap(UInt32.init)
        let plant = { (key: String, body: String) in
            guard let size = env[key].flatMap(Int.init) else { return }
            try? FileManager.default.createDirectory(at: self.store.directory, withIntermediateDirectories: true)
            try? Data(body.utf8).write(to: self.store.url(size))
        }
        plant("SOLO_TEST_CORRUPT", #"{"version":1,"game":"not a game""#)
        plant("SOLO_TEST_FUTURE", #"{"version":2,"game":"{}","elapsedMs":0}"#)
        store.failWrites = env["SOLO_TEST_FAIL_WRITES"] == "1"
    }
    #endif
}

struct RootView: View {
    let app: AppModel
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        NavigationStack {
            Group {
                switch app.screen {
                case .home: HomeView(app: app)
                case .live: LiveScreen(live: app.live, prefs: app.prefs)
                case .setup: SetupView(app: app)
                case .settings: SettingsView(app: app)
                case .game:
                    if let session = app.session { GameScreen(app: app, session: session) } else { HomeView(app: app) }
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Theme.canvas.ignoresSafeArea())
            .navigationBarTitleDisplayMode(.inline)
            .toolbarBackground(Theme.canvas, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .principal) {
                    // Wordmark: the app mark and "Sudoku Rivals", as in the web header.
                    HStack(spacing: 8) {
                        AppMark(side: 26)
                        Text("Sudoku \(Text("Rivals").fontWeight(.heavy))")
                            .font(.system(.headline, design: .rounded))
                    }
                    .accessibilityElement(children: .ignore)
                    .accessibilityLabel(L10n.t("app.title"))
                    .accessibilityAddTraits(.isHeader)
                    .accessibilityIdentifier("app-title")
                }
            }
        }
        .tint(Theme.accent)
        .animation(reduceMotion ? nil : .easeOut(duration: 0.16), value: app.screen)
    }
}
