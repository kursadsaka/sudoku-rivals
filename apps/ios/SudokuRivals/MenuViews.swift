import SwiftUI
import UIKit

// MARK: Shared building blocks

/// Presses settle slightly, like the web's `:active { scale(.97) }`; no movement under Reduce Motion.
struct PressStyle: ButtonStyle {
    var fill: Color
    var foreground: Color
    var stroke: Color
    var radius: CGFloat = 16
    var shadow = false
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.isEnabled) private var enabled

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .foregroundStyle(foreground)
            .background(fill, in: RoundedRectangle(cornerRadius: radius, style: .continuous))
            .overlay { RoundedRectangle(cornerRadius: radius, style: .continuous).strokeBorder(stroke, lineWidth: 1) }
            .shadow(color: .black.opacity(shadow ? 0.08 : 0), radius: 6, y: 3)
            .opacity(enabled ? 1 : 0.45)
            .scaleEffect(configuration.isPressed && !reduceMotion ? 0.97 : 1)
            .animation(.easeOut(duration: 0.14), value: configuration.isPressed)
    }
}

struct PrimaryButton: View {
    let title: String
    let id: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(title).font(.headline).multilineTextAlignment(.center)
                .frame(maxWidth: .infinity, minHeight: 52).padding(.horizontal, 16)
        }
        .buttonStyle(PressStyle(fill: Theme.brand, foreground: Theme.onBrand, stroke: Theme.brand, shadow: true))
        .accessibilityIdentifier(id)
    }
}

struct SecondaryButton: View {
    let title: String
    let id: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(title).font(.headline).multilineTextAlignment(.center)
                .frame(maxWidth: .infinity, minHeight: 52).padding(.horizontal, 16)
        }
        .buttonStyle(PressStyle(fill: Theme.background, foreground: Theme.accent, stroke: Theme.line))
        .accessibilityIdentifier(id)
    }
}

/// Eyebrow · title · lede, the opening of every web page.
struct PageHeader: View {
    var eyebrow: String?
    let title: String
    var lede: String?
    var large = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let eyebrow {
                Text(eyebrow.uppercased(with: L10n.turkish ? Locale(identifier: "tr") : Locale(identifier: "en")))
                    .font(.caption.weight(.bold)).tracking(1.4).foregroundStyle(Theme.accent)
            }
            Text(title)
                .font(Theme.display(large ? .largeTitle : .title, weight: .bold))
                .foregroundStyle(Theme.foreground)
                .accessibilityAddTraits(.isHeader)
            if let lede {
                Text(lede).foregroundStyle(Theme.muted).fixedSize(horizontal: false, vertical: true)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct SectionLabel: View {
    let text: String
    var body: some View {
        Text(text.uppercased(with: L10n.turkish ? Locale(identifier: "tr") : Locale(identifier: "en")))
            .font(.caption.weight(.semibold)).tracking(1.2).foregroundStyle(Theme.muted)
            .accessibilityAddTraits(.isHeader)
    }
}

/// Toolbar back button used by the menu pages (games draw their own navigation row).
struct BackToolbar: ToolbarContent {
    var title = L10n.t("nav.back")
    let action: () -> Void

    var body: some ToolbarContent {
        ToolbarItem(placement: .topBarLeading) {
            Button(action: action) { Label(title, systemImage: "chevron.backward").labelStyle(.titleAndIcon) }
                .accessibilityIdentifier("back")
        }
    }
}

/// Five levels in one row (widths follow the words, each ≥ 48 pt); a column at accessibility text sizes.
struct LevelPicker: View {
    @Binding var selection: String
    var disabled = false
    let changed: () -> Void

    var body: some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 4) { options }
            VStack(spacing: 4) { options }
        }
        .padding(4)
        .background(Theme.track, in: RoundedRectangle(cornerRadius: 16, style: .continuous))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(L10n.t("setup.difficulty"))
        .accessibilityIdentifier("difficulty")
    }

    private var options: some View {
        ForEach(AppPreferences.levels, id: \.self) { level in
            let on = level == selection
            Button {
                guard selection != level else { return }
                selection = level
                changed()
            } label: {
                Text(L10n.t("level.\(level)"))
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(1).minimumScaleFactor(0.75)
                    .foregroundStyle(on ? Theme.foreground : Theme.muted)
                    .frame(maxWidth: .infinity, minHeight: 44)
                    .padding(.horizontal, 4)
                    .background {
                        if on {
                            RoundedRectangle(cornerRadius: 12, style: .continuous).fill(Theme.background)
                                .overlay { RoundedRectangle(cornerRadius: 12, style: .continuous).strokeBorder(Theme.accent, lineWidth: 1.5) }
                                .shadow(color: .black.opacity(0.06), radius: 3, y: 1)
                        }
                    }
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .frame(minWidth: 48)
            .disabled(disabled)
            .accessibilityAddTraits(on ? .isSelected : [])
            .accessibilityIdentifier("difficulty-\(level)")
        }
    }
}

/// A large selectable card (board size, live mode): checked = tinted with an accent edge and a tick.
struct ChoiceCard<Art: View>: View {
    let title: String
    let detail: String
    let selected: Bool
    let id: String
    var titleFont: Font = Theme.display(.title2, weight: .bold)
    @ViewBuilder let art: () -> Art
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 6) {
                art().foregroundStyle(selected ? Theme.accent : Theme.muted)
                Text(title).font(titleFont).foregroundStyle(Theme.foreground)
                    .padding(.trailing, 24)
                Text(detail).font(.footnote).foregroundStyle(Theme.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .padding(14)
            .background(selected ? Theme.selected : Theme.background, in: RoundedRectangle(cornerRadius: 18, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: 18, style: .continuous)
                    .strokeBorder(selected ? Theme.accent : Theme.line, lineWidth: 1.5)
            }
            .overlay(alignment: .topTrailing) {
                if selected {
                    Image(systemName: "checkmark.circle.fill").font(.title3)
                        .symbolRenderingMode(.palette).foregroundStyle(Theme.onBrand, Theme.brand)
                        .padding(10).accessibilityHidden(true)
                }
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? .isSelected : [])
        .accessibilityIdentifier(id)
    }
}

/// The board-size glyph: a rounded square with the size's box lines.
struct SizeArt: View {
    let size: Int
    var body: some View {
        Canvas { context, canvas in
            let w = canvas.width, inset: CGFloat = 1.5
            var path = Path(roundedRect: CGRect(x: inset, y: inset, width: w - 2 * inset, height: w - 2 * inset), cornerRadius: w * 0.17)
            let third = (w - 2 * inset) / 3
            let columns = size == 6 ? [w / 2] : [inset + third, inset + 2 * third]
            for x in columns { path.move(to: CGPoint(x: x, y: inset)); path.addLine(to: CGPoint(x: x, y: w - inset)) }
            for y in [inset + third, inset + 2 * third] { path.move(to: CGPoint(x: inset, y: y)); path.addLine(to: CGPoint(x: w - inset, y: y)) }
            context.stroke(path, with: .foreground, lineWidth: 2)
        }
        .frame(width: 40, height: 40)
        .accessibilityHidden(true)
    }
}

// MARK: Home

struct HomeView: View {
    let app: AppModel

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                PageHeader(eyebrow: L10n.t("home.eyebrow"), title: L10n.t("home.title"), lede: L10n.t("home.intro"), large: true)
                    .padding(.bottom, 6)

                ModeCard(title: L10n.t("home.solo"), detail: L10n.t("home.soloDetail"), primary: true, id: "solo-start") {
                    Image(systemName: "person.fill").font(.system(size: 30))
                } action: {
                    app.setupError = nil
                    app.screen = .setup
                }
                ModeCard(title: L10n.t("home.friends"), detail: L10n.t("home.friendsDetail"), primary: false, id: "friend-start") {
                    FriendsArt()
                } action: {
                    app.openLive()
                }

                if app.live.holdsSeat {
                    Button { app.openLive() } label: {
                        HStack {
                            Text(L10n.t("home.liveResume")).font(.headline)
                            Spacer()
                            Image(systemName: "arrow.right").accessibilityHidden(true)
                        }
                        .foregroundStyle(Theme.accent)
                        .padding(.horizontal, 16).frame(minHeight: 60)
                    }
                    .buttonStyle(PressStyle(fill: Theme.background, foreground: Theme.foreground, stroke: Theme.rival, radius: 18))
                    .accessibilityIdentifier("live-resume")
                }

                let ready = SoloStore.sizes.filter { app.status($0).isReadable }
                if !ready.isEmpty {
                    SectionLabel(text: L10n.t("home.continue")).padding(.top, 14)
                    VStack(spacing: 10) {
                        ForEach(ready, id: \.self) { size in resumeButton(size) }
                    }
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("solo-resume")
                }

                ForEach(SoloStore.sizes, id: \.self) { size in
                    if let message = problem(size) {
                        VStack(alignment: .leading, spacing: 6) {
                            Label(message, systemImage: "exclamationmark.triangle")
                            Button(L10n.t("error.retry")) { app.refreshSlots() }
                                .font(.subheadline.weight(.semibold)).underline()
                                .frame(minHeight: 44)
                                .accessibilityIdentifier("retry")
                        }
                        .font(.subheadline)
                        .foregroundStyle(Theme.error)
                        .padding(12)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .card(radius: 14, fill: Theme.errorBackground, stroke: Theme.error)
                        .accessibilityElement(children: .contain)
                        .accessibilityIdentifier("storage-status")
                    }
                }
            }
            .padding(.horizontal, 20)
            .padding(.top, 8)
            .padding(.bottom, 28)
            .frame(maxWidth: 720)
            .frame(maxWidth: .infinity)
        }
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button { app.openSettings() } label: { Image(systemName: "slider.horizontal.3") }
                    .accessibilityLabel(L10n.t("home.settings"))
                    .accessibilityIdentifier("settings-open")
            }
        }
        .onAppear { app.refreshSlots() }
    }

    @ViewBuilder private func resumeButton(_ size: Int) -> some View {
        if case .ready(let envelope, let complete) = app.status(size) {
            Button { app.resume(size) } label: {
                HStack(spacing: 12) {
                    Text("\(size)×\(size)")
                        .font(Theme.display(.subheadline).monospacedDigit())
                        .foregroundStyle(Theme.accent)
                        .frame(width: 44, height: 44)
                        .background(Theme.selected, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
                    VStack(alignment: .leading, spacing: 2) {
                        Text(L10n.t(complete ? "home.completed" : "home.savedGame")).font(.headline)
                            .foregroundStyle(Theme.foreground)
                        Text([app.level(size).map { L10n.t("level.\($0)") }, L10n.time(envelope.elapsedMs)]
                            .compactMap { $0 }.joined(separator: " · "))
                            .font(.subheadline.monospacedDigit()).foregroundStyle(Theme.muted)
                    }
                    Spacer(minLength: 8)
                    if !complete { Text(L10n.t("home.resume")).font(.subheadline.weight(.semibold)) }
                    Image(systemName: complete ? "arrow.right" : "play.fill").font(.subheadline).accessibilityHidden(true)
                }
                .foregroundStyle(Theme.accent)
                .padding(10).padding(.trailing, 4)
                .frame(maxWidth: .infinity, minHeight: 64)
            }
            .buttonStyle(PressStyle(fill: Theme.background, foreground: Theme.foreground, stroke: Theme.line, radius: 18))
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

/// A home mode card: art tile, title and one line, and a round arrow. Solo is the filled brand card.
struct ModeCard<Art: View>: View {
    let title: String
    let detail: String
    let primary: Bool
    let id: String
    @ViewBuilder let art: () -> Art
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 16) {
                art()
                    .frame(width: 64, height: 64)
                    .background(primary ? Color.white.opacity(0.16) : Theme.canvas,
                                in: RoundedRectangle(cornerRadius: 16, style: .continuous))
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 4) {
                    Text(title).font(Theme.display(.title3, weight: .bold))
                    Text(detail).font(.subheadline).opacity(primary ? 0.86 : 1)
                        .foregroundStyle(primary ? Theme.onBrand : Theme.muted)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                Image(systemName: "arrow.right").font(.subheadline.weight(.semibold))
                    .foregroundStyle(primary ? Theme.brand : Theme.accent)
                    .frame(width: 36, height: 36)
                    .background(primary ? Theme.onBrand : Theme.selected, in: Circle())
                    .accessibilityHidden(true)
            }
            .padding(16)
            .frame(maxWidth: .infinity, minHeight: 100)
        }
        .buttonStyle(PressStyle(fill: primary ? Theme.brand : Theme.background,
                                foreground: primary ? Theme.onBrand : Theme.foreground,
                                stroke: primary ? Theme.brand : Theme.line, radius: 22, shadow: true))
        .accessibilityIdentifier(id)
    }
}

/// Two players: you in blue in front, your rival in amber behind.
struct FriendsArt: View {
    var body: some View {
        ZStack {
            Image(systemName: "person.fill").font(.system(size: 26)).foregroundStyle(Theme.rival).offset(x: 9, y: -2)
            Image(systemName: "person.fill").font(.system(size: 30)).foregroundStyle(Theme.accent).offset(x: -7, y: 2)
                .shadow(color: Theme.canvas, radius: 0, x: 1.5, y: 0)
        }
        .accessibilityHidden(true)
    }
}

// MARK: Solo setup

struct SetupView: View {
    let app: AppModel
    @State private var confirming = false

    var body: some View {
        @Bindable var prefs = app.prefs
        let size = prefs.soloSize
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                PageHeader(eyebrow: L10n.t("setup.eyebrow"), title: L10n.t("setup.title"), lede: L10n.t("setup.intro"))

                VStack(alignment: .leading, spacing: 8) {
                    SectionLabel(text: L10n.t("setup.size"))
                    HStack(spacing: 12) {
                        ForEach([6, 9], id: \.self) { option in
                            ChoiceCard(title: "\(option)×\(option)", detail: L10n.t(option == 6 ? "setup.six" : "setup.nine"),
                                       selected: size == option, id: "size-\(option)") {
                                SizeArt(size: option).padding(.bottom, 4)
                            } action: {
                                guard prefs.soloSize != option else { return }
                                prefs.soloSize = option
                                app.cue(.select)
                            }
                            .disabled(app.preparing)
                        }
                    }
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityElement(children: .contain)
                    .accessibilityIdentifier("board-size")
                }

                VStack(alignment: .leading, spacing: 8) {
                    SectionLabel(text: L10n.t("setup.difficulty"))
                    LevelPicker(selection: $prefs.soloDifficulty, disabled: app.preparing) { app.cue(.select) }
                }

                let status = app.status(size)
                if status.exists {
                    Label(L10n.t(status.isReadable ? "setup.hasSave" : "setup.hasUnreadable"),
                          systemImage: status.isReadable ? "info.circle" : "exclamationmark.triangle")
                        .font(.subheadline)
                        .foregroundStyle(status.isReadable ? Theme.muted : Theme.error)
                        .accessibilityIdentifier("replace-note")
                }
                if let error = app.setupError {
                    Label(error, systemImage: "exclamationmark.triangle").foregroundStyle(Theme.error)
                        .accessibilityIdentifier("setup-error")
                }
                PrimaryButton(title: L10n.t(app.preparing ? "setup.preparing" : "setup.start"), id: "game-start") {
                    app.refreshSlots()
                    if app.status(size).exists { confirming = true } else { app.startNew(size: size, difficulty: prefs.soloDifficulty) }
                }
                .disabled(app.preparing)
                Text(L10n.t("setup.generator")).font(.footnote).foregroundStyle(Theme.muted)
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 8)
            .frame(maxWidth: 520)
            .frame(maxWidth: .infinity)
        }
        .toolbar { BackToolbar { app.screen = .home } }
        .alert(L10n.t(app.status(size).isReadable ? "confirm.replace" : "confirm.replaceUnreadable"),
               isPresented: $confirming) {
            Button(L10n.t("confirm.continue"), role: .destructive) { app.startNew(size: size, difficulty: prefs.soloDifficulty) }
                .accessibilityIdentifier("confirm-replace")
            Button(L10n.t("confirm.cancel"), role: .cancel) {}
        }
        .onAppear { app.refreshSlots() }
    }
}

// MARK: Settings

struct SettingsView: View {
    let app: AppModel
    @Environment(\.openURL) private var openURL

    var body: some View {
        @Bindable var prefs = app.prefs
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                PageHeader(title: L10n.t("settings.title"))
                VStack(spacing: 0) {
                    row {
                        Text(L10n.t("settings.appearance")).font(.body.weight(.medium))
                        Spacer(minLength: 12)
                        Picker(L10n.t("settings.appearance"), selection: $prefs.appearance) {
                            ForEach(AppPreferences.Appearance.allCases, id: \.self) { option in
                                Text(L10n.t("appearance.\(option.rawValue)")).tag(option)
                            }
                        }
                        .pickerStyle(.segmented)
                        .fixedSize()
                        .accessibilityIdentifier("theme")
                    }
                    Divider().overlay(Theme.line)
                    row {
                        Toggle(isOn: $prefs.sound) { labelled("settings.sound", "settings.soundNote") }
                            .tint(Theme.brand)
                            .accessibilityIdentifier("sound-toggle")
                    }
                    if Self.supportsHaptics {
                        Divider().overlay(Theme.line)
                        row {
                            Toggle(isOn: $prefs.haptics) { labelled("settings.haptics", "settings.hapticsNote") }
                                .tint(Theme.brand)
                                .accessibilityIdentifier("haptics-toggle")
                        }
                    }
                    Divider().overlay(Theme.line)
                    row {
                        labelled("settings.language", "settings.languageNote", value: L10n.t("settings.languageName"))
                        Spacer(minLength: 12)
                        Button(L10n.t("settings.openSettings")) {
                            if let url = URL(string: UIApplication.openSettingsURLString) { openURL(url) }
                        }
                        .font(.subheadline.weight(.semibold))
                        .frame(minHeight: 44)
                        .accessibilityIdentifier("language")
                    }
                }
                .padding(.horizontal, 16)
                .card(radius: 20)

                Text(L10n.t("settings.motionNote")).font(.footnote).foregroundStyle(Theme.muted)

                SectionLabel(text: L10n.t("settings.storage")).padding(.top, 14)
                Text(L10n.t("settings.storageNote")).font(.footnote).foregroundStyle(Theme.muted)
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(SoloStore.sizes, id: \.self) { size in
                        Text(storageLine(size)).font(.subheadline.monospacedDigit()).foregroundStyle(Theme.muted)
                    }
                }
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("storage-list")
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 8)
            .frame(maxWidth: 560)
            .frame(maxWidth: .infinity)
        }
        .toolbar { BackToolbar { app.closeSettings() } }
        .onChange(of: prefs.sound) { _, on in
            Feedback.shared.sound = on
            if on { Feedback.shared.play(.select) } // let the switch be heard once it is on
        }
        .onChange(of: prefs.haptics) { _, on in
            Feedback.shared.haptics = on
            if on { Feedback.shared.play(.select) }
        }
        .onAppear { app.refreshSlots() }
    }

    static var supportsHaptics: Bool { UIDevice.current.userInterfaceIdiom == .phone }

    private func row<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        HStack(alignment: .center, spacing: 12) { content() }
            .frame(maxWidth: .infinity, minHeight: 60, alignment: .leading)
            .padding(.vertical, 8)
    }

    private func labelled(_ title: String, _ note: String, value: String? = nil) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(value.map { "\(L10n.t(title)) · \($0)" } ?? L10n.t(title)).font(.body.weight(.medium))
                .foregroundStyle(Theme.foreground)
            Text(L10n.t(note)).font(.footnote).foregroundStyle(Theme.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private func storageLine(_ size: Int) -> String {
        let name = "\(size)×\(size)"
        switch app.status(size) {
        case .ready(let envelope, let complete):
            return "\(name) · \(L10n.t(complete ? "home.completed" : "home.savedGame")) · \(L10n.time(envelope.elapsedMs))"
        case .missing: return "\(name) · \(L10n.t("settings.none"))"
        case .corrupt: return L10n.t("storage.corrupt", ["size": name])
        case .unsupportedVersion: return L10n.t("storage.unsupported", ["size": name])
        case .unavailable: return L10n.t("storage.unavailable", ["size": name])
        }
    }
}
