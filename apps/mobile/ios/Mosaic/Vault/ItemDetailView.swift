import MosaicKit
import Observation
import SwiftUI

/// A one-time code on screen.
struct OtpDisplay: Equatable {
    let code: String
    let remaining: Int
    let period: Int
}

/// The state of one item detail: the item, the revealed values, the one-time code and the notices. Revealed
/// values are `SecretValue` handles that the lock wipes (`CoreHolder.reveals`); they hide again after
/// `revealSeconds`, in the background, while the screen is captured and on a screenshot (spec 3.13).
@MainActor
@Observable
final class ItemDetailModel {
    static let revealSeconds = 60
    private static let noticeSeconds = 4

    let vaultId: String
    let itemId: String
    private(set) var detail: ItemDetail?
    private(set) var failureCode: String?
    private(set) var notice: String?
    private(set) var revealed: [String: String] = [:]
    private(set) var otp: OtpDisplay?

    @ObservationIgnored private let app: AppModel
    @ObservationIgnored private let vault: VaultModel
    @ObservationIgnored private var secrets: [String: SecretValue] = [:]
    @ObservationIgnored private var timers: [String: Task<Void, Never>] = [:]
    @ObservationIgnored private var otpSecret: SecretValue?
    @ObservationIgnored private var noticeTask: Task<Void, Never>?

    init(app: AppModel, vault: VaultModel, vaultId: String, itemId: String) {
        self.app = app
        self.vault = vault
        self.vaultId = vaultId
        self.itemId = itemId
    }

    var isHotp: Bool { detail?.otp?.scheme == .hotp }

    // MARK: Loading

    func load() async {
        let vaultId = self.vaultId
        let itemId = self.itemId
        do {
            detail = try await vault.holder.call { core in try core.item(vaultId: vaultId, itemId: itemId) }
            failureCode = nil
        } catch {
            failureCode = ErrorText.code(of: error)
        }
    }

    func toggleFavorite() async {
        guard let current = detail else { return }
        let vaultId = self.vaultId
        let itemId = self.itemId
        let next = !current.summary.favorite
        do {
            try await vault.holder.call { core in
                try core.setFavorite(vaultId: vaultId, itemId: itemId, favorite: next)
            }
            vault.changed()
            await load()
        } catch {
            failureCode = ErrorText.code(of: error)
        }
    }

    /// Moves the item to the trash. Returns whether it worked.
    func delete() async -> Bool {
        let vaultId = self.vaultId
        let itemId = self.itemId
        do {
            try await vault.holder.call { core in try core.trashItem(vaultId: vaultId, itemId: itemId) }
            vault.changed()
            return true
        } catch {
            failureCode = ErrorText.code(of: error)
            return false
        }
    }

    // MARK: Reveal and copy

    func reveal(_ field: FieldView) async {
        guard app.capture.revealAllowed else { return }
        let vaultId = self.vaultId
        let itemId = self.itemId
        let key = field.key
        do {
            let secret = try await vault.holder.call { core in
                try core.reveal(vaultId: vaultId, itemId: itemId, field: key)
            }
            vault.holder.reveals.register(secret)
            if let old = secrets[key] {
                vault.holder.reveals.release(old)
            }
            secrets[key] = secret
            revealed[key] = Self.display(secret)
            timers[key]?.cancel()
            timers[key] = Task { [weak self] in
                try? await Task.sleep(for: .seconds(Self.revealSeconds))
                guard !Task.isCancelled else { return }
                self?.hide(key)
            }
            showNotice(
                L10n.itemSecretRevealed(field: FieldLabels.label(for: field), seconds: Self.revealSeconds))
        } catch {
            failureCode = ErrorText.code(of: error)
        }
    }

    func hide(_ key: String) {
        timers[key]?.cancel()
        timers[key] = nil
        if let secret = secrets.removeValue(forKey: key) {
            vault.holder.reveals.release(secret)
        }
        revealed[key] = nil
    }

    func hideAll() {
        for key in Array(secrets.keys) {
            hide(key)
        }
        revealed.removeAll()
    }

    func copy(_ field: FieldView) async {
        let seconds = vault.clipboardSeconds
        if !field.concealed, let value = field.value {
            var bytes = Data(value.utf8)
            PasteboardGuard.copy(bytes: &bytes, expiringAfter: seconds)
            showNotice(L10n.copyDone(seconds: seconds))
            return
        }
        let vaultId = self.vaultId
        let itemId = self.itemId
        let key = field.key
        do {
            let secret = try await vault.holder.call { core in
                try core.reveal(vaultId: vaultId, itemId: itemId, field: key)
            }
            PasteboardGuard.copy(secret, expiringAfter: seconds)
            showNotice(L10n.copyDone(seconds: seconds))
        } catch {
            failureCode = ErrorText.code(of: error)
        }
    }

    // MARK: One-time code

    /// Fetches the code and counts down; fetches the next one when the period ends. A counter code (HOTP) is
    /// fetched once. Cancelled with the screen or when the app is not in the foreground.
    func runOtp() async {
        guard detail?.otp != nil else { return }
        let vaultId = self.vaultId
        let itemId = self.itemId
        while !Task.isCancelled {
            do {
                let fetched = try await vault.holder.call { core in
                    try core.otpCode(vaultId: vaultId, itemId: itemId)
                }
                if let old = otpSecret {
                    vault.holder.reveals.release(old)
                }
                vault.holder.reveals.register(fetched.code)
                otpSecret = fetched.code
                let text = Self.display(fetched.code)
                let period = Int(fetched.periodS)
                var left = max(Int(fetched.remainingS), 1)
                otp = OtpDisplay(code: text, remaining: left, period: period)
                if isHotp { return }
                while left > 0 && !Task.isCancelled {
                    try await Task.sleep(for: .seconds(1))
                    left -= 1
                    otp = OtpDisplay(code: text, remaining: left, period: period)
                }
            } catch is CancellationError {
                return
            } catch {
                failureCode = ErrorText.code(of: error)
                return
            }
        }
    }

    func clearOtp() {
        otp = nil
        if let secret = otpSecret {
            vault.holder.reveals.release(secret)
        }
        otpSecret = nil
    }

    func copyOtp() {
        guard let secret = otpSecret else { return }
        let seconds = vault.clipboardSeconds
        var bytes = secret.exposeBytes()
        PasteboardGuard.copy(bytes: &bytes, expiringAfter: seconds)
        showNotice(L10n.copyDone(seconds: seconds))
    }

    /// The screen goes away or the app leaves the foreground: nothing revealed stays.
    func hideEverything() {
        hideAll()
        clearOtp()
    }

    // MARK: Notices

    private func showNotice(_ text: String) {
        notice = text
        noticeTask?.cancel()
        noticeTask = Task { [weak self] in
            try? await Task.sleep(for: .seconds(Self.noticeSeconds))
            guard !Task.isCancelled else { return }
            self?.notice = nil
        }
    }

    private static func display(_ secret: SecretValue) -> String {
        var bytes = secret.exposeBytes()
        defer { bytes.wipe() }
        return String(decoding: bytes, as: UTF8.self)
    }
}

/// The detail of an item: the generic fields (`FieldView`), reveal and copy, the one-time code with its ring,
/// websites, tags and passkeys. Only logins and notes can be edited (the FFI saves nothing else).
struct ItemDetailView: View {
    @Environment(\.colorScheme) private var colorScheme
    let app: AppModel
    let vault: VaultModel
    let model: ItemDetailModel
    let onEdit: (ItemKind) -> Void
    let onClose: () -> Void
    @State private var confirmDelete = false

    var body: some View {
        let colors = colorScheme.mosaic
        ZStack(alignment: .bottom) {
            colors.bg.ignoresSafeArea()
            ScrollView {
                VStack(alignment: .leading, spacing: MosaicTokens.Space.s5) {
                    content
                }
                .padding(MosaicTokens.Space.s6)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            if let notice = model.notice {
                NoticeBanner(text: notice)
                    .padding(MosaicTokens.Space.s5)
            }
        }
        .navigationTitle(title)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            if let summary = model.detail?.summary {
                ToolbarItemGroup(placement: .topBarTrailing) {
                    Button {
                        Task { await model.toggleFavorite() }
                    } label: {
                        Image(systemName: summary.favorite ? "star.fill" : "star")
                    }
                    .accessibilityLabel(summary.favorite ? L10n.detailFavoriteRemove() : L10n.detailFavoriteAdd())
                    .accessibilityIdentifier("detail-favorite")
                    if summary.kind == .login || summary.kind == .secureNote {
                        Button(L10n.commonEdit()) { onEdit(summary.kind) }
                            .accessibilityIdentifier("detail-edit")
                    }
                }
            }
        }
        .task(id: TaskKey(version: vault.version, active: app.isSceneActive)) {
            await model.load()
        }
        .task(id: OtpKey(has: model.detail?.otp != nil, active: app.isSceneActive)) {
            if app.isSceneActive {
                await model.runOtp()
            } else {
                model.clearOtp()
            }
        }
        .onChange(of: app.isSceneActive) { _, active in
            if !active { model.hideEverything() }
        }
        .onChange(of: app.capture.isCaptured) { _, captured in
            if captured { model.hideEverything() }
        }
        .onChange(of: app.capture.screenshotCount) { _, _ in model.hideEverything() }
        .onDisappear { model.hideEverything() }
        .alert(
            L10n.detailDeleteTitle(title: model.detail.map { $0.summary.title.isEmpty ? L10n.vaultUntitled() : $0.summary.title } ?? L10n.vaultUntitled()),
            isPresented: $confirmDelete) {
            Button(L10n.detailDelete(), role: .destructive) {
                Task {
                    if await model.delete() { onClose() }
                }
            }
            Button(L10n.commonCancel(), role: .cancel) {}
        } message: {
            Text(L10n.detailDeleteBody())
        }
    }

    private var title: String {
        guard let summary = model.detail?.summary else { return "" }
        return summary.title.isEmpty ? L10n.vaultUntitled() : summary.title
    }

    @ViewBuilder
    private var content: some View {
        if let code = model.failureCode {
            ErrorLine(text: ErrorText.message(forCode: code))
        }
        if let detail = model.detail {
            HintText(text: FieldLabels.kind(detail.summary.kind))
            if detail.otp != nil {
                OtpBlock(model: model)
            }
            ForEach(detail.fields, id: \.key) { field in
                FieldRow(model: model, capture: app.capture, field: field)
            }
            if !detail.urls.isEmpty {
                SectionTitle(text: L10n.itemUrls(count: detail.urls.count))
                ForEach(Array(detail.urls.enumerated()), id: \.offset) { _, url in
                    if url.isApp {
                        BodyText(text: L10n.detailLinkedApp(name: UrlNormalizer.appName(url.url)))
                        if url.needsLinking {
                            HintText(text: L10n.detailNeedsLinking())
                        }
                    } else {
                        BodyText(text: url.url)
                            .accessibilityLabel(L10n.detailLabelValue(label: L10n.editorWebsites(), value: url.url))
                    }
                }
            }
            if !detail.tags.isEmpty {
                SectionTitle(text: L10n.itemTags())
                BodyText(text: detail.tags.joined(separator: ", "))
            }
            ForEach(Array(detail.passkeys.enumerated()), id: \.offset) { _, passkey in
                BodyText(text: L10n.detailPasskey(site: passkey.rpId))
                HintText(text: passkey.userName)
            }
            if !detail.attachments.isEmpty {
                HintText(text: L10n.detailAttachments(count: detail.attachments.count))
            }
            Button(L10n.detailDelete()) { confirmDelete = true }
                .buttonStyle(QuietButtonStyle())
                .accessibilityIdentifier("detail-delete")
        } else if model.failureCode == nil {
            HintText(text: L10n.commonWorking())
        }
    }
}

private struct TaskKey: Equatable {
    let version: Int
    let active: Bool
}

private struct OtpKey: Equatable {
    let has: Bool
    let active: Bool
}

/// One field of the generic detail: its label, the value or dots, reveal and copy.
private struct FieldRow: View {
    @Environment(\.colorScheme) private var colorScheme
    let model: ItemDetailModel
    let capture: CaptureObserver
    let field: FieldView

    var body: some View {
        let colors = colorScheme.mosaic
        let label = FieldLabels.label(for: field)
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s2) {
            HintText(text: label)
                .accessibilityHidden(!field.concealed)
            HStack(spacing: MosaicTokens.Space.s3) {
                valueText(colors: colors)
                    .frame(maxWidth: .infinity, alignment: .leading)
                if field.concealed {
                    if model.revealed[field.key] != nil {
                        Button(L10n.commonHide()) { model.hide(field.key) }
                            .buttonStyle(QuietButtonStyle())
                            .accessibilityLabel(L10n.itemSecretHideAction(field: label))
                    } else if capture.revealAllowed {
                        Button(L10n.commonShow()) { Task { await model.reveal(field) } }
                            .buttonStyle(QuietButtonStyle())
                            .accessibilityLabel(L10n.itemSecretRevealAction(field: label))
                            .accessibilityIdentifier("reveal-\(field.key)")
                    }
                }
                if field.copyable {
                    Button(L10n.commonCopy()) { Task { await model.copy(field) } }
                        .buttonStyle(QuietButtonStyle())
                        .accessibilityLabel(L10n.itemCopy(field: label))
                        .accessibilityIdentifier("copy-\(field.key)")
                }
            }
            if field.concealed, let note = capture.hiddenNote {
                HintText(text: note)
            }
        }
    }

    @ViewBuilder
    private func valueText(colors: MosaicColors) -> some View {
        if field.concealed {
            if let shown = model.revealed[field.key], capture.revealAllowed {
                Text(shown)
                    .font(Font.mosaicMono(MosaicTokens.FontSize.md))
                    .foregroundStyle(colors.text)
                    .textSelection(.disabled)
                    .privacySensitive()
                    .accessibilityIdentifier("value-\(field.key)")
            } else {
                Text("••••••••")
                    .font(Font.mosaicMono(MosaicTokens.FontSize.md))
                    .foregroundStyle(colors.text)
                    .accessibilityHidden(true)
            }
        } else {
            Text(field.value ?? "")
                .font(Font.mosaic(MosaicTokens.FontSize.md))
                .foregroundStyle(colors.text)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityLabel(L10n.detailLabelValue(label: FieldLabels.label(for: field), value: field.value ?? ""))
        }
    }
}

/// The one-time code with its countdown ring. VoiceOver reads neither the code nor the time until it is copied.
private struct OtpBlock: View {
    @Environment(\.colorScheme) private var colorScheme
    let model: ItemDetailModel

    var body: some View {
        let colors = colorScheme.mosaic
        let label = L10n.itemKindTotp()
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s2) {
            HintText(text: label)
            HStack(spacing: MosaicTokens.Space.s4) {
                HStack(spacing: MosaicTokens.Space.s4) {
                    if let otp = model.otp {
                        CountdownRing(fraction: OtpFormat.fraction(remaining: otp.remaining, period: otp.period))
                        Text(OtpFormat.group(otp.code))
                            .font(Font.mosaicMono(MosaicTokens.FontSize.xl))
                            .foregroundStyle(colors.text)
                            .privacySensitive()
                            .accessibilityIdentifier("otp-code")
                    } else {
                        Text("••• •••")
                            .font(Font.mosaicMono(MosaicTokens.FontSize.xl))
                            .foregroundStyle(colors.text)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .accessibilityElement(children: .ignore)
                .accessibilityLabel(label)
                .accessibilityAction(named: Text(L10n.detailReadCode())) {
                    // Only on the user's explicit request does the code reach VoiceOver, digit by digit.
                    if let otp = model.otp {
                        UIAccessibility.post(notification: .announcement, argument: otp.code.map(String.init).joined(separator: " "))
                    }
                }
                Button(L10n.commonCopy()) { model.copyOtp() }
                    .buttonStyle(QuietButtonStyle())
                    .disabled(model.otp == nil)
                    .accessibilityLabel(L10n.itemCopy(field: label))
                    .accessibilityIdentifier("copy-otp")
            }
            if let otp = model.otp {
                HintText(text: L10n.detailOtpLeft(seconds: otp.remaining))
            }
        }
    }
}

/// The ring that empties as the code runs out.
struct CountdownRing: View {
    @Environment(\.colorScheme) private var colorScheme
    let fraction: Double

    var body: some View {
        let colors = colorScheme.mosaic
        ZStack {
            Circle().stroke(colors.border, lineWidth: MosaicTokens.Space.s2)
            Circle()
                .trim(from: 0, to: CGFloat(fraction))
                .stroke(colors.accent, style: StrokeStyle(lineWidth: MosaicTokens.Space.s2, lineCap: .round))
                .rotationEffect(.degrees(-90))
        }
        .frame(width: MosaicTokens.Space.s7, height: MosaicTokens.Space.s7)
        .accessibilityHidden(true)
    }
}
