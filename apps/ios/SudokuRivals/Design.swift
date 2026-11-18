import AVFoundation
import SwiftUI
import UIKit

/// Paper and ink, the same tokens as the web (`apps/web/styles.css`): printed givens, ballpoint-blue entries,
/// pencil notes; amber is always the rival. Light/dark follow the system unless Settings pins a theme.
enum Theme {
    static func dynamic(_ light: UInt32, _ dark: UInt32) -> Color {
        func color(_ hex: UInt32) -> UIColor {
            UIColor(red: CGFloat(hex >> 16 & 0xff) / 255, green: CGFloat(hex >> 8 & 0xff) / 255,
                    blue: CGFloat(hex & 0xff) / 255, alpha: 1)
        }
        return Color(UIColor { $0.userInterfaceStyle == .dark ? color(dark) : color(light) })
    }

    static let canvas = dynamic(0xf4f1ea, 0x121214)
    static let background = dynamic(0xfffdf9, 0x1c1c20)
    static let foreground = dynamic(0x1c1b18, 0xf4f2ec)
    static let muted = dynamic(0x5d584e, 0xb6b2a8)
    static let line = dynamic(0xddd6c8, 0x34343b)
    static let border = dynamic(0x8b8476, 0x8f8b7f)
    static let grid = dynamic(0x2a2824, 0xa5a196)
    /// Text and icons in the brand hue (entries, links, selected outlines).
    static let accent = dynamic(0x1d4ed8, 0x93b4ff)
    static let onAccent = dynamic(0xffffff, 0x0b1430)
    /// Filled surfaces in the brand hue (primary buttons, selected cell, the solo card).
    static let brand = dynamic(0x1d4ed8, 0x3b66f0)
    static let onBrand = Color.white
    static let selected = dynamic(0xdbe6ff, 0x27375f)
    static let related = dynamic(0xeff2fa, 0x22242d)
    static let same = dynamic(0xcddbff, 0x2f4273)
    static let error = dynamic(0xb42318, 0xffa59c)
    static let errorBackground = dynamic(0xfdecea, 0x3a1d1c)
    /// Healthy-connection dot.
    static let ok = dynamic(0x15803d, 0x4ade80)
    /// Reconnecting: neither healthy nor failed.
    static let warn = dynamic(0xa15c07, 0xfbbf24)
    static let rival = dynamic(0xa8500a, 0xf5b453)
    static let rivalSoft = dynamic(0xfbe6c4, 0x3b2c12)
    static let track = dynamic(0xe6e0d3, 0x2e2e35)

    /// Rounded display face for headings, digits and timers (the web's `ui-rounded`).
    static func display(_ style: Font.TextStyle, weight: Font.Weight = .bold) -> Font {
        .system(style, design: .rounded, weight: weight)
    }
}

/// A card surface: paper on the canvas with a hairline edge and a soft shadow.
struct CardBackground: ViewModifier {
    var radius: CGFloat = 22
    var fill: Color = Theme.background
    var stroke: Color = Theme.line

    func body(content: Content) -> some View {
        content
            .background(fill, in: RoundedRectangle(cornerRadius: radius, style: .continuous))
            .overlay { RoundedRectangle(cornerRadius: radius, style: .continuous).strokeBorder(stroke, lineWidth: 1) }
            .shadow(color: .black.opacity(0.05), radius: 7, y: 3)
    }
}

extension View {
    func card(radius: CGFloat = 22, fill: Color = Theme.background, stroke: Color = Theme.line) -> some View {
        modifier(CardBackground(radius: radius, fill: fill, stroke: stroke))
    }
}

/// Settings and remembered choices, kept in `UserDefaults` (preferences only; games have their own files).
/// Board size defaults to 9×9 and difficulty to Easy, as on the web, until the player picks something else.
@MainActor
@Observable
final class AppPreferences {
    enum Appearance: String, CaseIterable { case system, light, dark }
    nonisolated static let levels = ["beginner", "easy", "medium", "hard", "expert"]

    /// Every stored value lives in one ignored struct; each public property reports reads and writes to
    /// Observation itself (`access` / `withMutation`) and writes through to `UserDefaults`. This avoids
    /// relying on property observers inside the `@Observable` macro expansion.
    private struct Values {
        var appearance = Appearance.system
        var sound = false
        var haptics = true
        var soloSize = 9
        var soloDifficulty = "easy"
        var liveMode = "race"
        var liveSize = 9
        var liveDifficulty = "easy"
        var displayName = ""
    }
    @ObservationIgnored private var values = Values()
    @ObservationIgnored private let defaults: UserDefaults
    private static let prefix = "sudoku-rivals."

    var appearance: Appearance {
        get { access(keyPath: \.appearance); return values.appearance }
        set { withMutation(keyPath: \.appearance) { values.appearance = newValue }; save("appearance", newValue.rawValue) }
    }
    /// Interaction sounds: off by default, as on the web.
    var sound: Bool {
        get { access(keyPath: \.sound); return values.sound }
        set { withMutation(keyPath: \.sound) { values.sound = newValue }; save("sound", newValue) }
    }
    /// Taptic feedback: on by default, the iOS convention; the system "System Haptics" switch still applies.
    var haptics: Bool {
        get { access(keyPath: \.haptics); return values.haptics }
        set { withMutation(keyPath: \.haptics) { values.haptics = newValue }; save("haptics", newValue) }
    }
    var soloSize: Int {
        get { access(keyPath: \.soloSize); return values.soloSize }
        set { withMutation(keyPath: \.soloSize) { values.soloSize = newValue }; save("soloSize", newValue) }
    }
    var soloDifficulty: String {
        get { access(keyPath: \.soloDifficulty); return values.soloDifficulty }
        set { withMutation(keyPath: \.soloDifficulty) { values.soloDifficulty = newValue }; save("soloDifficulty", newValue) }
    }
    var liveMode: String {
        get { access(keyPath: \.liveMode); return values.liveMode }
        set { withMutation(keyPath: \.liveMode) { values.liveMode = newValue }; save("liveMode", newValue) }
    }
    var liveSize: Int {
        get { access(keyPath: \.liveSize); return values.liveSize }
        set { withMutation(keyPath: \.liveSize) { values.liveSize = newValue }; save("liveSize", newValue) }
    }
    var liveDifficulty: String {
        get { access(keyPath: \.liveDifficulty); return values.liveDifficulty }
        set { withMutation(keyPath: \.liveDifficulty) { values.liveDifficulty = newValue }; save("liveDifficulty", newValue) }
    }
    var displayName: String {
        get { access(keyPath: \.displayName); return values.displayName }
        set { withMutation(keyPath: \.displayName) { values.displayName = newValue }; save("displayName", newValue) }
    }

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        func string(_ key: String) -> String? { defaults.string(forKey: Self.prefix + key) }
        func size(_ key: String) -> Int { [6, 9].contains(defaults.integer(forKey: Self.prefix + key)) ? defaults.integer(forKey: Self.prefix + key) : 9 }
        func level(_ key: String) -> String { string(key).flatMap { Self.levels.contains($0) ? $0 : nil } ?? "easy" }
        values = Values(
            appearance: string("appearance").flatMap(Appearance.init(rawValue:)) ?? .system,
            sound: defaults.bool(forKey: Self.prefix + "sound"),
            haptics: defaults.object(forKey: Self.prefix + "haptics") as? Bool ?? true,
            soloSize: size("soloSize"),
            soloDifficulty: level("soloDifficulty"),
            liveMode: string("liveMode") == "duel" ? "duel" : "race",
            liveSize: size("liveSize"),
            liveDifficulty: level("liveDifficulty"),
            displayName: String((string("displayName") ?? "").prefix(16)))
    }

    var colorScheme: ColorScheme? {
        switch appearance {
        case .system: nil
        case .light: .light
        case .dark: .dark
        }
    }

    private func save(_ key: String, _ value: Any) { defaults.set(value, forKey: Self.prefix + key) }

    #if DEBUG
    /// UI tests start from the defaults.
    static func reset(_ defaults: UserDefaults = .standard) {
        for key in defaults.dictionaryRepresentation().keys where key.hasPrefix(prefix) { defaults.removeObject(forKey: key) }
    }
    #endif
}

/// Opt-in feedback for things the player did or that happened in a match. Haptics use the Taptic Engine
/// (silent when the system turns haptics off); sounds are short synthesized tones played through an
/// ambient audio session, so they respect the ring/silent switch and never interrupt other audio.
@MainActor
final class Feedback {
    enum Cue { case tap, note, select, error, success, attention, ready }

    static let shared = Feedback()
    var haptics = true
    var sound = false { didSet { if sound != oldValue { configureAudio() } } }

    private let impact = UIImpactFeedbackGenerator(style: .light)
    private let selection = UISelectionFeedbackGenerator()
    private let notification = UINotificationFeedbackGenerator()
    private var players: [Cue: AVAudioPlayer] = [:]

    func play(_ cue: Cue) {
        if haptics {
            switch cue {
            case .tap: impact.impactOccurred(intensity: 0.6)
            case .note, .select: selection.selectionChanged()
            case .error: notification.notificationOccurred(.error)
            case .success: notification.notificationOccurred(.success)
            case .attention: notification.notificationOccurred(.warning)
            case .ready: impact.impactOccurred(intensity: 0.9)
            }
        }
        if sound, let player = player(cue) {
            player.currentTime = 0
            player.play()
        }
    }

    private func configureAudio() {
        let session = AVAudioSession.sharedInstance()
        if sound {
            try? session.setCategory(.ambient)
            try? session.setActive(true)
        } else {
            for player in players.values { player.stop() }
            players.removeAll()
            try? session.setActive(false)
        }
    }

    private func player(_ cue: Cue) -> AVAudioPlayer? {
        if let player = players[cue] { return player }
        // (frequency Hz, duration s) per note; quiet, short and soft-edged.
        let notes: [(Double, Double)] = switch cue {
        case .tap: [(880, 0.035)]
        case .note: [(1320, 0.025)]
        case .select: [(660, 0.03)]
        case .error: [(220, 0.07), (180, 0.09)]
        case .success: [(660, 0.08), (880, 0.08), (1320, 0.14)]
        case .attention: [(740, 0.07), (988, 0.1)]
        case .ready: [(988, 0.06)]
        }
        guard let player = try? AVAudioPlayer(data: Self.wave(notes)) else { return nil }
        player.volume = 0.35
        player.prepareToPlay()
        players[cue] = player
        return player
    }

    /// 16-bit mono PCM WAV of sine notes, each with a short fade in and out (no clicks).
    nonisolated static func wave(_ notes: [(Double, Double)], rate: Double = 44_100) -> Data {
        var samples: [Int16] = []
        for (frequency, duration) in notes {
            let count = Int(duration * rate)
            let fade = min(Double(count) / 2, rate * 0.008)
            for i in 0..<count {
                let edge = min(1, Double(i) / fade, Double(count - i) / fade)
                samples.append(Int16(sin(2 * .pi * frequency * Double(i) / rate) * edge * 0.5 * Double(Int16.max)))
            }
        }
        var data = Data()
        func append<T: FixedWidthInteger>(_ value: T) { withUnsafeBytes(of: value.littleEndian) { data.append(contentsOf: $0) } }
        let bytes = UInt32(samples.count * 2)
        data.append(contentsOf: Array("RIFF".utf8)); append(36 + bytes)
        data.append(contentsOf: Array("WAVEfmt ".utf8)); append(UInt32(16)); append(UInt16(1)); append(UInt16(1))
        append(UInt32(rate)); append(UInt32(rate) * 2); append(UInt16(2)); append(UInt16(16))
        data.append(contentsOf: Array("data".utf8)); append(bytes)
        for sample in samples { append(sample) }
        return data
    }
}

/// The app mark drawn in SwiftUI (same geometry as `icon.svg`): blue tile, amber card behind a white one,
/// a 3×3 grid and an amber centre cell.
struct AppMark: View {
    var side: CGFloat = 34

    var body: some View {
        Canvas { context, size in
            let s = size.width / 192
            func rect(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat) -> CGRect { CGRect(x: x * s, y: y * s, width: w * s, height: h * s) }
            let blue = Color(red: 0x1d / 255, green: 0x4e / 255, blue: 0xd8 / 255)
            let amber = Color(red: 0xf5 / 255, green: 0xb4 / 255, blue: 0x53 / 255)
            context.fill(Path(rect(0, 0, 192, 192)), with: .color(blue))
            var behind = context
            behind.translateBy(x: 96 * s, y: 96 * s)
            behind.rotate(by: .degrees(12))
            behind.translateBy(x: -96 * s + 11 * s, y: -96 * s - 13 * s)
            behind.fill(Path(roundedRect: rect(39, 39, 114, 114), cornerRadius: 25 * s), with: .color(amber))
            context.fill(Path(roundedRect: rect(39, 39, 114, 114), cornerRadius: 25 * s), with: .color(.white))
            for line in [rect(72.33, 39, 7, 114), rect(112.67, 39, 7, 114), rect(39, 72.33, 114, 7), rect(39, 112.67, 114, 7)] {
                context.fill(Path(line), with: .color(blue))
            }
            context.fill(Path(rect(79.33, 79.33, 33.33, 33.33)), with: .color(amber))
        }
        .frame(width: side, height: side)
        .clipShape(RoundedRectangle(cornerRadius: side * 0.26, style: .continuous))
        .accessibilityHidden(true)
    }
}
