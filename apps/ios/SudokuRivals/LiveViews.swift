import SwiftUI
import UIKit

/// Invite-based 1v1 live matches: setup → lobby/countdown → race or duel → result.
/// Every rule and result comes from the server through the shared Rust client.
struct LiveScreen: View {
    @Bindable var live: LiveSession
    let prefs: AppPreferences

    /// The snapshot while a match is being played (not lobby, result, lost session or protected record).
    private var activeGame: LiveView.Snapshot? {
        let view = live.view
        guard !live.protectedSave, let snapshot = view.snapshot, !view.voluntarilyLeft,
              !(snapshot.phase == "finished" && snapshot.result != nil),
              view.connection != "session_lost", snapshot.phase != "closed",
              snapshot.phase != "lobby", snapshot.phase != "countdown" else { return nil }
        return snapshot
    }

    /// What the feedback cues listen to: match start, a friend arriving, a wrong duel move, the result.
    private struct CueKey: Equatable {
        var phase: String?
        var opponent = false
        var receipt = ""
    }
    private var cueKey: CueKey? {
        let view = live.view
        guard let s = view.snapshot, !view.voluntarilyLeft else { return nil }
        let receipt = view.lastReceipt.map { "\($0.matchId):\($0.code):\(s.own.wrong)" } ?? ""
        return CueKey(phase: s.phase, opponent: s.opponent != nil, receipt: receipt)
    }

    /// A finished match with its server result (shown full screen, like the solo result).
    private var finished: LiveView.Snapshot? {
        let view = live.view
        guard !live.protectedSave, let snapshot = view.snapshot, !view.voluntarilyLeft,
              snapshot.phase == "finished", snapshot.result != nil else { return nil }
        return snapshot
    }

    var body: some View {
        let game = activeGame
        Group {
            if let game {
                LiveGameView(live: live, snapshot: game)
            } else if let finished {
                LiveResultView(live: live, snapshot: finished)
            } else {
                ScrollView {
                    VStack(alignment: .leading, spacing: 14) {
                        LiveNotices(live: live)
                        content
                    }
                    .padding(.horizontal, 20)
                    .padding(.vertical, 8)
                    .frame(maxWidth: 560)
                    .frame(maxWidth: .infinity)
                }
            }
        }
        .toolbar(game != nil && !UIDebug.enabled ? .hidden : .visible, for: .navigationBar)
        .toolbar { BackToolbar(title: L10n.t("nav.back")) { live.goHome() } }
        .onAppear { live.open() }
        .task {
            while !Task.isCancelled {
                live.tick()
                try? await Task.sleep(for: .milliseconds(250))
            }
        }
        .onChange(of: live.message) { _, message in if let message { announce(message) } }
        .onChange(of: cueKey) { before, now in cues(before, now) }
        .alert(L10n.t("leave.title"), isPresented: $live.leaveConfirmation) {
            Button(L10n.t("leave.confirm"), role: .destructive) { live.confirmLeave() }
                .accessibilityIdentifier("leave-confirm")
            Button(L10n.t("confirm.cancel"), role: .cancel) {}
        } message: {
            Text(L10n.t(live.snapshot?.phase == "playing" ? "leave.playing" : "leave.lobby"))
        }
        .alert(L10n.t("reset.title"), isPresented: $live.resetConfirmation) {
            Button(L10n.t("reset.confirm"), role: .destructive) { live.confirmReset() }
                .accessibilityIdentifier("live-reset-confirm")
            Button(L10n.t("confirm.cancel"), role: .cancel) {}
        } message: {
            Text(L10n.t("reset.detail"))
        }
    }

    /// The first look at a session (app start, room just created) stays silent.
    private func cues(_ before: CueKey?, _ now: CueKey?) {
        guard let before, let now, let s = live.snapshot else { return }
        let feedback = Feedback.shared
        if now.phase == "playing" && before.phase != "playing" { feedback.play(.attention) }
        else if now.phase == "finished" && before.phase != "finished" {
            feedback.play(s.result?.winner == s.own.seat ? .success : .select)
        } else if now.phase == "lobby" && now.opponent && !before.opponent { feedback.play(.ready) }
        if now.receipt != before.receipt && live.view.lastReceipt?.code == "wrong" { feedback.play(.error) }
    }

    @ViewBuilder private var content: some View {
        let view = live.view
        if live.protectedSave {
            VStack(alignment: .leading, spacing: 14) {
                PageHeader(eyebrow: L10n.t("live.title"), title: L10n.t("protected.title"), lede: L10n.t("protected.info"))
                PrimaryButton(title: L10n.t("protected.reset"), id: "live-recover") { live.requestReset() }
            }
        } else if let snapshot = view.snapshot, !view.voluntarilyLeft {
            // A finished match with a result is drawn by `body` (its panel scrolls by itself).
            if view.connection == "session_lost" || snapshot.phase == "closed" {
                VStack(alignment: .leading, spacing: 14) {
                    ConnectionStatus(connection: view.connection)
                    Text(L10n.t("lost.title")).font(Theme.display(.title)).accessibilityAddTraits(.isHeader)
                        .accessibilityIdentifier("result-heading")
                    Text(L10n.t(snapshot.mode == "race" ? "lost.detailRace" : "lost.detail")).foregroundStyle(Theme.muted)
                        .accessibilityIdentifier("result-details")
                    PrimaryButton(title: L10n.t("lost.reset"), id: "live-recover") { live.requestReset() }
                    SecondaryButton(title: L10n.t("nav.home"), id: "home") { live.goHome() }
                }
            } else if snapshot.phase == "lobby" || snapshot.phase == "countdown" {
                LiveLobbyView(live: live, snapshot: snapshot)
            } else {
                LiveGameView(live: live, snapshot: snapshot)
            }
        } else {
            LiveSetupView(live: live, prefs: prefs)
        }
    }
}

/// Storage-failure and error messages; transient, so they sit with the other notices.
struct LiveNotices: View {
    let live: LiveSession
    /// False on a game screen, where `LiveToast` shows the message over the title instead.
    var showsMessage = true

    var body: some View {
        if live.storageError {
            VStack(alignment: .leading, spacing: 6) {
                Label(L10n.t("live.storageError"), systemImage: "exclamationmark.triangle")
                Button(L10n.t("live.storageRetry")) { live.retryStorage() }
                    .font(.subheadline.weight(.semibold)).underline()
                    .frame(minHeight: 44)
                    .accessibilityIdentifier("live-save-retry")
            }
            .font(.subheadline)
            .foregroundStyle(Theme.error)
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .card(radius: 14, fill: Theme.errorBackground, stroke: Theme.error)
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("live-storage-status")
        }
        if showsMessage, let message = live.message {
            Label(message, systemImage: "exclamationmark.circle")
                .font(.subheadline)
                .foregroundStyle(Theme.error)
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .card(radius: 14, fill: Theme.errorBackground, stroke: Theme.error)
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("live-message")
        }
    }
}

private func announce(_ text: String) { AccessibilityNotification.Announcement(text).post() }

/// A passing match message (server unreachable, an action that failed) floats over the title and timer, between
/// Leave and the right edge, so the board, keys and tools never move. Each distinct message shows once for six
/// seconds or until closed: a retry loop failing the same way does not flash it again, and it returns only after
/// the message cleared and came back. `LiveScreen` already announces every new message to VoiceOver.
struct LiveToast: View {
    let message: String?
    @State private var shown: String?
    @State private var last: String?
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        ZStack {
            if let shown {
                HStack(spacing: 2) {
                    Label(shown, systemImage: "exclamationmark.circle").font(.footnote)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .fixedSize(horizontal: false, vertical: true)
                        .accessibilityIdentifier("live-message")
                    Button { withAnimation(.easeOut(duration: 0.15)) { self.shown = nil } } label: {
                        Image(systemName: "xmark").font(.footnote.weight(.semibold)).frame(width: 44, height: 44)
                    }
                    .accessibilityLabel(L10n.t("hint.dismiss"))
                    .accessibilityIdentifier("live-message-dismiss")
                }
                .foregroundStyle(Theme.error)
                .padding(.leading, 12)
                .frame(minHeight: 48)
                .background(Theme.errorBackground, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                .overlay { RoundedRectangle(cornerRadius: 14, style: .continuous).strokeBorder(Theme.error, lineWidth: 1) }
                .padding(.horizontal, 56)
                .accessibilityElement(children: .contain)
                .transition(reduceMotion ? .opacity : .move(edge: .top).combined(with: .opacity))
            }
        }
        .onAppear { update(message) }
        .onChange(of: message) { _, now in update(now) }
        .task(id: shown) {
            guard shown != nil else { return }
            try? await Task.sleep(for: .seconds(6))
            if !Task.isCancelled { withAnimation(.easeOut(duration: 0.2)) { shown = nil } }
        }
    }

    private func update(_ now: String?) {
        guard now != last else { return }
        last = now
        withAnimation(.easeOut(duration: 0.2)) { shown = now }
    }
}

/// Waiting for the server: a small turning arc in place of a sentence, in the same footprint, so nothing moves.
/// Under Reduce Motion it does not turn; it only breathes in opacity. VoiceOver reads its name.
struct PendingIndicator: View {
    var size: CGFloat = 18
    var id = "pending-indicator"
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation) { context in
            let t = context.date.timeIntervalSinceReferenceDate
            ZStack {
                Circle().stroke(lineWidth: size / 7.5).opacity(0.25)
                Circle().trim(from: 0, to: 0.25)
                    .stroke(style: StrokeStyle(lineWidth: size / 7.5, lineCap: .round))
                    .rotationEffect(.degrees(reduceMotion ? 0 : t.truncatingRemainder(dividingBy: 0.9) / 0.9 * 360))
            }
            .opacity(reduceMotion ? 0.675 + 0.325 * cos(t * .pi / 1.2) : 1)
            .frame(width: size, height: size)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(L10n.t("status.pending"))
        .accessibilityAddTraits(.updatesFrequently)
        .accessibilityIdentifier(id)
    }
}

private func modeName(_ mode: String) -> String { L10n.t(mode == "race" ? "live.race" : "live.duel") }

/// "Classic race · 9×9 · Easy"; rooms created without a level show none.
private func roomLine(_ snapshot: LiveView.Snapshot, short: Bool = false) -> String {
    let mode = short ? L10n.t(snapshot.mode == "race" ? "live.raceShort" : "live.duelShort") : modeName(snapshot.mode)
    let level = snapshot.difficulty.flatMap { AppPreferences.levels.contains($0) ? $0 : nil }
    return "\(mode) · \(sizeAndLevel(snapshot.size, level))"
}

private func clock(_ ms: UInt64) -> String { String(format: "%d:%02d", ms / 60_000, ms / 1000 % 60) }

struct ConnectionStatus: View {
    let connection: String
    /// Game screen: a healthy connection is only a dot (the label still says "Connected"); a problem shows text.
    var compact = false

    var body: some View {
        let key = ["connected", "reconnecting", "reconcile", "session_lost"].contains(connection) ? connection : "disconnected"
        let text = L10n.t("conn.\(key)")
        let healthy = connection == "connected"
        if compact && healthy {
            ConnectionDot(state: key)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(text)
                .accessibilityIdentifier("connection-status")
                .onChange(of: connection) { announce(text) }
        } else {
            HStack(spacing: 6) {
                ConnectionDot(state: key).accessibilityHidden(true)
                Text(text).foregroundStyle(healthy ? Theme.muted : ConnectionDot.tint(key))
            }
            .font(.footnote)
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("connection-status")
            .onChange(of: connection) { announce(text) }
        }
    }
}

/// The connection dot, the same on every screen. Shape and colour both carry the state, so it never relies on
/// colour alone: filled green (connected), half-filled amber (reconnecting), hollow red (disconnected or boards
/// differ), struck-through red (session lost). The text label next to it, or its VoiceOver label, names it.
struct ConnectionDot: View {
    let state: String

    static func tint(_ state: String) -> Color {
        switch state {
        case "connected": Theme.ok
        case "reconnecting": Theme.warn
        default: Theme.error
        }
    }

    var body: some View {
        let symbol = switch state {
        case "connected": "circle.fill"
        case "reconnecting": "circle.lefthalf.filled"
        case "session_lost": "xmark.circle"
        default: "circle"
        }
        Image(systemName: symbol)
            .font(.system(size: state == "session_lost" ? 10 : 9, weight: .bold))
            .foregroundStyle(Self.tint(state))
            .frame(width: 10, height: 10)
    }
}

struct LiveSetupView: View {
    @Bindable var live: LiveSession
    let prefs: AppPreferences
    @State private var tab = "create"

    var body: some View {
        let busy = live.view.pendingHttp
        let joining = tab == "join"
        VStack(alignment: .leading, spacing: 16) {
            PageHeader(eyebrow: L10n.t("live.title"), title: L10n.t(joining ? "live.joinTitle" : "live.setupTitle"),
                       lede: L10n.t(joining ? "live.joinIntro" : "live.setupIntro"))
            Picker(L10n.t("live.tabs"), selection: $tab) {
                Text(L10n.t("live.create")).tag("create")
                Text(L10n.t("live.join")).tag("join")
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("live-tabs")

            // The pinned match server is configuration, not something players need to read.
            if !live.isConfigured { Text(L10n.t("live.notConfigured")).foregroundStyle(Theme.error) }

            if joining {
                field(L10n.t("live.invite")) {
                    TextField("ABC234…", text: $live.invite)
                        .font(Theme.display(.title2, weight: .bold))
                        .multilineTextAlignment(.center)
                        .textInputAutocapitalization(.never) // links are case-sensitive; the core uppercases codes
                        .keyboardType(.asciiCapable)
                        .autocorrectionDisabled()
                        .submitLabel(.go)
                        .onSubmit { submitJoin(busy: busy) }
                        .inputBox(minHeight: 60)
                        .accessibilityLabel(L10n.t("live.invite"))
                        .accessibilityIdentifier("room-code")
                }
                nameField
                PrimaryButton(title: L10n.t("live.join"), id: "room-join", pending: busy) { submitJoin(busy: busy) }
                    .disabled(busy || !live.isConfigured || live.invite.trimmingCharacters(in: .whitespaces).isEmpty)
            } else {
                nameField
                VStack(alignment: .leading, spacing: 8) {
                    SectionLabel(text: L10n.t("live.mode"))
                    HStack(spacing: 12) {
                        ForEach(["race", "duel"], id: \.self) { mode in
                            ChoiceCard(title: modeName(mode), detail: L10n.t(mode == "race" ? "live.raceDetail" : "live.duelDetail"),
                                       selected: live.mode == mode, id: "mode-\(mode)",
                                       titleFont: Theme.display(.headline, weight: .bold)) {
                                EmptyView()
                            } action: {
                                guard live.mode != mode else { return }
                                live.mode = mode
                                prefs.liveMode = mode
                                Feedback.shared.play(.select)
                            }
                        }
                    }
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("game-mode")
                }
                VStack(alignment: .leading, spacing: 8) {
                    SectionLabel(text: L10n.t("live.size"))
                    Picker(L10n.t("live.size"), selection: $live.size) {
                        Text("6×6").tag(6)
                        Text("9×9").tag(9)
                    }
                    .pickerStyle(.segmented)
                    .accessibilityIdentifier("board-size")
                }
                VStack(alignment: .leading, spacing: 8) {
                    SectionLabel(text: L10n.t("setup.difficulty"))
                    LevelPicker(selection: Binding(get: { live.difficulty ?? "easy" }, set: { live.difficulty = $0 })) {
                        prefs.liveDifficulty = live.difficulty ?? "easy"
                        Feedback.shared.play(.select)
                    }
                    Text("\(L10n.t("level.\(live.difficulty ?? "easy")Help")) \(L10n.t("level.scope"))")
                        .font(.footnote).foregroundStyle(Theme.muted)
                        .fixedSize(horizontal: false, vertical: true)
                        .accessibilityIdentifier("difficulty-help")
                }
                PrimaryButton(title: L10n.t("live.create"), id: "room-create", pending: busy) {
                    remember()
                    live.create()
                }
                .disabled(busy || !live.isConfigured)
            }
        }
        .disabled(busy)
        .onChange(of: live.size) { _, size in prefs.liveSize = size }
    }

    private var nameField: some View {
        field(L10n.t("live.name")) {
            TextField(L10n.t("live.name"), text: $live.name)
                .textContentType(.nickname)
                .autocorrectionDisabled()
                .submitLabel(.done)
                .inputBox(minHeight: 52)
                .onChange(of: live.name) { _, name in if name.count > 16 { live.name = String(name.prefix(16)) } }
                .accessibilityIdentifier("guest-name")
        }
    }

    private func field<Content: View>(_ label: String, @ViewBuilder _ content: () -> Content) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(label).font(.subheadline.weight(.medium)).foregroundStyle(Theme.muted)
            content()
        }
    }

    private func remember() {
        let name = live.name.trimmingCharacters(in: .whitespaces)
        if !name.isEmpty { prefs.displayName = name }
    }

    private func submitJoin(busy: Bool) {
        guard !busy, live.isConfigured, !live.invite.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        remember()
        live.join()
    }
}

extension View {
    /// A text field drawn like the web's inputs: paper, a 1.5 pt edge, rounded corners.
    func inputBox(minHeight: CGFloat) -> some View {
        padding(.horizontal, 14)
            .frame(minHeight: minHeight)
            .background(Theme.background, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
            .overlay { RoundedRectangle(cornerRadius: 14, style: .continuous).strokeBorder(Theme.border, lineWidth: 1.5) }
    }
}

struct LiveLobbyView: View {
    let live: LiveSession
    let snapshot: LiveView.Snapshot
    @State private var copied = false

    var body: some View {
        let countdown = snapshot.phase == "countdown"
        let own = snapshot.own, view = live.view
        VStack(alignment: .leading, spacing: 14) {
            PageHeader(eyebrow: roomLine(snapshot), title: L10n.t(countdown ? "lobby.countdown" : "lobby.waiting"))
            ConnectionStatus(connection: view.connection)
            if let level = snapshot.difficulty, AppPreferences.levels.contains(level) {
                Text("\(Text(L10n.t("lobby.level", ["level": L10n.t("level.\(level)")])).bold().foregroundStyle(Theme.accent)) \(L10n.t("level.\(level)Help")) \(L10n.t("level.scope"))")
                    .font(.footnote)
                    .padding(12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Theme.selected, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                    .accessibilityIdentifier("room-difficulty")
            }

            VStack(spacing: 14) {
                SectionLabel(text: L10n.t("lobby.room"))
                HStack(spacing: 6) {
                    ForEach(Array(snapshot.roomCode.enumerated()), id: \.offset) { _, letter in
                        Text(String(letter))
                            .font(Theme.display(.title2, weight: .bold))
                            .frame(width: 38, height: 46)
                            .background(Theme.canvas, in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                            .overlay { RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Theme.line, lineWidth: 1) }
                    }
                }
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(L10n.t("lobby.code", ["code": snapshot.roomCode.map(String.init).joined(separator: " ")]))
                .accessibilityValue(snapshot.roomCode)
                .accessibilityIdentifier("room-code")
                HStack(spacing: 10) {
                    Button {
                        UIPasteboard.general.string = snapshot.roomCode
                        copied = true
                        Feedback.shared.play(.select)
                        announce(L10n.t("lobby.copied"))
                    } label: {
                        Label(L10n.t(copied ? "lobby.copied" : "lobby.copy"), systemImage: "doc.on.doc")
                            .font(.subheadline.weight(.semibold)).frame(maxWidth: .infinity, minHeight: 48)
                    }
                    .buttonStyle(PressStyle(fill: Theme.background, foreground: Theme.accent, stroke: Theme.line, radius: 14))
                    .accessibilityIdentifier("invite-copy")
                    if let url = live.inviteURL {
                        ShareLink(item: url) {
                            Label(L10n.t("lobby.share"), systemImage: "square.and.arrow.up")
                                .font(.subheadline.weight(.semibold)).frame(maxWidth: .infinity, minHeight: 48)
                        }
                        .buttonStyle(PressStyle(fill: Theme.background, foreground: Theme.accent, stroke: Theme.line, radius: 14))
                        .accessibilityIdentifier("invite-share")
                    }
                }
            }
            .padding(18)
            .frame(maxWidth: .infinity)
            .card()

            VStack(spacing: 10) {
                seat(name: own.name, you: true, ready: own.ready, connected: own.connected)
                if let opponent = snapshot.opponent {
                    seat(name: opponent.name, you: false, ready: opponent.ready, connected: opponent.connected)
                } else {
                    seat(name: nil, you: false, ready: false, connected: true)
                }
            }
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("players")

            if countdown {
                Text(countdownText(view)).font(Theme.display(.title3).monospacedDigit()).foregroundStyle(Theme.accent)
                    .frame(maxWidth: .infinity)
                    .accessibilityIdentifier("timer")
            }
            PrimaryButton(title: L10n.t(own.ready ? "lobby.unready" : "lobby.setReady"), id: "ready-toggle") {
                Feedback.shared.play(.select)
                live.command("ready", ready: !own.ready)
            }
            .disabled(snapshot.opponent == nil || view.connection != "connected" || view.pendingCommand)
        }
    }

    /// A player row: initial in a round avatar (you blue, rival amber, empty dashed) and a ready pill.
    private func seat(name: String?, you: Bool, ready: Bool, connected: Bool) -> some View {
        HStack(spacing: 12) {
            Text(name.flatMap { $0.trimmingCharacters(in: .whitespaces).first.map { String($0).uppercased() } } ?? "+")
                .font(Theme.display(.body, weight: .bold))
                .foregroundStyle(name == nil ? Theme.muted : you ? Theme.accent : Theme.rival)
                .frame(width: 44, height: 44)
                .background(name == nil ? Color.clear : you ? Theme.selected : Theme.rivalSoft, in: Circle())
                .overlay { if name == nil { Circle().strokeBorder(Theme.border, style: StrokeStyle(lineWidth: 1.5, dash: [4, 3])) } }
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 0) {
                Text(name ?? L10n.t("lobby.noOpponent")).font(.body.weight(name == nil ? .regular : .semibold))
                    .foregroundStyle(name == nil ? Theme.muted : Theme.foreground)
                if you { Text(L10n.t("lobby.youShort")).font(.footnote).foregroundStyle(Theme.muted) }
                else if name != nil && !connected { Text(L10n.t("lobby.opponentOffline").trimmingCharacters(in: CharacterSet(charactersIn: " ·"))).font(.footnote).foregroundStyle(Theme.error) }
            }
            Spacer(minLength: 8)
            if name != nil {
                Text(L10n.t(ready ? "lobby.ready" : "lobby.notReady"))
                    .font(.footnote.weight(.semibold))
                    .foregroundStyle(ready ? Color.white : Theme.muted)
                    .padding(.horizontal, 10).padding(.vertical, 5)
                    .background(ready ? Color(red: 0x15 / 255, green: 0x80 / 255, blue: 0x3d / 255) : Theme.track, in: Capsule())
            }
        }
        .padding(10).padding(.trailing, 4)
        .frame(maxWidth: .infinity, minHeight: 64)
        .card(radius: 18)
    }

    private func countdownText(_ view: LiveView) -> String {
        guard let start = snapshot.startMs, let now = view.estimatedServerMs else { return L10n.t("lobby.syncing") }
        let remaining = start > now ? start - now : 0
        return remaining == 0 ? L10n.t("lobby.startWait") : L10n.t("lobby.startsIn", ["seconds": (remaining + 999) / 1000])
    }
}

struct LiveGameView: View {
    @Bindable var live: LiveSession
    let snapshot: LiveView.Snapshot
    @Environment(\.dynamicTypeSize) private var typeSize

    var body: some View {
        let view = live.view
        let race = snapshot.mode == "race"
        let content = boardContent(race: race)
        GameScreenLayout { density in
            VStack(spacing: density == .compact ? 4 : 6) {
                navBar(view: view, race: race)
                hud(race: race, content: content, compact: density == .compact)
                statusRow(race: race, content: content)
            }
            .overlay(alignment: .top) { LiveToast(message: live.message) }
        } board: {
            if let content {
                BoardView(content: content) { live.selected = $0 }
            } else {
                ProgressView(L10n.t("game.syncing")).frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        } bottom: {
            VStack(spacing: 8) { bottom(view: view, race: race, content: content) }
        }
        .background { ArrowKeys { live.moveSelection(rows: $0, columns: $1) } }
        .background {
            DeleteKeyCatcher { if race, live.canInput() { live.edit("erase") } }.accessibilityHidden(true)
        }
    }

    private func navBar(view: LiveView, race: Bool) -> some View {
        GameNavBar(mode: roomLine(snapshot, short: true), back: { live.goHome() }) {
            Text(timerText(view, race: race)).font(Theme.display(.title2).monospacedDigit())
                .foregroundStyle(!race && (view.remainingMs ?? 90_000) <= 10_000 ? Theme.error : Theme.foreground)
                .accessibilityLabel(L10n.t(race ? "live.elapsed" : "live.remaining") + " " + timerText(view, race: race))
                .accessibilityIdentifier("timer")
        } trailing: {
            EmptyView()
        }
    }

    /// You on the left in blue, the rival mirrored on the right in amber: filled share in a race, score in a duel.
    @ViewBuilder private func hud(race: Bool, content: BoardContent?, compact: Bool) -> some View {
        let rivalName = snapshot.opponent?.name ?? L10n.t("hud.opponent")
        if race {
            let own = content.map(Self.progress) ?? 0
            let rival = Double(min(100, snapshot.opponent?.progressPercent ?? 0)) / 100
            VStack(spacing: 4) {
                HStack(alignment: .firstTextBaseline) {
                    Text(L10n.t("hud.you")).font(.footnote.weight(.semibold)).foregroundStyle(Theme.accent)
                    Text("\(Int(own * 100))%").font(Theme.display(.subheadline).monospacedDigit())
                    Spacer(minLength: 8)
                    Text("\(snapshot.opponent?.progressPercent ?? 0)%").font(Theme.display(.subheadline).monospacedDigit())
                        .accessibilityLabel(L10n.t("game.opponentProgress", ["percent": snapshot.opponent?.progressPercent ?? 0]))
                        .accessibilityIdentifier("opponent-progress")
                    Text(rivalName).font(.footnote.weight(.semibold)).foregroundStyle(Theme.rival).lineLimit(1)
                        .accessibilityIdentifier("opponent-name")
                }
                if !compact {
                    HStack(spacing: 10) {
                        Meter(value: own)
                        Meter(value: rival, tint: Theme.rival, leading: false)
                    }
                }
            }
        } else {
            let remaining = Double(live.view.remainingMs ?? 90_000) / 90_000
            VStack(spacing: 4) {
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Text(L10n.t("hud.you")).font(.footnote.weight(.semibold)).foregroundStyle(Theme.accent).accessibilityHidden(true)
                    Text("\(snapshot.own.score)").font(Theme.display(.title3).monospacedDigit())
                        .accessibilityLabel(L10n.t("game.you", ["score": snapshot.own.score]))
                        .accessibilityIdentifier("own-score")
                    Text("×\(snapshot.own.combo)").font(.caption.weight(.semibold).monospacedDigit())
                        .foregroundStyle(Theme.accent)
                        .padding(.horizontal, 7).padding(.vertical, 1)
                        .background(Theme.selected, in: Capsule())
                        .opacity(snapshot.own.combo > 1 ? 1 : 0)
                        .accessibilityLabel(L10n.t("game.combo", ["combo": snapshot.own.combo]))
                        .accessibilityIdentifier("combo")
                    Spacer(minLength: 8)
                    Text("\(snapshot.opponent?.score ?? 0)").font(Theme.display(.title3).monospacedDigit())
                        .accessibilityLabel(L10n.t("game.opponentScore", ["score": snapshot.opponent?.score ?? 0]))
                        .accessibilityIdentifier("opponent-score")
                    Text(rivalName).font(.footnote.weight(.semibold)).foregroundStyle(Theme.rival).lineLimit(1)
                        .accessibilityIdentifier("opponent-name")
                }
                Meter(value: remaining, tint: remaining <= 10.0 / 90 ? Theme.error : Theme.accent)
            }
        }
    }

    static func progress(_ content: BoardContent) -> Double {
        let blanks = content.givens.indices.filter { content.givens[$0] == 0 }
        guard !blanks.isEmpty else { return 0 }
        return Double(blanks.filter { content.values[$0] != 0 }.count) / Double(blanks.count)
    }

    /// One status row: connection (a dot while healthy) and the message of the moment.
    @ViewBuilder private func statusRow(race: Bool, content: BoardContent?) -> some View {
        let view = live.view
        let status = content.map { statusText(race: race, conflicts: !$0.conflicts.isEmpty) }
        let neutral = status == L10n.t("game.incomplete")
        let locked = !race && (view.lockedRemainingMs ?? 0) > 0
        let alert = locked || !(content?.conflicts.isEmpty ?? true)
        GameStatusRow {
            HStack(spacing: 8) {
                ConnectionStatus(connection: view.connection, compact: true)
                if let status, !neutral {
                    if status == L10n.t("status.pending") {
                        PendingIndicator(id: "game-status")
                    } else {
                        StatusMessage(text: status, identifier: locked ? "input-lock" : "game-status", alert: alert)
                            .lineLimit(typeSize.isAccessibilitySize ? nil : 1).minimumScaleFactor(0.8)
                    }
                }
                Spacer(minLength: 0)
            }
            .background { if neutral, let status { StatusMessage(text: status, identifier: "game-status", neutral: true) } }
        }
        .onChange(of: status) { _, text in if let text, text != L10n.t("game.incomplete") { announce(text) } }
    }

    @ViewBuilder private func bottom(view: LiveView, race: Bool, content: BoardContent?) -> some View {
        if view.reconcileRequired {
            VStack(alignment: .leading, spacing: 8) {
                Text(L10n.t("reconcile.message")).font(.subheadline).accessibilityIdentifier("reconcile-message")
                SecondaryButton(title: L10n.t("reconcile.local"), id: "reconcile-local") { live.reconcile(keepLocal: true) }
                SecondaryButton(title: L10n.t("reconcile.server"), id: "reconcile-server") { live.reconcile(keepLocal: false) }
            }
        }
        if let content { controls(race: race, content: content) }
        // Only the storage failure, which needs its Retry, stays in the flow; passing messages float above.
        LiveNotices(live: live, showsMessage: false)
    }

    private func boardContent(race: Bool) -> BoardContent? {
        let own = snapshot.own
        guard own.givens.count == snapshot.size * snapshot.size else { return nil }
        if race {
            guard let draft = live.view.race, draft.cells.count == own.givens.count else { return nil }
            return BoardContent(size: snapshot.size, values: draft.cells, givens: own.givens,
                                notes: { draft.notes.indices.contains($0) ? draft.notes[$0] : [] },
                                conflicts: Set(draft.conflicts), selected: live.selected)
        }
        guard own.cells.count == own.givens.count else { return nil }
        return BoardContent(size: snapshot.size, values: own.cells, givens: own.givens, notes: { _ in [] },
                            conflicts: [], selected: live.selected, locked: { own.cells[$0] != 0 })
    }

    @ViewBuilder private func controls(race: Bool, content: BoardContent) -> some View {
        let n = snapshot.size, index = live.selected
        let value = content.values[index]
        let editable = live.canInput() && content.givens[index] == 0 && (race || value == 0)
        if UIDebug.enabled {
            Text(content.givens[index] != 0 ? L10n.t("selection.given")
                 : L10n.t("selection", ["row": index / n + 1, "column": index % n + 1]))
                .font(.subheadline).foregroundStyle(Theme.muted)
                .accessibilityHidden(true)
        }
        NumberPad(size: n, values: content.values, notesMode: race && live.notesMode, enabled: editable) { digit in
            let before = live.view.race?.conflicts ?? []
            live.enter(digit)
            if race {
                let now = live.view.race?.conflicts ?? []
                Feedback.shared.play(live.notesMode ? .note : now.contains(index) && !before.contains(index) ? .error : .tap)
            } else {
                Feedback.shared.play(.tap)
            }
        }
        .accessibilityHint(race ? "" : L10n.t("live.duelRules"))
        if race, let draft = live.view.race {
            let tools = typeSize.isAccessibilitySize ? AnyLayout(VStackLayout(spacing: 8)) : AnyLayout(HStackLayout(spacing: 4))
            tools {
                NotesToggle(isOn: $live.notesMode, enabled: live.canInput()) { Feedback.shared.play(.select) }
                ToolButton(title: L10n.t("game.erase"), icon: "eraser", id: "erase", enabled: editable) {
                    live.edit("erase"); Feedback.shared.play(.tap)
                }
                ToolButton(title: L10n.t("game.undo"), icon: "arrow.uturn.backward", id: "undo",
                           enabled: draft.canUndo && live.canInput()) { live.edit("undo"); Feedback.shared.play(.tap) }
                    .keyboardShortcut("z", modifiers: .command)
                ToolButton(title: L10n.t("game.redo"), icon: "arrow.uturn.forward", id: "redo",
                           enabled: draft.canRedo && live.canInput()) { live.edit("redo"); Feedback.shared.play(.tap) }
                    .keyboardShortcut("z", modifiers: [.command, .shift])
            }
        } else if UIDebug.enabled {
            Text(L10n.t("live.duelRules")).font(.footnote).foregroundStyle(Theme.muted)
        }
    }

    private func timerText(_ view: LiveView, race: Bool) -> String {
        if race {
            guard let now = view.estimatedServerMs, let start = snapshot.startMs else { return L10n.t("game.syncing") }
            return clock(now > start ? now - start : 0)
        }
        return view.remainingMs.map { clock(($0 + 999) / 1000 * 1000) } ?? L10n.t("game.syncing")
    }

    private func statusText(race: Bool, conflicts: Bool) -> String {
        let view = live.view
        let lock = view.lockedRemainingMs ?? 0
        let receipt = view.lastReceipt.flatMap { $0.matchId == snapshot.matchId ? $0.code : nil }
        if !race && lock > 0 { return L10n.t("status.lock", ["seconds": (lock + 999) / 1000]) }
        if !race && view.remainingMs == 0 { return L10n.t("status.timeUp") }
        if view.pendingCommand { return L10n.t("status.pending") }
        if receipt == "wrong" { return L10n.t("status.wrong") }
        if receipt == "not_correct" { return L10n.t("status.notCorrect") }
        if conflicts { return L10n.t("game.conflict") }
        return L10n.t("game.incomplete")
    }
}

struct LiveResultView: View {
    let live: LiveSession
    let snapshot: LiveView.Snapshot

    var body: some View {
        let own = snapshot.own, opponent = snapshot.opponent, result = snapshot.result
        let reason = result?.reason
        let outcome: String = {
            guard let result, !["both_disconnected", "resource_limit"].contains(result.reason) else { return "noContest" }
            guard let winner = result.winner else { return "draw" }
            return winner == own.seat ? "win" : "lose"
        }()
        let symbol = outcome == "win" ? "trophy.fill" : outcome == "lose" ? "flag.fill" : "equal"
        let tint = outcome == "win" ? Theme.accent : outcome == "lose" ? Theme.rival : Theme.muted
        let fill = outcome == "win" ? Theme.selected : outcome == "lose" ? Theme.rivalSoft : Theme.track
        CenteredPanel(symbol: symbol, symbolTint: tint, symbolFill: fill, eyebrow: roomLine(snapshot),
                      title: L10n.t("result.\(outcome)"), titleId: "result-heading") {
            ConnectionStatus(connection: live.view.connection)
            LiveNotices(live: live)
            // An empty seat (friend left) has no score to show; never print a fake 0.
            Group {
                if snapshot.mode == "duel" {
                    HStack(spacing: 14) {
                        score(name: own.name, value: own.score, rival: false)
                        Text("—").foregroundStyle(Theme.muted).accessibilityHidden(true)
                        if let opponent { score(name: opponent.name, value: opponent.score ?? 0, rival: true) }
                    }
                    .padding(14)
                    .frame(maxWidth: .infinity)
                    .card(radius: 18)
                } else {
                    Text(L10n.t("result.raceDone")).foregroundStyle(Theme.muted)
                }
            }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel(snapshot.mode != "duel" ? L10n.t("result.raceDone")
                                : opponent.map { L10n.t("result.duelScore", ["own": own.score, "opponent": $0.score ?? 0]) }
                                ?? L10n.t("result.ownScore", ["own": own.score]))
            .accessibilityIdentifier("result-details")
            Text(L10n.t("reason.\(reason ?? "completed")")).font(.subheadline).foregroundStyle(Theme.muted)
                .multilineTextAlignment(.center)
            Group {
                if opponent == nil {
                    Text(L10n.t("rematch.opponentLeft"))
                } else if own.rematch {
                    Text(L10n.t("rematch.mine"))
                } else if opponent?.rematch == true {
                    Text(L10n.t("rematch.theirs"))
                }
            }
            .font(.subheadline).foregroundStyle(Theme.muted).multilineTextAlignment(.center)
            VStack(spacing: 10) {
                PrimaryButton(title: L10n.t("rematch.ask"), id: "rematch") { Feedback.shared.play(.tap); live.command("rematch") }
                    .disabled(opponent == nil || own.rematch || live.view.pendingCommand || live.view.connection != "connected")
                if live.view.connection == "session_lost" {
                    PrimaryButton(title: L10n.t("lost.reset"), id: "live-recover") { live.requestReset() }
                }
                SecondaryButton(title: L10n.t("nav.home"), id: "home") { live.goHome() }
            }
            .padding(.top, 8)
        }
        .onAppear { announce(L10n.t("result.\(outcome)")) }
    }

    private func score(name: String, value: Int, rival: Bool) -> some View {
        VStack(spacing: 2) {
            Text(name).font(.footnote.weight(.semibold)).foregroundStyle(rival ? Theme.rival : Theme.accent).lineLimit(1)
            Text("\(value)").font(Theme.display(.largeTitle).monospacedDigit())
        }
        .frame(maxWidth: .infinity)
    }
}
