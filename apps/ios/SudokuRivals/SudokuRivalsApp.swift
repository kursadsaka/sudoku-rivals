import SwiftUI

@main
struct SudokuRivalsApp: App {
    @UIApplicationDelegateAdaptor private var delegate: AppDelegate
    @State private var app = AppModel()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView(app: app)
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
    enum Screen { case home, setup, game, live }
    var screen: Screen = .home
    var session: SoloSession?
    private(set) var slots: [Int: SlotStatus] = [:]
    private(set) var store = SoloStore.standard
    var setupError: String?
    private var seedOverride: UInt32?
    let live: LiveSession

    init() {
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
        live.onHome = { [unowned self] in screen = .home }
        // A match that still holds our seat reopens after a cold start.
        if live.visible { screen = .live }
    }

    func openLive() { screen = .live }

    func refreshSlots() {
        slots = Dictionary(uniqueKeysWithValues: SoloStore.sizes.map { ($0, store.read($0)) })
    }

    func status(_ size: Int) -> SlotStatus { slots[size] ?? .missing }

    func resume(_ size: Int) {
        refreshSlots()
        guard case .ready(let envelope, _) = status(size),
              let session = try? SoloSession.restore(envelope, store: store) else { return }
        session.resume() // the player chose to continue; a completed game stays complete
        self.session = session
        screen = .game
    }

    /// Call only after the user confirmed replacing an existing slot.
    func startNew(size: Int) {
        setupError = nil
        let current = store.read(size)
        if current.exists && !current.isReadable {
            do { try store.archiveUnreadable(size) } catch {
                setupError = L10n.t("setup.archiveFailed")
                refreshSlots()
                return
            }
        }
        do {
            session = try SoloSession.start(size: size, seed: seedOverride ?? .random(in: .min ... .max), store: store)
            screen = .game
        } catch {
            setupError = L10n.t("setup.failed")
        }
        refreshSlots()
    }

    /// Leaves the game only once it is safely on disk, unless the user chose to discard.
    func leaveGame(discard: Bool = false, to next: Screen = .home) {
        guard let session else { screen = next; return }
        if !discard {
            session.pause()
            guard session.saveState == .saved else { return }
        }
        self.session = nil
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

    var body: some View {
        NavigationStack {
            Group {
                switch app.screen {
                case .home: HomeView(app: app)
                case .live: LiveScreen(live: app.live)
                case .setup: SetupView(app: app)
                case .game:
                    if let session = app.session { GameScreen(app: app, session: session) } else { HomeView(app: app) }
                }
            }
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .principal) {
                    Text(L10n.t("app.title"))
                        .font(.headline)
                        .accessibilityAddTraits(.isHeader)
                        .accessibilityIdentifier("app-title")
                }
            }
        }
        .tint(Theme.accent)
    }
}

enum Theme {
    static func dynamic(_ light: UInt32, _ dark: UInt32) -> Color {
        func color(_ hex: UInt32) -> UIColor {
            UIColor(red: CGFloat(hex >> 16 & 0xff) / 255, green: CGFloat(hex >> 8 & 0xff) / 255,
                    blue: CGFloat(hex & 0xff) / 255, alpha: 1)
        }
        return Color(UIColor { $0.userInterfaceStyle == .dark ? color(dark) : color(light) })
    }
    // Starting values from docs/ui-contract.json tokens (contrast-checked there).
    static let foreground = dynamic(0x111827, 0xf9fafb)
    static let muted = dynamic(0x4b5563, 0xd1d5db)
    static let accent = dynamic(0x1d4ed8, 0x93c5fd)
    static let onAccent = dynamic(0xffffff, 0x111827)
    static let selected = dynamic(0xdbeafe, 0x1e3a5f)
    static let error = dynamic(0xb91c1c, 0xfca5a5)
    static let border = dynamic(0x6b7280, 0x9ca3af)
}
