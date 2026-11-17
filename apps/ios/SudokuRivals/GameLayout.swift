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

/// One-screen game layout shared by solo, race and duel (6×6 and 9×9), same order as web and Android:
/// `top` (navigation row + one status row), the board (largest square the remaining height allows),
/// then `bottom` (digits, tools, transient notices). Landscape puts top+bottom in a side panel. When even
/// the minimum board cannot fit (very small screens, accessibility text sizes) the page scrolls
/// instead of clipping a control.
struct GameScreenLayout<Top: View, Board: View, Bottom: View>: View {
    let top: Top
    let board: Board
    let bottom: Bottom

    init(@ViewBuilder top: () -> Top, @ViewBuilder board: () -> Board, @ViewBuilder bottom: () -> Bottom) {
        self.top = top()
        self.board = board()
        self.bottom = bottom()
    }

    var body: some View {
        GeometryReader { proxy in
            let landscape = proxy.size.width >= 560 && proxy.size.width > proxy.size.height * 1.2
            ViewThatFits(in: .vertical) {
                GameLayout(landscape: landscape, scrolls: false) { top; board; bottom }
                ScrollView { GameLayout(landscape: landscape, scrolls: true) { top; board; bottom } }
            }
            .padding(.horizontal, 12)
        }
    }
}

/// Three subviews in, one column (or board + side panel) out. `scrolls == false` fills the proposed
/// height; `scrolls == true` is the content height for a `ScrollView` fallback.
struct GameLayout: Layout {
    let landscape: Bool
    let scrolls: Bool

    static let minBoard: CGFloat = 280
    static let minBoardLandscape: CGFloat = 240
    static let maxBoard: CGFloat = 560
    static let gap: CGFloat = 8
    static let pad: CGFloat = 8
    static let panel: CGFloat = 300

    private func measure(_ subviews: Subviews, width: CGFloat) -> (top: CGFloat, bottom: CGFloat) {
        (subviews[0].sizeThatFits(.init(width: width, height: nil)).height,
         subviews[2].sizeThatFits(.init(width: width, height: nil)).height)
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? 390
        if let height = proposal.height, !scrolls { return CGSize(width: width, height: height) }
        if landscape {
            let panelHeights = measure(subviews, width: Self.panel)
            let board = scrolls ? min(width - Self.panel - Self.gap, Self.maxBoard) : Self.minBoardLandscape
            return CGSize(width: width, height: max(board, panelHeights.top + Self.gap + panelHeights.bottom) + 2 * Self.pad)
        }
        let heights = measure(subviews, width: width)
        let board = min(width, scrolls ? Self.maxBoard : Self.minBoard)
        return CGSize(width: width, height: 2 * Self.pad + heights.top + Self.gap + board + Self.gap + heights.bottom)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let width = bounds.width
        if landscape {
            let heights = measure(subviews, width: Self.panel)
            let side = min(bounds.height - 2 * Self.pad, width - Self.panel - Self.gap, Self.maxBoard)
            let board = max(side, Self.minBoardLandscape)
            let used = board + Self.gap + Self.panel
            let left = bounds.minX + max(0, (width - used) / 2)
            subviews[1].place(at: CGPoint(x: left, y: bounds.minY + max(Self.pad, (bounds.height - board) / 2)),
                              anchor: .topLeading, proposal: .init(width: board, height: board))
            let panelHeight = heights.top + Self.gap + heights.bottom
            var y = bounds.minY + max(Self.pad, (bounds.height - panelHeight) / 2)
            let x = left + board + Self.gap
            subviews[0].place(at: CGPoint(x: x, y: y), anchor: .topLeading, proposal: .init(width: Self.panel, height: nil))
            y += heights.top + Self.gap
            subviews[2].place(at: CGPoint(x: x, y: y), anchor: .topLeading, proposal: .init(width: Self.panel, height: nil))
            return
        }
        let heights = measure(subviews, width: width)
        let free = bounds.height - 2 * Self.pad - heights.top - heights.bottom - 2 * Self.gap
        let fits = !scrolls && free >= Self.minBoard
        let board = min(width, Self.maxBoard, scrolls ? Self.maxBoard : max(free, Self.minBoard))
        let content = Self.pad + heights.top + Self.gap + board + Self.gap + heights.bottom + Self.pad
        // Navigation stays at the top; spare height (a board capped by width) is split above the board and
        // below it, so the controls stay next to the board.
        let spare = fits ? max(0, bounds.height - content) : 0
        var y = bounds.minY + Self.pad
        subviews[0].place(at: CGPoint(x: bounds.minX, y: y), anchor: .topLeading, proposal: .init(width: width, height: nil))
        y += heights.top + Self.gap + spare / 2
        subviews[1].place(at: CGPoint(x: bounds.minX + (width - board) / 2, y: y), anchor: .topLeading,
                          proposal: .init(width: board, height: board))
        y += board + Self.gap
        subviews[2].place(at: CGPoint(x: bounds.minX, y: y), anchor: .topLeading, proposal: .init(width: width, height: nil))
    }
}

/// Navigation row: back · `Mode · N×N` + time · trailing slot (solo: pause; live: rival name).
struct GameNavBar<Time: View, Trailing: View>: View {
    let mode: String
    let back: () -> Void
    let time: Time
    let trailing: Trailing

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
            .frame(minWidth: 72, alignment: .leading)
            Spacer(minLength: 0)
            // "SOLO · 9×9 · EASY" over a large timer, as on the web.
            VStack(spacing: 0) {
                Text(mode.uppercased(with: L10n.turkish ? Locale(identifier: "tr") : Locale(identifier: "en")))
                    .font(.caption.weight(.semibold)).tracking(0.9).foregroundStyle(Theme.muted)
                    .lineLimit(1).minimumScaleFactor(0.8)
                    .accessibilityLabel(mode)
                    .accessibilityAddTraits(.isHeader)
                time
            }
            Spacer(minLength: 0)
            trailing.frame(minWidth: 72, minHeight: 48, alignment: .trailing)
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
    init(@ViewBuilder content: () -> Content) { self.content = content() }

    var body: some View {
        content
            .font(.subheadline)
            .frame(maxWidth: .infinity, minHeight: 24)
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
        Text(text)
            .foregroundStyle(alert ? Theme.error : Theme.foreground)
            .opacity(neutral && !UIDebug.enabled ? 0 : 1)
            .accessibilityIdentifier(identifier)
    }
}

/// Digits: 6×6 one row (3×2 when narrower than 328 pt), 9×9 as 5+4 with the second row centred.
struct DigitPadLayout: Layout {
    let count: Int
    let rowHeight: CGFloat
    static let spacing: CGFloat = 8
    static let minCell: CGFloat = 48

    private func columns(_ width: CGFloat) -> Int {
        if count > 6 { return 5 }
        return width >= CGFloat(count) * Self.minCell + CGFloat(count - 1) * Self.spacing ? count : 3
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let width = proposal.width ?? 320
        let rows = (count + columns(width) - 1) / columns(width)
        return CGSize(width: width, height: CGFloat(rows) * rowHeight + CGFloat(rows - 1) * Self.spacing)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let cols = columns(bounds.width)
        let cell = (bounds.width - CGFloat(cols - 1) * Self.spacing) / CGFloat(cols)
        for (index, subview) in subviews.enumerated() {
            let row = index / cols, column = index % cols
            let items = min(cols, count - row * cols)
            let rowWidth = CGFloat(items) * cell + CGFloat(items - 1) * Self.spacing
            let x = bounds.minX + (bounds.width - rowWidth) / 2 + CGFloat(column) * (cell + Self.spacing)
            let y = bounds.minY + CGFloat(row) * (rowHeight + Self.spacing)
            subview.place(at: CGPoint(x: x, y: y), anchor: .topLeading, proposal: .init(width: cell, height: rowHeight))
        }
    }
}
