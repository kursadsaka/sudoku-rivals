import SwiftUI

struct GameScreen: View {
    let app: AppModel
    let session: SoloSession

    var body: some View {
        // While paused the board is not rendered, so it also leaves the accessibility tree.
        if session.complete {
            ResultView(app: app, session: session)
        } else if session.paused {
            PausedView(app: app, session: session)
        } else {
            PlayingView(app: app, session: session)
        }
    }
}

/// "9×9 · Easy": the size and, for rated puzzles, the level.
func sizeAndLevel(_ size: Int, _ level: String?) -> String {
    ["\(size)×\(size)", level.map { L10n.t("level.\($0)") }].compactMap { $0 }.joined(separator: " · ")
}

/// While the game is saved and the debug flag is off, "saved" is not drawn but stays reachable for screen
/// readers (a 1 pt element with the same identifier), like the web's screen-reader-only text.
struct QuietSaveMarker: View {
    let session: SoloSession

    var body: some View {
        if session.saveState == .saved && !UIDebug.enabled {
            Color.clear.frame(width: 1, height: 1)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(L10n.t("saved"))
                .accessibilityIdentifier("storage-status")
        }
    }
}

struct StorageStatus: View {
    let session: SoloSession

    var body: some View {
        if session.saveState == .saved {
            Label(L10n.t("saved"), systemImage: "checkmark.circle")
                .font(.footnote).foregroundStyle(Theme.muted)
                .accessibilityIdentifier("storage-status")
        } else {
            VStack(alignment: .leading, spacing: 6) {
                Label(L10n.t("error.save"), systemImage: "exclamationmark.triangle")
                Button(L10n.t("error.retry")) { session.save() }
                    .font(.subheadline.weight(.semibold)).underline()
                    .frame(minHeight: 44)
                    .accessibilityIdentifier("retry")
            }
            .font(.subheadline)
            .foregroundStyle(Theme.error)
            .padding(.horizontal, 12).padding(.vertical, 6)
            .frame(maxWidth: .infinity, alignment: .leading)
            .card(radius: 14, fill: Theme.errorBackground, stroke: Theme.error)
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("storage-status")
        }
    }
}

struct PlayingView: View {
    let app: AppModel
    @Bindable var session: SoloSession
    @State private var hint: LogicalHint?
    @State private var notice: String?
    @Environment(\.dynamicTypeSize) private var typeSize
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        GameScreenLayout {
            VStack(spacing: 6) { navBar; Meter(value: progress); statusRow }
                .overlay(alignment: .top) { toast }
        } board: {
            BoardView(session: session, pattern: hint?.patternCells ?? [])
        } bottom: {
            VStack(spacing: 8) { controls }
        }
        .toolbar(UIDebug.enabled ? .visible : .hidden, for: .navigationBar)
        .background { ArrowKeys { session.move(rowDelta: $0, columnDelta: $1); dismissLayers() } }
        .background { DeleteKeyCatcher { if session.canEdit { report(session.erase()) } }.accessibilityHidden(true) }
        .onChange(of: session.game) { hint = nil }
        .onChange(of: session.selected) { if hint.map({ $0.index != session.selected }) ?? false { hint = nil } }
    }

    /// Share of the empty cells that now hold a number (the same measure shown for a rival).
    private var progress: Double {
        let board = session.board
        let blanks = board.givens.indices.filter { board.givens[$0] == 0 }
        guard !blanks.isEmpty else { return 0 }
        return Double(blanks.filter { board.cells[$0].value != 0 }.count) / Double(blanks.count)
    }

    private var navBar: some View {
        GameNavBar(mode: "\(L10n.t("mode.solo")) · \(sizeAndLevel(session.size, session.level))", back: { app.leaveGame() }) {
            timer
        } trailing: {
            Button { session.pause(); app.cue(.select) } label: {
                Image(systemName: "pause.fill").font(.body).frame(width: 48, height: 48)
            }
            .buttonStyle(PressStyle(fill: Theme.background, foreground: Theme.foreground, stroke: Theme.line, radius: 14))
            .accessibilityLabel(L10n.t("game.pause"))
            .accessibilityIdentifier("pause")
        }
    }

    private var timer: some View {
        TimelineView(.periodic(from: .now, by: 1)) { _ in
            let text = L10n.t("game.elapsed", ["time": L10n.time(session.elapsedMs)])
            Text(L10n.time(session.elapsedMs)).font(Theme.display(.title2).monospacedDigit())
                .foregroundStyle(Theme.foreground)
                .accessibilityLabel(text)
                .accessibilityIdentifier("timer")
        }
    }

    private var statusRow: some View {
        let conflict = !session.conflicts.isEmpty
        return GameStatusRow {
            if conflict {
                Label { StatusMessage(text: L10n.t("game.conflict"), identifier: "game-status", alert: true) }
                    icon: { Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(Theme.error) }
                    .font(.subheadline.weight(.semibold))
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("game-status")
            } else {
                StatusMessage(text: L10n.t("game.incomplete"), identifier: "game-status", neutral: true)
            }
        }
    }

    /// A passing message floats over the title and timer (between back and pause); nothing below moves.
    @ViewBuilder private var toast: some View {
        if let notice {
            HStack(spacing: 2) {
                Text(notice).font(.footnote).frame(maxWidth: .infinity, alignment: .leading)
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityIdentifier("game-notice")
                Button { self.notice = nil } label: {
                    Image(systemName: "xmark").font(.footnote.weight(.semibold)).frame(width: 40, height: 44)
                }
                .accessibilityLabel(L10n.t("hint.dismiss"))
            }
            .foregroundStyle(Theme.canvas)
            .padding(.leading, 12)
            .frame(minHeight: 48)
            .background(Theme.foreground, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .padding(.horizontal, 56)
            .transition(reduceMotion ? .opacity : .move(edge: .top).combined(with: .opacity))
        }
    }

    @ViewBuilder private var controls: some View {
        if UIDebug.enabled { selectionLine }
        VStack(spacing: 8) {
            NumberPad(size: session.size, values: session.board.cells.map(\.value), notesMode: session.notesMode,
                      enabled: session.canEdit) { enter($0) }
            tools.overlay(alignment: .bottomLeading) { QuietSaveMarker(session: session) }
        }
        // The hint is laid over the keys, so the board and the keys keep their place and size.
        .overlay(alignment: .bottom) {
            if let hint {
                hintCard(hint)
                    .transition(reduceMotion ? .opacity : .move(edge: .bottom).combined(with: .opacity))
            }
        }
        if session.saveState == .failed || UIDebug.enabled { StorageStatus(session: session) }
        if UIDebug.enabled {
            Text(L10n.t("keyboard", ["size": session.size])).font(.footnote).foregroundStyle(Theme.muted)
                .accessibilityHidden(true)
        }
    }

    private var selectionLine: some View {
        let n = session.size, index = session.selected
        return Text(session.board.isGiven(index) ? L10n.t("selection.given")
                    : L10n.t("selection", ["row": index / n + 1, "column": index % n + 1]))
            .font(.subheadline).foregroundStyle(Theme.muted)
            .accessibilityHidden(true) // The selected cell already announces this.
    }

    private var tools: some View {
        let layout = typeSize.isAccessibilitySize
            ? AnyLayout(VStackLayout(alignment: .leading, spacing: 8)) : AnyLayout(HStackLayout(spacing: 4))
        return layout {
            NotesToggle(isOn: $session.notesMode) { app.cue(.select) }
            ToolButton(title: L10n.t("game.erase"), icon: "eraser", id: "erase", enabled: session.canEdit) {
                report(session.erase(), cue: .tap)
            }
            ToolButton(title: L10n.t("game.undo"), icon: "arrow.uturn.backward", id: "undo", enabled: !session.board.undo.isEmpty) {
                report(session.undo(), cue: .tap)
            }
            .keyboardShortcut("z", modifiers: .command)
            ToolButton(title: L10n.t("game.redo"), icon: "arrow.uturn.forward", id: "redo", enabled: !session.board.redo.isEmpty) {
                report(session.redo(), cue: .tap)
            }
            .keyboardShortcut("z", modifiers: [.command, .shift])
            ToolButton(title: L10n.t("game.hint"), icon: "lightbulb", id: "hint", enabled: true) { showHint() }
        }
    }

    private func hintCard(_ hint: LogicalHint) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            ScrollView {
                HStack(alignment: .top, spacing: 8) {
                    Image(systemName: "lightbulb").foregroundStyle(Theme.accent).accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 4) {
                        ForEach(Array(hintLines(hint).enumerated()), id: \.offset) { _, line in Text(line) }
                    }
                    .font(.subheadline)
                    .foregroundStyle(Theme.foreground)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .accessibilityElement(children: .combine)
                    .accessibilityIdentifier("hint-explanation")
                }
            }
            .scrollBounceBehavior(.basedOnSize)
            HStack(spacing: 8) {
                Button { self.hint = nil } label: {
                    Text(L10n.t("hint.dismiss")).font(.subheadline.weight(.semibold)).padding(.horizontal, 12).frame(minHeight: 44)
                }
                .buttonStyle(PressStyle(fill: Theme.background, foreground: Theme.foreground, stroke: Theme.border, radius: 12))
                .accessibilityIdentifier("hint-dismiss")
                Button { apply(hint) } label: {
                    Text(L10n.t("hint.apply")).font(.subheadline.weight(.semibold)).frame(maxWidth: .infinity, minHeight: 44)
                }
                .buttonStyle(PressStyle(fill: Theme.brand, foreground: Theme.onBrand, stroke: Theme.brand, radius: 12))
                .accessibilityIdentifier("hint-apply")
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .background(Theme.background, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
        .overlay { RoundedRectangle(cornerRadius: 16, style: .continuous).strokeBorder(Theme.accent, lineWidth: 1.5) }
        .shadow(color: .black.opacity(0.08), radius: 8, y: 3)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(L10n.t("game.hint"))
        .accessibilityIdentifier("hint-card")
    }

    /// One sentence per elimination the placement depends on, then the placement itself (as on the web).
    private func hintLines(_ hint: LogicalHint) -> [String] {
        let n = session.size
        func unit(_ value: LogicalHint.Unit) -> String { L10n.t("hint.\(value.kind)", ["number": value.index + 1]) }
        let steps = (hint.eliminations ?? []).map { step -> String in
            if step.technique == "locked_candidates" {
                return L10n.t("hint.locked", ["unit": unit(step.unit), "target": step.target.map(unit) ?? "",
                                              "value": step.digits.first ?? 0])
            }
            return L10n.t(step.technique == "naked_pair" ? "hint.nakedPair" : "hint.hiddenPair",
                          ["unit": unit(step.unit), "a": step.digits.first ?? 0, "b": step.digits.dropFirst().first ?? 0])
        }
        let key = (hint.technique == "naked_single" ? "hint.naked" : "hint.hidden") + (steps.isEmpty ? "" : "Then")
        let last = L10n.t(key, ["row": hint.index / n + 1, "column": hint.index % n + 1, "value": hint.value,
                                "unit": hint.unit.map(unit) ?? ""])
        return steps + [last]
    }

    private func showHint() {
        dismissLayers()
        switch session.hint() {
        case .hint(let found):
            withAnimation(reduceMotion ? nil : .easeOut(duration: 0.2)) { hint = found }
            session.selected = found.index
            app.cue(.ready)
            announce(hintLines(found).joined(separator: " "))
        case .noneWithSingles: say(L10n.t("hint.none"), cue: nil)
        case .conflicts: say(L10n.t("hint.conflicts"), cue: nil)
        case .unsolvable: say(L10n.t("hint.unsolvable"), cue: nil)
        case .searchLimit: say(L10n.t("hint.searchLimit"), cue: nil)
        }
    }

    private func enter(_ digit: Int) { afterEntry(session.enter(digit), notes: session.notesMode) }

    /// An applied hint is an ordinary, undoable set.
    private func apply(_ hint: LogicalHint) { afterEntry(session.applyHint(hint), notes: false) }

    private func afterEntry(_ reason: String?, notes: Bool) {
        if reason == nil {
            app.cue(session.complete ? .success
                    : !notes && session.conflicts.contains(session.selected) ? .error : notes ? .note : .tap)
        }
        report(reason)
    }

    private func report(_ reason: String?, cue: Feedback.Cue? = nil) {
        switch reason {
        case nil:
            dismissLayers()
            if let cue { app.cue(cue) }
        case "GivenCell": say(L10n.t("error.given"))
        case "FilledCell": say(L10n.t("error.filled"))
        case "NotPlaying": break
        default: say(L10n.t("error.generic"))
        }
    }

    private func dismissLayers() {
        notice = nil
        hint = nil
    }

    private func say(_ text: String, cue: Feedback.Cue? = .error) {
        withAnimation(reduceMotion ? nil : .easeOut(duration: 0.2)) { notice = text }
        if let cue { app.cue(cue) }
        announce(text)
    }
    private func announce(_ text: String) { AccessibilityNotification.Announcement(text).post() }
}

/// Arrow keys have no on-screen button; these invisible buttons carry the shortcuts.
struct ArrowKeys: View {
    let move: (Int, Int) -> Void

    var body: some View {
        ZStack {
            Button("") { move(-1, 0) }.keyboardShortcut(.upArrow, modifiers: [])
            Button("") { move(1, 0) }.keyboardShortcut(.downArrow, modifiers: [])
            Button("") { move(0, -1) }.keyboardShortcut(.leftArrow, modifiers: [])
            Button("") { move(0, 1) }.keyboardShortcut(.rightArrow, modifiers: [])
        }
        .opacity(0)
        .accessibilityHidden(true)
    }
}

struct ToolLabel: View {
    let title: String
    let icon: String
    var body: some View {
        VStack(spacing: 2) {
            Image(systemName: icon).font(.title3).accessibilityHidden(true)
            Text(title).font(.caption2.weight(.medium)).lineLimit(1).minimumScaleFactor(0.8)
        }
        .frame(maxWidth: .infinity, minHeight: 52)
    }
}

/// Tools are quiet: icon over a small label on no background; the active notes switch is filled.
struct ToolStyle: ButtonStyle {
    var on = false
    @Environment(\.isEnabled) private var enabled

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .foregroundStyle(on ? Theme.onBrand : Theme.foreground)
            .background(on ? Theme.brand : configuration.isPressed ? Theme.selected : .clear,
                        in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .opacity(enabled ? 1 : 0.4)
            .contentShape(Rectangle())
    }
}

struct ToolButton: View {
    let title: String
    let icon: String
    let id: String
    let enabled: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) { ToolLabel(title: title, icon: icon) }
            .buttonStyle(ToolStyle())
            .disabled(!enabled)
            .accessibilityIdentifier(id)
    }
}

/// The notes (pencil) switch: a toggle, so VoiceOver and tests read its state as a value.
struct NotesToggle: View {
    @Binding var isOn: Bool
    var enabled = true
    let changed: () -> Void

    var body: some View {
        Toggle(isOn: Binding(get: { isOn }, set: { isOn = $0; changed() })) {
            ToolLabel(title: L10n.t("game.notes"), icon: "pencil")
        }
        .toggleStyle(.button)
        .buttonStyle(ToolStyle(on: isOn))
        .disabled(!enabled)
        .keyboardShortcut("n", modifiers: [])
        .accessibilityIdentifier("notes-toggle")
    }
}

/// A key: paper with a thicker bottom edge; each key shows how many of its digit are still missing,
/// and a finished digit steps back. In notes mode the keys turn to dashed pencil keys.
struct KeyStyle: ButtonStyle {
    var notes = false
    var done = false
    @Environment(\.isEnabled) private var enabled
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func makeBody(configuration: Configuration) -> some View {
        let shape = RoundedRectangle(cornerRadius: 14, style: .continuous)
        return configuration.label
            .foregroundStyle(notes || done ? Theme.muted : Theme.accent)
            .background(done ? Color.clear : configuration.isPressed ? Theme.selected : Theme.background, in: shape)
            .overlay {
                shape.strokeBorder(Theme.line, style: StrokeStyle(lineWidth: 1, dash: notes ? [4, 3] : []))
            }
            .overlay(alignment: .bottom) {
                if !notes && !done { Capsule().fill(Theme.line).frame(height: 2).padding(.horizontal, 6).accessibilityHidden(true) }
            }
            .opacity(enabled ? 1 : 0.45)
            .scaleEffect(configuration.isPressed && !reduceMotion ? 0.97 : 1)
    }
}

struct NumberPad: View {
    let size: Int
    /// Current cell values, for the "left" counts.
    let values: [Int]
    let notesMode: Bool
    let enabled: Bool
    let enter: (Int) -> Void
    @Environment(\.dynamicTypeSize) private var typeSize

    var body: some View {
        let placed = values.reduce(into: Array(repeating: 0, count: size + 1)) { counts, value in
            if (1...size).contains(value) { counts[value] += 1 }
        }
        DigitPadLayout(count: size, rowHeight: typeSize.isAccessibilitySize ? 72 : 52) {
            ForEach(1...size, id: \.self) { digit in
                let left = max(0, size - placed[digit])
                Button { enter(digit) } label: {
                    Text("\(digit)")
                        .font(Theme.display(notesMode ? .title2 : .title, weight: notesMode ? .medium : .semibold))
                        .dynamicTypeSize(...DynamicTypeSize.xxxLarge)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .overlay(alignment: .topTrailing) {
                            if left > 0 {
                                Text("\(left)").font(.caption2.weight(.semibold)).foregroundStyle(Theme.muted)
                                    .dynamicTypeSize(...DynamicTypeSize.xLarge)
                                    .padding(.top, 4).padding(.trailing, 6)
                            }
                        }
                }
                .buttonStyle(KeyStyle(notes: notesMode, done: left == 0))
                .disabled(!enabled)
                .keyboardShortcut(KeyEquivalent(Character("\(digit)")), modifiers: [])
                .accessibilityLabel(L10n.t("digit.label", ["digit": digit]))
                .accessibilityValue(left > 0 ? L10n.t("digit.left", ["count": left]) : "")
                .accessibilityIdentifier("number-\(digit)")
            }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(L10n.t("numbers"))
    }
}

/// What a board shows. Solo and live games fill it from their own state.
struct BoardContent {
    let size: Int
    let values: [Int]
    let givens: [Int]
    let notes: (Int) -> [Int]
    let conflicts: Set<Int>
    let selected: Int
    /// Duel cells the server has locked after a correct entry.
    var locked: (Int) -> Bool = { _ in false }
    /// Cells of an open hint's elimination pattern.
    var pattern: Set<Int> = []

    /// 6×6 boxes are 2 rows × 3 columns; 9×9 boxes are 3 × 3.
    var boxRows: Int { size == 6 ? 2 : 3 }
}

struct BoardView: View {
    let content: BoardContent
    let select: (Int) -> Void

    init(content: BoardContent, select: @escaping (Int) -> Void) {
        self.content = content
        self.select = select
    }

    init(session: SoloSession, pattern: Set<Int> = []) {
        let board = session.board
        self.init(content: BoardContent(size: board.size, values: board.cells.map(\.value), givens: board.givens,
                                        notes: board.notes, conflicts: session.conflicts, selected: session.selected,
                                        pattern: pattern)) {
            session.selected = $0
        }
    }

    var body: some View {
        let n = content.size
        Color.clear
            .aspectRatio(1, contentMode: .fit)
            .overlay {
                GeometryReader { geometry in
                    let cell = geometry.size.width / CGFloat(n)
                    VStack(spacing: 0) {
                        ForEach(0..<n, id: \.self) { row in
                            HStack(spacing: 0) {
                                ForEach(0..<n, id: \.self) { column in
                                    CellView(content: content, index: row * n + column, side: cell, select: select)
                                }
                            }
                        }
                    }
                    .overlay { GridLines(n: n, boxRows: content.boxRows).allowsHitTesting(false).accessibilityHidden(true) }
                }
            }
            .background(Theme.background)
            .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay { RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Theme.grid, lineWidth: 2) }
            .shadow(color: .black.opacity(0.06), radius: 8, y: 3)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("\(n)×\(n) Sudoku")
            .accessibilityIdentifier("game-board")
    }
}

/// Inner lines only (the outer edge is the board's rounded border): thin cell lines, 2 pt box lines.
struct GridLines: View {
    let n: Int
    let boxRows: Int

    var body: some View {
        Canvas { context, size in
            let step = size.width / CGFloat(n)
            for i in 1..<n {
                let offset = CGFloat(i) * step
                let thickRow = i % boxRows == 0, thickColumn = i % 3 == 0
                var horizontal = Path(), vertical = Path()
                horizontal.move(to: CGPoint(x: 0, y: offset)); horizontal.addLine(to: CGPoint(x: size.width, y: offset))
                vertical.move(to: CGPoint(x: offset, y: 0)); vertical.addLine(to: CGPoint(x: offset, y: size.height))
                context.stroke(horizontal, with: .color(thickRow ? Theme.grid : Theme.line), lineWidth: thickRow ? 2 : 1)
                context.stroke(vertical, with: .color(thickColumn ? Theme.grid : Theme.line), lineWidth: thickColumn ? 2 : 1)
            }
        }
    }
}

struct CellView: View {
    let content: BoardContent
    let index: Int
    let side: CGFloat
    let select: (Int) -> Void

    var body: some View {
        let n = content.size
        let row = index / n, column = index % n
        let value = content.values[index]
        let given = content.givens[index] != 0
        let conflict = content.conflicts.contains(index)
        let selected = content.selected == index
        let notes = content.notes(index)
        let style = appearance(selected: selected, row: row, column: column, value: value, conflict: conflict)

        Button { select(index) } label: {
            ZStack {
                Rectangle().fill(style.fill)
                if content.pattern.contains(index) && !selected {
                    Rectangle().strokeBorder(Theme.accent, lineWidth: 2)
                }
                if value != 0 {
                    Text("\(value)")
                        .font(.system(size: side * 0.56, weight: given ? .semibold : .medium, design: .rounded))
                        .foregroundStyle(style.ink(given: given))
                } else if !notes.isEmpty {
                    NotesGrid(notes: Set(notes), size: n, side: side, ink: selected ? Theme.onBrand : Theme.muted)
                }
                if conflict {
                    Text("!")
                        .font(.system(size: side * 0.24, weight: .heavy))
                        .foregroundStyle(selected ? Theme.onBrand : Theme.error)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
                        .padding(.top, 2).padding(.trailing, 4)
                }
            }
            .frame(width: side, height: side)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label(row: row, column: column, value: value, given: given, notes: notes, conflict: conflict))
        .accessibilityAddTraits(selected ? .isSelected : [])
        .accessibilityIdentifier("cell-\(row + 1)-\(column + 1)")
    }

    private struct Appearance {
        let fill: Color
        let selected: Bool
        let conflict: Bool
        func ink(given: Bool) -> Color {
            selected ? Theme.onBrand : conflict ? Theme.error : given ? Theme.foreground : Theme.accent
        }
    }

    /// Selected: brand fill. Then same digit, conflict, row/column/box peers, plain paper (as on the web).
    private func appearance(selected: Bool, row: Int, column: Int, value: Int, conflict: Bool) -> Appearance {
        let n = content.size, boxRows = content.boxRows, s = content.selected
        let selectedValue = content.values.indices.contains(s) ? content.values[s] : 0
        let fill: Color
        if selected { fill = Theme.brand }
        else if conflict { fill = Theme.errorBackground }
        else if content.pattern.contains(index) { fill = Theme.selected }
        else if selectedValue != 0 && value == selectedValue { fill = Theme.same }
        else if s / n == row || s % n == column || (s / n / boxRows == row / boxRows && s % n / 3 == column / 3) { fill = Theme.related }
        else { fill = Theme.background }
        return Appearance(fill: fill, selected: selected, conflict: conflict)
    }

    private func label(row: Int, column: Int, value: Int, given: Bool, notes: [Int], conflict: Bool) -> String {
        L10n.t("cell.label", [
            "row": row + 1, "column": column + 1,
            "kind": L10n.t(given ? "game.given" : content.locked(index) ? "game.locked" : "game.editable"),
            "value": value == 0 ? L10n.t("game.empty") : "\(value)",
            "notes": notes.isEmpty ? L10n.t("game.noNotes") : notes.map(String.init).joined(separator: " "),
            "conflict": conflict ? L10n.t("cell.conflict") : "",
        ])
    }
}

struct NotesGrid: View {
    let notes: Set<Int>
    let size: Int
    let side: CGFloat
    var ink: Color = Theme.muted

    var body: some View {
        let rows = size == 6 ? 2 : 3
        VStack(spacing: 0) {
            ForEach(0..<rows, id: \.self) { row in
                HStack(spacing: 0) {
                    ForEach(0..<3, id: \.self) { column in
                        let digit = row * 3 + column + 1
                        Text(notes.contains(digit) ? "\(digit)" : " ")
                            .font(.system(size: max(7, side * 0.26), weight: .medium))
                            .foregroundStyle(ink)
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                }
            }
        }
        .padding(side * 0.04)
    }
}

/// Pause and result share one centred layout: symbol tile, heading, line, a stat pill, then actions.
struct CenteredPanel<Actions: View>: View {
    let symbol: String
    var symbolTint: Color = Theme.accent
    var symbolFill: Color = Theme.selected
    var eyebrow: String?
    let title: String
    var titleId: String?
    var lede: String?
    @ViewBuilder let actions: () -> Actions

    var body: some View {
        ScrollView {
            VStack(spacing: 14) {
                Image(systemName: symbol).font(.system(size: 30, weight: .semibold))
                    .foregroundStyle(symbolTint)
                    .frame(width: 76, height: 76)
                    .background(symbolFill, in: RoundedRectangle(cornerRadius: 24, style: .continuous))
                    .padding(.bottom, 6)
                    .accessibilityHidden(true)
                if let eyebrow {
                    Text(eyebrow.uppercased(with: L10n.turkish ? Locale(identifier: "tr") : Locale(identifier: "en")))
                        .font(.caption.weight(.bold)).tracking(1.4).foregroundStyle(Theme.accent)
                }
                Text(title).font(Theme.display(.title, weight: .bold)).multilineTextAlignment(.center)
                    .accessibilityAddTraits(.isHeader)
                    .accessibilityIdentifier(titleId ?? "")
                if let lede {
                    Text(lede).foregroundStyle(Theme.muted).multilineTextAlignment(.center)
                        .fixedSize(horizontal: false, vertical: true)
                }
                actions()
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 24)
            .frame(maxWidth: 440)
            .frame(maxWidth: .infinity)
        }
    }
}

/// "9×9 · Easy" small over a large time, in a pill.
struct StatPill: View {
    let caption: String
    let value: String
    var label: String?
    var id: String?

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text(caption).font(.footnote.weight(.semibold)).foregroundStyle(Theme.muted)
            Text(value).font(Theme.display(.title2).monospacedDigit()).foregroundStyle(Theme.foreground)
        }
        .padding(.horizontal, 18).padding(.vertical, 10)
        .card(radius: 16)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(label ?? "\(caption) · \(value)")
        .accessibilityIdentifier(id ?? "")
    }
}

struct PausedView: View {
    let app: AppModel
    let session: SoloSession

    var body: some View {
        CenteredPanel(symbol: "pause.fill", title: L10n.t("paused.title"), lede: L10n.t("paused.detail")) {
            StatPill(caption: sizeAndLevel(session.size, session.level), value: L10n.time(session.elapsedMs),
                     label: "\(sizeAndLevel(session.size, session.level)) · \(L10n.t("game.elapsed", ["time": L10n.time(session.elapsedMs)]))",
                     id: "timer")
            if session.saveState == .failed || UIDebug.enabled { StorageStatus(session: session) }
            VStack(spacing: 10) {
                PrimaryButton(title: L10n.t("game.resume"), id: "resume") { session.resume(); app.cue(.select) }
                SecondaryButton(title: L10n.t("game.exit"), id: "save-exit") { app.leaveGame() }
                if session.saveState == .failed { DiscardButton(app: app) }
            }
            .padding(.top, 12)
            QuietSaveMarker(session: session)
        }
        .onAppear { AccessibilityNotification.Announcement(L10n.t("paused.title")).post() }
    }
}

struct DiscardButton: View {
    let app: AppModel
    var next: AppModel.Screen = .home
    @State private var confirming = false

    var body: some View {
        Button(L10n.t("game.discardExit"), role: .destructive) { confirming = true }
            .underline()
            .frame(maxWidth: .infinity, minHeight: 48)
            .accessibilityIdentifier("discard-exit")
            .alert(L10n.t("game.confirmDiscard"), isPresented: $confirming) {
                Button(L10n.t("game.discardExit"), role: .destructive) { app.leaveGame(discard: true, to: next) }
                    .accessibilityIdentifier("confirm-discard")
                Button(L10n.t("confirm.cancel"), role: .cancel) {}
            }
    }
}

struct ResultView: View {
    let app: AppModel
    let session: SoloSession

    var body: some View {
        CenteredPanel(symbol: "checkmark", eyebrow: L10n.t("home.completed"), title: L10n.t("result.title"),
                      titleId: "result-heading", lede: L10n.t("result.detail")) {
            // Screen readers hear "Sudoku complete" as the status, like the web's live region.
            Text(L10n.t("game.complete")).font(.footnote).opacity(0).frame(height: 1)
                .accessibilityIdentifier("game-status")
            StatPill(caption: sizeAndLevel(session.size, session.level), value: L10n.time(session.elapsedMs), id: "result-details")
            if session.saveState == .failed || UIDebug.enabled { StorageStatus(session: session) } else { QuietSaveMarker(session: session) }
            VStack(spacing: 10) {
                PrimaryButton(title: L10n.t("result.newGame"), id: "new-game") { app.leaveGame(to: .setup) }
                SecondaryButton(title: L10n.t("nav.home"), id: "home") { app.leaveGame() }
                if session.saveState == .failed { DiscardButton(app: app) }
            }
            .padding(.top, 12)
        }
        .onAppear { AccessibilityNotification.Announcement(L10n.t("game.complete")).post() }
    }
}

/// SwiftUI's `.delete` shortcut does not match the hardware Delete key on iOS,
/// so a first-responder view reads Delete/Backspace presses directly.
/// shortcut: not exercised by tests — simulator XCUITest never delivers Delete presses; check on a device with a keyboard.
struct DeleteKeyCatcher: UIViewRepresentable {
    let action: () -> Void

    final class CatcherView: UIView {
        var action: () -> Void = {}
        override var canBecomeFirstResponder: Bool { true }
        override func didMoveToWindow() {
            super.didMoveToWindow()
            if window != nil { becomeFirstResponder() }
        }
        override func pressesBegan(_ presses: Set<UIPress>, with event: UIPressesEvent?) {
            let codes = presses.compactMap { $0.key?.keyCode }
            if codes.contains(.keyboardDeleteOrBackspace) || codes.contains(.keyboardDeleteForward) {
                action()
            } else {
                super.pressesBegan(presses, with: event)
            }
        }
    }

    func makeUIView(context: Context) -> CatcherView { CatcherView() }
    func updateUIView(_ view: CatcherView, context: Context) { view.action = action }
}
