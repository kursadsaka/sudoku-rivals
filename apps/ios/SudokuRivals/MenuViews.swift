import SwiftUI

struct PrimaryButton: View {
    let title: String
    let id: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(title).font(.headline).frame(maxWidth: .infinity, minHeight: 48)
        }
        .buttonStyle(.borderedProminent)
        .foregroundStyle(Theme.onAccent)
        .accessibilityIdentifier(id)
    }
}

struct SecondaryButton: View {
    let title: String
    let id: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(title).font(.headline).frame(maxWidth: .infinity, minHeight: 48)
        }
        .buttonStyle(.bordered)
        .accessibilityIdentifier(id)
    }
}

struct HomeView: View {
    let app: AppModel

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Button { app.setupError = nil; app.screen = .setup } label: {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(L10n.t("home.solo")).font(.title3.bold())
                        Text(L10n.t("home.soloDetail")).font(.subheadline)
                    }
                    .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
                    .padding(.vertical, 8)
                }
                .buttonStyle(.borderedProminent)
                .foregroundStyle(Theme.onAccent)
                .accessibilityIdentifier("solo-start")

                Button { app.openLive() } label: {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(L10n.t("home.friends")).font(.title3.bold())
                        Text(L10n.t("home.friendsDetail")).font(.subheadline)
                    }
                    .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
                    .padding(.vertical, 8)
                }
                .buttonStyle(.bordered)
                .accessibilityIdentifier("friend-start")

                let ready = SoloStore.sizes.filter { app.status($0).isReadable }
                if !ready.isEmpty {
                    Text(L10n.t("home.continue")).font(.headline).padding(.top, 8)
                    VStack(spacing: 8) {
                        ForEach(ready, id: \.self) { size in resumeButton(size) }
                    }
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("solo-resume")
                }

                ForEach(SoloStore.sizes, id: \.self) { size in
                    if let message = problem(size) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(message, systemImage: "exclamationmark.triangle").foregroundStyle(Theme.error)
                            Spacer()
                            Button(L10n.t("error.retry")) { app.refreshSlots() }
                                .frame(minHeight: 48)
                                .accessibilityIdentifier("retry")
                        }
                        .accessibilityElement(children: .contain)
                        .accessibilityIdentifier("storage-status")
                    }
                }
            }
            .padding()
        }
        .onAppear { app.refreshSlots() }
    }

    @ViewBuilder private func resumeButton(_ size: Int) -> some View {
        if case .ready(let envelope, let complete) = app.status(size) {
        Button { app.resume(size) } label: {
            HStack {
                VStack(alignment: .leading) {
                    Text("\(size)×\(size)").font(.headline)
                    Text("\(L10n.t(complete ? "home.completed" : "home.savedGame")) · \(L10n.time(envelope.elapsedMs))")
                        .font(.subheadline).foregroundStyle(Theme.muted)
                }
                Spacer()
                Text(complete ? L10n.t("home.completed") : L10n.t("home.resume")).font(.subheadline)
                Image(systemName: "chevron.right").accessibilityHidden(true)
            }
            .frame(maxWidth: .infinity, minHeight: 48)
        }
        .buttonStyle(.bordered)
        .accessibilityIdentifier("solo-resume-\(size)")
        }
    }

    private func problem(_ size: Int) -> String? {
        let name = "\(size)×\(size)"
        switch app.status(size) {
        case .corrupt: return L10n.t("storage.corrupt", ["size": name])
        case .unsupportedVersion: return L10n.t("storage.unsupported", ["size": name])
        case .unavailable: return L10n.t("storage.unavailable", ["size": name])
        case .missing, .ready: return nil
        }
    }
}

struct SetupView: View {
    let app: AppModel
    @State private var size = 6
    @State private var confirming = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text(L10n.t("setup.title")).font(.title2.bold()).accessibilityAddTraits(.isHeader)
                Text(L10n.t("setup.size")).font(.headline)
                Picker(L10n.t("setup.size"), selection: $size) {
                    Text("6×6").tag(6)
                    Text("9×9").tag(9)
                }
                .pickerStyle(.segmented)
                .accessibilityIdentifier("board-size")
                Text(L10n.t(size == 6 ? "setup.six" : "setup.nine")).foregroundStyle(Theme.muted)
                Text(L10n.t("setup.generator")).font(.footnote).foregroundStyle(Theme.muted)
                Text(L10n.t("setup.difficulty")).font(.footnote).foregroundStyle(Theme.muted)
                    .accessibilityIdentifier("difficulty-note")

                let status = app.status(size)
                if status.exists {
                    Label(L10n.t(status.isReadable ? "setup.hasSave" : "setup.hasUnreadable"),
                          systemImage: status.isReadable ? "info.circle" : "exclamationmark.triangle")
                        .foregroundStyle(status.isReadable ? Theme.foreground : Theme.error)
                        .accessibilityIdentifier("replace-note")
                }
                if let error = app.setupError {
                    Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(Theme.error)
                        .accessibilityIdentifier("setup-error")
                }
                PrimaryButton(title: L10n.t("setup.start"), id: "game-start") {
                    app.refreshSlots()
                    if app.status(size).exists { confirming = true } else { app.startNew(size: size) }
                }
            }
            .padding()
        }
        .toolbar {
            ToolbarItem(placement: .topBarLeading) {
                Button { app.screen = .home } label: { Label(L10n.t("nav.back"), systemImage: "chevron.backward") }
                    .accessibilityIdentifier("back")
            }
        }
        .alert(L10n.t(app.status(size).isReadable ? "confirm.replace" : "confirm.replaceUnreadable"),
               isPresented: $confirming) {
            Button(L10n.t("confirm.continue"), role: .destructive) { app.startNew(size: size) }
                .accessibilityIdentifier("confirm-replace")
            Button(L10n.t("confirm.cancel"), role: .cancel) {}
        }
        .onAppear { app.refreshSlots() }
    }
}
