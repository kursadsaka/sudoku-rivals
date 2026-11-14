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

struct StorageStatus: View {
    let session: SoloSession

    var body: some View {
        if session.saveState == .saved {
            Label(L10n.t("saved"), systemImage: "checkmark.circle")
                .font(.footnote).foregroundStyle(Theme.muted)
                .accessibilityIdentifier("storage-status")
        } else {
            VStack(alignment: .leading, spacing: 8) {
                Label(L10n.t("error.save"), systemImage: "exclamationmark.triangle").foregroundStyle(Theme.error)
                Button(L10n.t("error.retry")) { session.save() }
                    .buttonStyle(.bordered)
                    .frame(minHeight: 48)
                    .accessibilityIdentifier("retry")
            }
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
    @Environment(\.horizontalSizeClass) private var widthClass
    @Environment(\.dynamicTypeSize) private var typeSize

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                status
                if widthClass == .regular {
                    HStack(alignment: .top, spacing: 24) {
                        BoardView(session: session).frame(maxWidth: 560)
                        VStack(alignment: .leading, spacing: 12) { controls }.frame(maxWidth: 420)
                    }
                } else {
                    BoardView(session: session)
                    controls
                }
            }
            .padding()
        }
        .toolbar {
            ToolbarItem(placement: .topBarLeading) {
                Button { app.leaveGame() } label: { Label(L10n.t("nav.home"), systemImage: "house") }
                    .accessibilityIdentifier("back")
            }
            ToolbarItem(placement: .topBarTrailing) {
                Button { session.pause() } label: { Label(L10n.t("game.pause"), systemImage: "pause.fill") }
                    .accessibilityIdentifier("pause")
            }
        }
        .background { ArrowKeys { session.move(rowDelta: $0, columnDelta: $1) } }
        .background { DeleteKeyCatcher { if session.canEdit { report(session.erase()) } }.accessibilityHidden(true) }
        .onChange(of: session.game) { hint = nil }
    }

    private var status: some View {
        ViewThatFits(in: .horizontal) {
            HStack { timer; Spacer(); gameStatus }
            VStack(alignment: .leading, spacing: 4) { timer; gameStatus }
        }
    }

    private var timer: some View {
        TimelineView(.periodic(from: .now, by: 1)) { _ in
            let text = L10n.t("game.elapsed", ["time": L10n.time(session.elapsedMs)])
            Text(text).font(.body.monospacedDigit())
                .accessibilityLabel(text)
                .accessibilityIdentifier("timer")
        }
    }

    private var gameStatus: some View {
        let conflict = !session.conflicts.isEmpty
        return Label(L10n.t(conflict ? "game.conflict" : "game.incomplete"),
                     systemImage: conflict ? "exclamationmark.triangle.fill" : "circle.dotted")
            .foregroundStyle(conflict ? Theme.error : Theme.muted)
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("game-status")
    }

    @ViewBuilder private var controls: some View {
        selectionLine
        NumberPad(size: session.size, notesMode: session.notesMode, enabled: session.canEdit) { report(session.enter($0)) }
        tools
        if let hint { hintCard(hint) }
        if let notice {
            Text(notice).foregroundStyle(Theme.foreground)
                .accessibilityIdentifier("game-notice")
        }
        StorageStatus(session: session)
        Text(L10n.t("keyboard", ["size": session.size])).font(.footnote).foregroundStyle(Theme.muted)
            .accessibilityHidden(true)
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
            Toggle(isOn: $session.notesMode) { ToolLabel(title: L10n.t("game.notes"), icon: "pencil") }
                .toggleStyle(.button)
                .keyboardShortcut("n", modifiers: [])
                .accessibilityIdentifier("notes-toggle")
            ToolButton(title: L10n.t("game.erase"), icon: "delete.left", id: "erase", enabled: session.canEdit) {
                report(session.erase())
            }
            ToolButton(title: L10n.t("game.undo"), icon: "arrow.uturn.backward", id: "undo", enabled: !session.board.undo.isEmpty) {
                report(session.undo())
            }
            .keyboardShortcut("z", modifiers: .command)
            ToolButton(title: L10n.t("game.redo"), icon: "arrow.uturn.forward", id: "redo", enabled: !session.board.redo.isEmpty) {
                report(session.redo())
            }
            .keyboardShortcut("z", modifiers: [.command, .shift])
            ToolButton(title: L10n.t("game.hint"), icon: "lightbulb", id: "hint", enabled: true) { showHint() }
        }
    }

    private func hintCard(_ hint: LogicalHint) -> some View {
        let n = session.size
        let row = hint.index / n + 1, column = hint.index % n + 1
        let unit = hint.unit.map { L10n.t("hint.\($0.kind)", ["number": $0.index + 1]) } ?? ""
        let text = L10n.t(hint.technique == "naked_single" ? "hint.naked" : "hint.hidden",
                          ["row": row, "column": column, "value": hint.value, "unit": unit])
        return GroupBox {
            VStack(alignment: .leading, spacing: 8) {
                Text(text).accessibilityIdentifier("hint-explanation")
                HStack {
                    Button(L10n.t("hint.apply")) { report(session.applyHint(hint)) }
                        .buttonStyle(.borderedProminent).foregroundStyle(Theme.onAccent)
                        .frame(minHeight: 48)
                        .accessibilityIdentifier("hint-apply")
                    Button(L10n.t("hint.dismiss")) { self.hint = nil }
                        .buttonStyle(.bordered).frame(minHeight: 48)
                        .accessibilityIdentifier("hint-dismiss")
                }
            }
        } label: {
            Label(L10n.t("game.hint"), systemImage: "lightbulb")
        }
    }

    private func showHint() {
        notice = nil
        switch session.hint() {
        case .hint(let found):
            hint = found
            session.selected = found.index
            announce(L10n.t(found.technique == "naked_single" ? "hint.naked" : "hint.hidden", [
                "row": found.index / session.size + 1, "column": found.index % session.size + 1, "value": found.value,
                "unit": found.unit.map { L10n.t("hint.\($0.kind)", ["number": $0.index + 1]) } ?? "",
            ]))
        case .noneWithSingles: say(L10n.t("hint.none"))
        case .conflicts: say(L10n.t("hint.conflicts"))
        case .unsolvable: say(L10n.t("hint.unsolvable"))
        case .searchLimit: say(L10n.t("hint.searchLimit"))
        }
    }

    private func report(_ reason: String?) {
        switch reason {
        case nil: notice = nil
        case "GivenCell": say(L10n.t("error.given"))
        case "FilledCell": say(L10n.t("error.filled"))
        case "NotPlaying": break
        default: say(L10n.t("error.generic"))
        }
    }

    private func say(_ text: String) { notice = text; announce(text) }
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
            Text(title).font(.caption).lineLimit(2).multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, minHeight: 48)
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
            .buttonStyle(.bordered)
            .disabled(!enabled)
            .accessibilityIdentifier(id)
    }
}

struct NumberPad: View {
    let size: Int
    let notesMode: Bool
    let enabled: Bool
    let enter: (Int) -> Void

    var body: some View {
        let n = size
        let columns = Array(repeating: GridItem(.flexible(), spacing: 8), count: n == 6 ? 6 : 5)
        LazyVGrid(columns: columns, spacing: 8) {
            ForEach(1...n, id: \.self) { digit in
                Button { enter(digit) } label: {
                    Text("\(digit)")
                        .font(.title.weight(notesMode ? .regular : .semibold))
                        .frame(maxWidth: .infinity, minHeight: 56)
                }
                .buttonStyle(.bordered)
                .disabled(!enabled)
                .keyboardShortcut(KeyEquivalent(Character("\(digit)")), modifiers: [])
                .accessibilityLabel(L10n.t("digit.label", ["digit": digit]))
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

    init(session: SoloSession) {
        let board = session.board
        self.init(content: BoardContent(size: board.size, values: board.cells.map(\.value), givens: board.givens,
                                        notes: board.notes, conflicts: session.conflicts, selected: session.selected)) {
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
            .accessibilityElement(children: .contain)
            .accessibilityLabel("\(n)×\(n) Sudoku")
            .accessibilityIdentifier("game-board")
    }
}

struct GridLines: View {
    let n: Int
    let boxRows: Int

    var body: some View {
        Canvas { context, size in
            let step = size.width / CGFloat(n)
            for i in 0...n {
                let offset = CGFloat(i) * step
                let thickRow = i % boxRows == 0, thickColumn = i % 3 == 0
                var horizontal = Path(), vertical = Path()
                horizontal.move(to: CGPoint(x: 0, y: offset)); horizontal.addLine(to: CGPoint(x: size.width, y: offset))
                vertical.move(to: CGPoint(x: offset, y: 0)); vertical.addLine(to: CGPoint(x: offset, y: size.height))
                context.stroke(horizontal, with: .color(thickRow ? Theme.foreground : Theme.border.opacity(0.6)), lineWidth: thickRow ? 2 : 0.5)
                context.stroke(vertical, with: .color(thickColumn ? Theme.foreground : Theme.border.opacity(0.6)), lineWidth: thickColumn ? 2 : 0.5)
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

        Button { select(index) } label: {
            ZStack {
                Rectangle().fill(background(selected: selected, row: row, column: column))
                if value != 0 {
                    Text("\(value)")
                        .font(.system(size: side * 0.58, weight: given ? .bold : .regular, design: .rounded))
                        .foregroundStyle(conflict ? Theme.error : given ? Theme.foreground : Theme.accent)
                } else if !notes.isEmpty {
                    NotesGrid(notes: Set(notes), size: n, side: side)
                }
                if conflict {
                    Text("!")
                        .font(.system(size: side * 0.28, weight: .heavy))
                        .foregroundStyle(Theme.error)
                        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
                        .padding(2)
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

    private func background(selected: Bool, row: Int, column: Int) -> Color {
        if selected { return Theme.selected }
        let n = content.size, boxRows = content.boxRows, s = content.selected
        let peer = s / n == row || s % n == column || (s / n / boxRows == row / boxRows && s % n / 3 == column / 3)
        return peer ? Theme.selected.opacity(0.35) : Color(uiColor: .systemBackground)
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

    var body: some View {
        let rows = size == 6 ? 2 : 3
        VStack(spacing: 0) {
            ForEach(0..<rows, id: \.self) { row in
                HStack(spacing: 0) {
                    ForEach(0..<3, id: \.self) { column in
                        let digit = row * 3 + column + 1
                        Text(notes.contains(digit) ? "\(digit)" : " ")
                            .font(.system(size: side * 0.24, weight: .medium))
                            .foregroundStyle(Theme.muted)
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                }
            }
        }
        .padding(side * 0.04)
    }
}

struct PausedView: View {
    let app: AppModel
    let session: SoloSession

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Image(systemName: "pause.circle").font(.system(size: 48)).foregroundStyle(Theme.accent)
                    .accessibilityHidden(true)
                Text(L10n.t("paused.title")).font(.title.bold()).accessibilityAddTraits(.isHeader)
                Text(L10n.t("paused.detail")).foregroundStyle(Theme.muted)
                Text("\(session.size)×\(session.size) · \(L10n.t("game.elapsed", ["time": L10n.time(session.elapsedMs)]))")
                    .font(.body.monospacedDigit())
                    .accessibilityIdentifier("timer")
                StorageStatus(session: session)
                PrimaryButton(title: L10n.t("game.resume"), id: "resume") { session.resume() }
                SecondaryButton(title: L10n.t("game.exit"), id: "save-exit") { app.leaveGame() }
                if session.saveState == .failed {
                    DiscardButton(app: app)
                }
            }
            .padding()
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
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Image(systemName: "checkmark.seal.fill").font(.system(size: 48)).foregroundStyle(Theme.accent)
                    .accessibilityHidden(true)
                Text(L10n.t("game.complete")).font(.subheadline.bold()).foregroundStyle(Theme.muted)
                    .accessibilityIdentifier("game-status")
                Text(L10n.t("result.title")).font(.title.bold()).accessibilityAddTraits(.isHeader)
                    .accessibilityIdentifier("result-heading")
                Text(L10n.t("result.detail")).foregroundStyle(Theme.muted)
                Text("\(session.size)×\(session.size) · \(L10n.time(session.elapsedMs))")
                    .font(.body.monospacedDigit())
                    .accessibilityIdentifier("result-details")
                StorageStatus(session: session)
                PrimaryButton(title: L10n.t("result.newGame"), id: "new-game") { app.leaveGame(to: .setup) }
                SecondaryButton(title: L10n.t("nav.home"), id: "home") { app.leaveGame() }
                if session.saveState == .failed { DiscardButton(app: app) }
            }
            .padding()
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
