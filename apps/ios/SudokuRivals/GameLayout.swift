import SwiftUI

/// Developer-only extra text on the game screens (app title bar, selected-cell line, keyboard help,
/// "game in progress", "saved", duel rules). Off by default; only Debug builds honour the
/// `-SudokuDebug` launch argument, so Staging and Release can never switch it on. Not persisted.
enum UIDebug {
    static let enabled: Bool = {
        #if DEBUG
        ProcessInfo.processInfo.arguments.contains("-SudokuDebug")
        #else
        false
        #endif
    }()
}

/// Use the available safe-area height so navigation and system bars count toward the layout.
enum GameDensity: Equatable {
    case compact, standard, tallPhone

    var gap: CGFloat { self == .compact ? 6 : 8 }
    var padding: CGFloat { self == .compact ? 4 : 8 }
    var minimumBoard: CGFloat { self == .compact ? 240 : 280 }

    func keyHeight(size: Int) -> CGFloat {
        switch self {
        case .compact: 48
        case .standard: 52
        case .tallPhone: size == 6 ? 76 : 60
        }
    }
}

private struct GameDensityKey: EnvironmentKey {
    static let defaultValue = GameDensity.standard
}

extension EnvironmentValues {
    var gameDensity: GameDensity {
        get { self[GameDensityKey.self] }
        set { self[GameDensityKey.self] = newValue }
    }
}

/// One-screen game layout shared by solo, race and duel (6×6 and 9×9), same order as web and Android:
/// `top` (navigation row + one status row), the board (largest square the remaining height allows),
/// then `bottom` (digits, tools, transient notices). Landscape puts top+bottom in a side panel. When even
/// the minimum board cannot fit (very small screens, accessibility text sizes) the page scrolls
/// instead of clipping a control.
struct GameScreenLayout<Top: View, Board: View, Bottom: View>: View {
    let top: (GameDensity) -> Top
    let board: Board
    let bottom: Bottom
    @Environment(\.dynamicTypeSize) private var typeSize

    init(@ViewBuilder top: @escaping (GameDensity) -> Top,
         @ViewBuilder board: () -> Board, @ViewBuilder bottom: () -> Bottom) {
        self.top = top
        self.board = board()
        self.bottom = bottom()
    }

    var body: some View {
        GeometryReader { proxy in
            let landscape = proxy.size.width >= 560 && proxy.size.width > proxy.size.height * 1.2
            let tallPhone = !landscape && proxy.size.width < 560 && proxy.size.height >= 740 && !typeSize.isAccessibilitySize
            let density: GameDensity = tallPhone ? .tallPhone : proxy.size.height <= 600 ? .compact : .standard
            ViewThatFits(in: .vertical) {
                layout(landscape: landscape, density: density)
                ScrollView { layout(landscape: landscape, density: density, scrolls: true) }
            }
            .padding(.horizontal, 12)
        }
    }

    private func layout(landscape: Bool, density: GameDensity, scrolls: Bool = false) -> some View {
        GameLayout(landscape: landscape, density: density, scrolls: scrolls) { top(density); board; bottom }
            .environment(\.gameDensity, density)
    }
}

/// Three subviews in, one column (or board + side panel) out. `scrolls == false` fills the proposed
/// height; `scrolls == true` is the content height for a `ScrollView` fallback.
struct GameLayout: Layout {
    let landscape: Bool
    let density: GameDensity
    let scrolls: Bool

    private let maxBoard: CGFloat = 560
    private let panel: CGFloat = 300
    private var gap: CGFloat { density.gap }
    private var pad: CGFloat { density.padding }

    private func boardRange(width: CGFloat) -> ClosedRange<CGFloat> {
        let maximum = min(maxBoard, landscape ? width - panel - gap : width)
        let minimum = landscape ? 240 : density.minimumBoard
        return min(minimum, maximum)...maximum
    }

    private func measure(_ subviews: Subviews, width: CGFloat) -> (top: CGFloat, bottom: CGFloat) {
        (subviews[0].sizeThatFits(.init(width: width, height: nil)).height,
         subviews[2].sizeThatFits(.init(width: width, height: nil)).height)
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? 390
        let range = boardRange(width: width)
        let board = scrolls ? range.upperBound : range.lowerBound
        let heights = measure(subviews, width: landscape ? panel : width)
        let controls = heights.top + gap + heights.bottom
        let minimumHeight = 2 * pad + (landscape ? max(board, controls) : board + gap + controls)
        return CGSize(width: width, height: scrolls ? minimumHeight : max(proposal.height ?? 0, minimumHeight))
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let width = bounds.width
        let range = boardRange(width: width)
        let heights = measure(subviews, width: landscape ? panel : width)
        let panelHeight = heights.top + gap + heights.bottom
        let free = bounds.height - 2 * pad - (landscape ? 0 : panelHeight + gap)
        let board = scrolls ? range.upperBound : min(max(free, range.lowerBound), range.upperBound)
        let boardSize = ProposedViewSize(width: board, height: board)
        if landscape {
            let left = bounds.midX - (board + gap + panel) / 2
            let x = left + board + gap
            let y = bounds.midY - panelHeight / 2
            let panelSize = ProposedViewSize(width: panel, height: nil)
            subviews[0].place(at: CGPoint(x: x, y: y), anchor: .topLeading, proposal: panelSize)
            subviews[1].place(at: CGPoint(x: left, y: bounds.midY - board / 2), anchor: .topLeading, proposal: boardSize)
            subviews[2].place(at: CGPoint(x: x, y: y + heights.top + gap), anchor: .topLeading, proposal: panelSize)
            return
        }
        let columnSize = ProposedViewSize(width: width, height: nil)
        // Keep the controls next to the board and share any spare height above and below them.
        let spare = max(0, free - board)
        let topY = bounds.minY + pad
        let boardY = topY + heights.top + gap + spare / 2
        subviews[0].place(at: CGPoint(x: bounds.minX, y: topY), anchor: .topLeading, proposal: columnSize)
        subviews[1].place(at: CGPoint(x: bounds.midX - board / 2, y: boardY), anchor: .topLeading, proposal: boardSize)
        subviews[2].place(at: CGPoint(x: bounds.minX, y: boardY + board + gap), anchor: .topLeading, proposal: columnSize)
    }
}

/// Navigation row: back · `Mode · N×N` + time · trailing slot (solo: pause).
struct GameNavBar<Time: View, Trailing: View>: View {
    let mode: String
    let back: () -> Void
    let time: Time
    let trailing: Trailing
    @Environment(\.dynamicTypeSize) private var typeSize

    init(mode: String, back: @escaping () -> Void, @ViewBuilder time: () -> Time, @ViewBuilder trailing: () -> Trailing) {
        self.mode = mode
        self.back = back
        self.time = time()
        self.trailing = trailing()
    }

    var body: some View {
        HStack(spacing: 8) {
            Button(action: back) {
                Image(systemName: "chevron.backward").font(.body.weight(.semibold)).frame(width: 48, height: 48)
            }
            .buttonStyle(PressStyle(fill: Theme.background, foreground: Theme.foreground, stroke: Theme.line, radius: 14))
            .accessibilityLabel(L10n.t("nav.back"))
            .accessibilityIdentifier("back")
            .frame(width: 48, alignment: .leading)
            Spacer(minLength: 0)
            // "SOLO · 9×9 · EASY" over a large timer, as on the web.
            VStack(spacing: 0) {
                Text(mode.uppercased(with: L10n.turkish ? Locale(identifier: "tr") : Locale(identifier: "en")))
                    .font(.caption.weight(.semibold)).tracking(0.9).foregroundStyle(Theme.muted)
                    .lineLimit(typeSize.isAccessibilitySize ? nil : 1).minimumScaleFactor(0.8)
                    .multilineTextAlignment(.center)
                    .accessibilityLabel(mode)
                    .accessibilityAddTraits(.isHeader)
                time
            }
            Spacer(minLength: 0)
            trailing.frame(width: 48, height: 48, alignment: .trailing)
        }
        .frame(minHeight: 48)
    }
}

/// A thin progress bar: filled share of a board, or the duel's remaining time.
struct Meter: View {
    let value: Double
    var tint: Color = Theme.accent
    var leading = true

    var body: some View {
        Capsule().fill(Theme.track)
            .overlay(alignment: leading ? .leading : .trailing) {
                GeometryReader { proxy in
                    Capsule().fill(tint)
                        .frame(width: proxy.size.width * min(max(value, 0), 1))
                        .frame(maxWidth: .infinity, alignment: leading ? .leading : .trailing)
                }
            }
            .frame(height: 6)
            .accessibilityHidden(true)
    }
}

/// The one status row: fixed minimum height so a message appearing never moves the board.
struct GameStatusRow<Content: View>: View {
    let content: Content
    @Environment(\.gameDensity) private var density
    init(@ViewBuilder content: () -> Content) { self.content = content() }

    var body: some View {
        content
            .font(.subheadline)
            .frame(maxWidth: .infinity, minHeight: density == .compact ? 18 : 24)
    }
}

/// A status message. In its neutral state ("game in progress") it stays in the accessibility tree
/// with its identifier but is invisible unless the debug flag is on, like the web's screen-reader-only text.
struct StatusMessage: View {
    let text: String
    let identifier: String
    var neutral = false
    var alert = false

    var body: some View {
        if neutral && !UIDebug.enabled {
            Color.clear.frame(width: 1, height: 1)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(text)
                .accessibilityIdentifier(identifier)
        } else {
            Text(text)
                .foregroundStyle(alert ? Theme.error : Theme.foreground)
                .accessibilityIdentifier(identifier)
        }
    }
}

/// Six digits use two rows. Nine use three rows on tall phones and two where height is limited.
struct DigitPadLayout: Layout {
    let count: Int
    let rowHeight: CGFloat
    let density: GameDensity

    private var spacing: CGFloat { density.gap }

    private func columns(_ width: CGFloat) -> Int {
        if count == 6 || density == .tallPhone || width < 5 * 48 + 4 * spacing { return 3 }
        return 5
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? 320
        let cols = columns(width)
        let rows = (count + cols - 1) / cols
        return CGSize(width: width, height: CGFloat(rows) * rowHeight + CGFloat(rows - 1) * spacing)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let cols = columns(bounds.width)
        let cell = (bounds.width - CGFloat(cols - 1) * spacing) / CGFloat(cols)
        for (index, subview) in subviews.enumerated() {
            let row = index / cols, column = index % cols
            let items = min(cols, count - row * cols)
            let rowWidth = CGFloat(items) * cell + CGFloat(items - 1) * spacing
            let x = bounds.minX + (bounds.width - rowWidth) / 2 + CGFloat(column) * (cell + spacing)
            let y = bounds.minY + CGFloat(row) * (rowHeight + spacing)
            subview.place(at: CGPoint(x: x, y: y), anchor: .topLeading, proposal: .init(width: cell, height: rowHeight))
        }
    }
}
