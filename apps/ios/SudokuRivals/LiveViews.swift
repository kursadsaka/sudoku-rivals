import SwiftUI
import UIKit

/// Invite-based 1v1 live matches: setup → lobby/countdown → race or duel → result.
/// Every rule and result comes from the server through the shared Rust client.
struct LiveScreen: View {
    @Bindable var live: LiveSession

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 12) {
                if live.storageError {
                    VStack(alignment: .leading, spacing: 8) {
                        Label(L10n.t("live.storageError"), systemImage: "exclamationmark.triangle").foregroundStyle(Theme.error)
                        Button(L10n.t("live.storageRetry")) { live.retryStorage() }
                            .buttonStyle(.bordered).frame(minHeight: 48)
                            .accessibilityIdentifier("live-save-retry")
                    }
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("live-storage-status")
                }
                if let message = live.message {
                    Label(message, systemImage: "exclamationmark.circle")
                        .foregroundStyle(Theme.error)
                        .accessibilityElement(children: .combine)
                        .accessibilityIdentifier("live-message")
                }
                content
            }
            .padding()
        }
        .toolbar {
            ToolbarItem(placement: .topBarLeading) {
                Button { live.goHome() } label: { Label(L10n.t("nav.home"), systemImage: "house") }
                    .accessibilityIdentifier("back")
            }
        }
        .onAppear { live.open() }
        .task {
            while !Task.isCancelled {
                live.tick()
                try? await Task.sleep(for: .milliseconds(250))
            }
        }
        .onChange(of: live.message) { _, message in if let message { announce(message) } }
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

    @ViewBuilder private var content: some View {
        let view = live.view
        if live.protectedSave {
            VStack(alignment: .leading, spacing: 12) {
                Text(L10n.t("protected.title")).font(.title2.bold()).accessibilityAddTraits(.isHeader)
                Text(L10n.t("protected.info"))
                PrimaryButton(title: L10n.t("protected.reset"), id: "live-recover") { live.requestReset() }
            }
        } else if let snapshot = view.snapshot, !view.voluntarilyLeft {
            if view.connection == "session_lost" || snapshot.phase == "closed" {
                VStack(alignment: .leading, spacing: 12) {
                    ConnectionStatus(connection: view.connection)
                    Text(L10n.t("lost.title")).font(.title2.bold()).accessibilityAddTraits(.isHeader)
                        .accessibilityIdentifier("result-heading")
                    Text(L10n.t(snapshot.mode == "race" ? "lost.detailRace" : "lost.detail")).accessibilityIdentifier("result-details")
                    PrimaryButton(title: L10n.t("lost.reset"), id: "live-recover") { live.requestReset() }
                    SecondaryButton(title: L10n.t("nav.home"), id: "home") { live.goHome() }
                }
            } else if snapshot.phase == "finished" {
                LiveResultView(live: live, snapshot: snapshot)
            } else if snapshot.phase == "lobby" || snapshot.phase == "countdown" {
                LiveLobbyView(live: live, snapshot: snapshot)
            } else {
                LiveGameView(live: live, snapshot: snapshot)
            }
        } else {
            LiveSetupView(live: live)
        }
    }
}

private func announce(_ text: String) { AccessibilityNotification.Announcement(text).post() }

private func modeName(_ mode: String) -> String { L10n.t(mode == "race" ? "live.race" : "live.duel") }

private func clock(_ ms: UInt64) -> String { String(format: "%02d:%02d", ms / 60_000, ms / 1000 % 60) }

struct ConnectionStatus: View {
    let connection: String

    var body: some View {
        let key = ["connected", "reconnecting", "reconcile", "session_lost"].contains(connection) ? connection : "disconnected"
        let text = L10n.t("conn.\(key)")
        Label(text, systemImage: connection == "connected" ? "wifi" : "wifi.exclamationmark")
            .font(.subheadline)
            .foregroundStyle(connection == "connected" ? Theme.muted : Theme.error)
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("connection-status")
            .onChange(of: connection) { announce(text) }
    }
}

struct LiveSetupView: View {
    @Bindable var live: LiveSession

    var body: some View {
        let busy = live.view.pendingHttp
        VStack(alignment: .leading, spacing: 12) {
            Text(L10n.t("home.friends")).font(.title2.bold()).accessibilityAddTraits(.isHeader)
            if !live.isConfigured { Text(L10n.t("live.notConfigured")).foregroundStyle(Theme.error) }
            Text(L10n.t("live.name")).font(.headline)
            TextField(L10n.t("live.name"), text: $live.name)
                .textFieldStyle(.roundedBorder)
                .textContentType(.nickname)
                .autocorrectionDisabled()
                .frame(minHeight: 44)
                .accessibilityIdentifier("guest-name")
            Text(L10n.t("live.mode")).font(.headline)
            Picker(L10n.t("live.mode"), selection: $live.mode) {
                Text(L10n.t("live.race")).tag("race")
                Text(L10n.t("live.duel")).tag("duel")
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("game-mode")
            Text(L10n.t(live.mode == "race" ? "live.raceDetail" : "live.duelDetail")).foregroundStyle(Theme.muted)
            Text(L10n.t("setup.size")).font(.headline)
            Picker(L10n.t("setup.size"), selection: $live.size) {
                Text("6×6").tag(6)
                Text("9×9").tag(9)
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("board-size")
            PrimaryButton(title: L10n.t(busy ? "live.waiting" : "live.create"), id: "room-create") { live.create() }
                .disabled(busy || !live.isConfigured)
            Text(L10n.t("live.or")).font(.headline).padding(.top, 8).accessibilityAddTraits(.isHeader)
            TextField(L10n.t("live.invite"), text: $live.invite)
                .textFieldStyle(.roundedBorder)
                .textInputAutocapitalization(.never) // links are case-sensitive; the core uppercases codes
                .keyboardType(.asciiCapable)
                .autocorrectionDisabled()
                .frame(minHeight: 44)
                .accessibilityIdentifier("room-code")
            SecondaryButton(title: L10n.t("live.join"), id: "room-join") { live.join() }
                .disabled(busy || !live.isConfigured || live.invite.trimmingCharacters(in: .whitespaces).isEmpty)
        }
        .disabled(busy)
    }
}

struct LiveLobbyView: View {
    let live: LiveSession
    let snapshot: LiveView.Snapshot
    @State private var copied = false

    var body: some View {
        let countdown = snapshot.phase == "countdown"
        let own = snapshot.own, view = live.view
        VStack(alignment: .leading, spacing: 12) {
            Text(L10n.t(countdown ? "lobby.countdown" : "lobby.waiting")).font(.title2.bold())
                .accessibilityAddTraits(.isHeader)
            Text("\(modeName(snapshot.mode)) · \(snapshot.size)×\(snapshot.size)").foregroundStyle(Theme.muted)
            ConnectionStatus(connection: view.connection)
            Text(snapshot.roomCode)
                .font(.largeTitle.monospaced().bold())
                .textSelection(.enabled)
                .accessibilityLabel(L10n.t("lobby.code", ["code": snapshot.roomCode.map(String.init).joined(separator: " ")]))
                .accessibilityValue(snapshot.roomCode)
                .accessibilityIdentifier("room-code")
            HStack {
                Button {
                    UIPasteboard.general.string = snapshot.roomCode
                    copied = true
                    announce(L10n.t("lobby.copied"))
                } label: {
                    Label(L10n.t(copied ? "lobby.copied" : "lobby.copy"), systemImage: "doc.on.doc").frame(minHeight: 44)
                }
                .buttonStyle(.bordered)
                .accessibilityIdentifier("invite-copy")
                if let url = live.inviteURL {
                    ShareLink(item: url) {
                        Label(L10n.t("lobby.share"), systemImage: "square.and.arrow.up").frame(minHeight: 44)
                    }
                    .buttonStyle(.bordered)
                    .accessibilityIdentifier("invite-share")
                }
            }
            VStack(alignment: .leading, spacing: 4) {
                Text(L10n.t("lobby.you", ["name": own.name, "state": L10n.t(own.ready ? "lobby.ready" : "lobby.notReady")]))
                if let opponent = snapshot.opponent {
                    Text(opponent.name + " · " + L10n.t(opponent.ready ? "lobby.ready" : "lobby.notReady")
                         + (opponent.connected ? "" : L10n.t("lobby.opponentOffline")))
                } else {
                    Text(L10n.t("lobby.noOpponent")).foregroundStyle(Theme.muted)
                }
            }
            .accessibilityElement(children: .combine)
            .accessibilityIdentifier("players")
            if countdown {
                Text(countdownText(view)).font(.title2.monospacedDigit()).accessibilityIdentifier("timer")
            }
            PrimaryButton(title: L10n.t(own.ready ? "lobby.unready" : "lobby.setReady"), id: "ready-toggle") {
                live.command("ready", ready: !own.ready)
            }
            .disabled(snapshot.opponent == nil || view.connection != "connected" || view.pendingCommand)
        }
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
    @Environment(\.horizontalSizeClass) private var widthClass

    var body: some View {
        let view = live.view
        let race = snapshot.mode == "race"
        VStack(alignment: .leading, spacing: 12) {
            ConnectionStatus(connection: view.connection)
            HStack {
                Text("\(modeName(snapshot.mode)) · \(snapshot.size)×\(snapshot.size)").font(.headline)
                Spacer()
                Text(timerText(view, race: race)).font(.body.monospacedDigit())
                    .accessibilityLabel(L10n.t(race ? "live.elapsed" : "live.remaining") + " " + timerText(view, race: race))
                    .accessibilityIdentifier("timer")
            }
            scoreLine(race: race)
            if view.reconcileRequired {
                VStack(alignment: .leading, spacing: 8) {
                    Text(L10n.t("reconcile.message")).accessibilityIdentifier("reconcile-message")
                    SecondaryButton(title: L10n.t("reconcile.local"), id: "reconcile-local") { live.reconcile(keepLocal: true) }
                    SecondaryButton(title: L10n.t("reconcile.server"), id: "reconcile-server") { live.reconcile(keepLocal: false) }
                }
            }
            if let content = boardContent(race: race) {
                let status = statusText(race: race, conflicts: !content.conflicts.isEmpty)
                let locked = !race && (view.lockedRemainingMs ?? 0) > 0
                Text(status)
                    .foregroundStyle(locked || !content.conflicts.isEmpty ? Theme.error : Theme.foreground)
                    .accessibilityIdentifier(locked ? "input-lock" : "game-status")
                    .onChange(of: status) { _, text in if text != L10n.t("game.incomplete") { announce(text) } }
                if widthClass == .regular {
                    HStack(alignment: .top, spacing: 24) {
                        BoardView(content: content) { live.selected = $0 }.frame(maxWidth: 560)
                        VStack(alignment: .leading, spacing: 12) { controls(race: race, content: content) }.frame(maxWidth: 420)
                    }
                } else {
                    BoardView(content: content) { live.selected = $0 }
                    controls(race: race, content: content)
                }
            } else {
                ProgressView(L10n.t("game.syncing"))
            }
        }
        .background { ArrowKeys { live.moveSelection(rows: $0, columns: $1) } }
        .background {
            DeleteKeyCatcher { if race, live.canInput() { live.edit("erase") } }.accessibilityHidden(true)
        }
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

    @ViewBuilder private func scoreLine(race: Bool) -> some View {
        if race {
            Text(L10n.t("game.opponentProgress", ["percent": snapshot.opponent?.progressPercent ?? 0]))
                .accessibilityIdentifier("opponent-progress")
        } else {
            ViewThatFits(in: .horizontal) {
                HStack { scores }
                VStack(alignment: .leading) { scores }
            }
        }
    }

    @ViewBuilder private var scores: some View {
        Text(L10n.t("game.you", ["score": snapshot.own.score])).bold().accessibilityIdentifier("own-score")
        Spacer()
        Text(L10n.t("game.opponentScore", ["score": snapshot.opponent?.score ?? 0])).accessibilityIdentifier("opponent-score")
        Spacer()
        Text(L10n.t("game.combo", ["combo": snapshot.own.combo])).accessibilityIdentifier("combo")
    }

    @ViewBuilder private func controls(race: Bool, content: BoardContent) -> some View {
        let n = snapshot.size, index = live.selected
        let value = content.values[index]
        let editable = live.canInput() && content.givens[index] == 0 && (race || value == 0)
        Text(content.givens[index] != 0 ? L10n.t("selection.given")
             : L10n.t("selection", ["row": index / n + 1, "column": index % n + 1]))
            .font(.subheadline).foregroundStyle(Theme.muted)
            .accessibilityHidden(true)
        NumberPad(size: n, notesMode: race && live.notesMode, enabled: editable) { live.enter($0) }
        if race, let draft = live.view.race {
            HStack(spacing: 4) {
                Toggle(isOn: $live.notesMode) { ToolLabel(title: L10n.t("game.notes"), icon: "pencil") }
                    .toggleStyle(.button)
                    .keyboardShortcut("n", modifiers: [])
                    .accessibilityIdentifier("notes-toggle")
                ToolButton(title: L10n.t("game.erase"), icon: "delete.left", id: "erase", enabled: editable) { live.edit("erase") }
                ToolButton(title: L10n.t("game.undo"), icon: "arrow.uturn.backward", id: "undo",
                           enabled: draft.canUndo && live.canInput()) { live.edit("undo") }
                    .keyboardShortcut("z", modifiers: .command)
                ToolButton(title: L10n.t("game.redo"), icon: "arrow.uturn.forward", id: "redo",
                           enabled: draft.canRedo && live.canInput()) { live.edit("redo") }
                    .keyboardShortcut("z", modifiers: [.command, .shift])
            }
        } else {
            Text(L10n.t("live.duelRules")).font(.footnote).foregroundStyle(Theme.muted)
        }
    }

    private func timerText(_ view: LiveView, race: Bool) -> String {
        if race {
            guard let now = view.estimatedServerMs, let start = snapshot.startMs else { return L10n.t("game.syncing") }
            return clock(now > start ? now - start : 0)
        }
        return view.remainingMs.map(clock) ?? L10n.t("game.syncing")
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
        let title: String = {
            guard let result, !["both_disconnected", "resource_limit"].contains(result.reason) else { return "result.noContest" }
            guard let winner = result.winner else { return "result.draw" }
            return winner == own.seat ? "result.win" : "result.lose"
        }()
        VStack(alignment: .leading, spacing: 12) {
            Text("\(modeName(snapshot.mode)) · \(snapshot.size)×\(snapshot.size)").font(.subheadline).foregroundStyle(Theme.muted)
            Text(L10n.t(title)).font(.title.bold()).accessibilityAddTraits(.isHeader)
                .accessibilityIdentifier("result-heading")
            ConnectionStatus(connection: live.view.connection)
            // An empty seat (friend left) has no score to show; never print a fake 0.
            Text(snapshot.mode != "duel" ? L10n.t("result.raceDone")
                 : opponent.map { L10n.t("result.duelScore", ["own": own.score, "opponent": $0.score ?? 0]) }
                    ?? L10n.t("result.ownScore", ["own": own.score]))
                .accessibilityIdentifier("result-details")
            Text(L10n.t("reason.\(reason ?? "completed")"))
            if opponent == nil {
                Text(L10n.t("rematch.opponentLeft"))
            } else if own.rematch {
                Text(L10n.t("rematch.mine"))
            } else if opponent?.rematch == true {
                Text(L10n.t("rematch.theirs"))
            }
            PrimaryButton(title: L10n.t("rematch.ask"), id: "rematch") { live.command("rematch") }
                .disabled(opponent == nil || own.rematch || live.view.pendingCommand || live.view.connection != "connected")
            SecondaryButton(title: L10n.t("nav.home"), id: "home") { live.goHome() }
        }
        .onAppear { announce(L10n.t(title)) }
    }
}
