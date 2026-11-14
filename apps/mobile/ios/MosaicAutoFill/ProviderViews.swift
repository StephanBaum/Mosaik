import MosaicKit
import SwiftUI

/// The one screen of the extension. Every string comes from `L10n` (the shared catalog) and every color,
/// size and font from the design tokens. The extension target cannot use the app's components, so the few it
/// needs are small copies here.
struct ProviderRootView: View {
    @Environment(\.colorScheme) private var colorScheme
    @Bindable var model: ProviderModel

    var body: some View {
        let colors = colorScheme.mosaic
        ZStack {
            colors.bg.ignoresSafeArea()
            VStack(spacing: 0) {
                header(colors)
                Rectangle().fill(colors.divider).frame(height: MosaicTokens.BorderWidth.hairline)
                content
            }
        }
    }

    private func header(_ colors: MosaicColors) -> some View {
        HStack {
            Text(L10n.appName())
                .font(Font.mosaic(MosaicTokens.FontSize.lg, weight: .semibold))
                .foregroundStyle(colors.text)
                .accessibilityAddTraits(.isHeader)
            Spacer()
            Button(isConfiguration ? L10n.commonDone() : L10n.commonCancel()) { model.cancel() }
                .font(Font.mosaic(MosaicTokens.FontSize.md, weight: .medium))
                .foregroundStyle(colors.text)
                .frame(minWidth: MosaicTokens.Size.touchMin, minHeight: MosaicTokens.Size.touchMin)
                .accessibilityIdentifier("provider-cancel")
        }
        .padding(.horizontal, MosaicTokens.Space.s6)
        .padding(.vertical, MosaicTokens.Space.s3)
    }

    private var isConfiguration: Bool {
        if case .configuration = model.screen { return true }
        return false
    }

    @ViewBuilder
    private var content: some View {
        switch model.screen {
        case .loading:
            ProviderMessage(text: L10n.extLoading(), showsProgress: true, onRetry: nil)
        case let .message(text, retry):
            if retry {
                ProviderMessage(text: text, showsProgress: false, onRetry: { Task { await model.retry() } })
            } else {
                ProviderMessage(text: text, showsProgress: false, onRetry: nil)
            }
        case .unlock:
            UnlockPanel(model: model)
        case let .confirmFill(site):
            ConfirmFillPanel(model: model, site: site)
        case .logins:
            LoginsPanel(model: model)
        case .passkeys:
            PasskeysPanel(model: model)
        case .confirmRegistration:
            ConfirmPanel(model: model)
        case .configuration:
            ProviderScroll {
                ProviderTitle(text: L10n.extConfigTitle())
                ProviderBody(text: L10n.extConfigBody())
            }
        }
    }
}

// MARK: Building blocks

private struct ProviderScroll<Content: View>: View {
    private let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: MosaicTokens.Space.s5) {
                content
            }
            .padding(MosaicTokens.Space.s6)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .scrollBounceBehavior(.basedOnSize)
    }
}

private struct ProviderTitle: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.xl, weight: .semibold))
            .foregroundStyle(colorScheme.mosaic.text)
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityAddTraits(.isHeader)
    }
}

private struct ProviderBody: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String
    var muted = false

    var body: some View {
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.md))
            .foregroundStyle(muted ? colorScheme.mosaic.textMuted : colorScheme.mosaic.textSecondary)
            .fixedSize(horizontal: false, vertical: true)
    }
}

private struct ProviderError: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.sm))
            .foregroundStyle(colorScheme.mosaic.danger)
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityIdentifier("provider-error")
    }
}

/// A warning about the address, with the border and fill of the warning tokens.
private struct ProviderWarning: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        let colors = colorScheme.mosaic
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.sm))
            .foregroundStyle(colors.text)
            .fixedSize(horizontal: false, vertical: true)
            .padding(MosaicTokens.Space.s4)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(colors.warningSubtle, in: RoundedRectangle(cornerRadius: MosaicTokens.Radius.field))
            .overlay(
                RoundedRectangle(cornerRadius: MosaicTokens.Radius.field)
                    .strokeBorder(colors.warningBorder, lineWidth: MosaicTokens.BorderWidth.hairline)
            )
            .accessibilityIdentifier("provider-warning")
    }
}

/// The one primary action of a screen (the static fallback of the design; the foil does not render natively).
private struct ProviderPrimaryStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(Font.mosaic(MosaicTokens.FontSize.md, weight: .semibold))
            .foregroundStyle(MosaicTokens.PrimaryAction.label)
            .frame(maxWidth: .infinity, minHeight: MosaicTokens.Size.touch)
            .padding(.horizontal, MosaicTokens.Space.s5)
            .background(
                configuration.isPressed
                    ? MosaicTokens.PrimaryAction.fallbackHighlight : MosaicTokens.PrimaryAction.fallback,
                in: RoundedRectangle(cornerRadius: MosaicTokens.Radius.control)
            )
            .opacity(isEnabled ? 1 : 0.5)
            .contentShape(Rectangle())
    }
}

private struct ProviderQuietStyle: ButtonStyle {
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        let colors = colorScheme.mosaic
        configuration.label
            .font(Font.mosaic(MosaicTokens.FontSize.md, weight: .medium))
            .foregroundStyle(colors.text)
            .frame(maxWidth: .infinity, minHeight: MosaicTokens.Size.touch)
            .padding(.horizontal, MosaicTokens.Space.s5)
            .background(
                configuration.isPressed ? colors.bgSelected : Color.clear,
                in: RoundedRectangle(cornerRadius: MosaicTokens.Radius.control)
            )
            .overlay(
                RoundedRectangle(cornerRadius: MosaicTokens.Radius.control)
                    .strokeBorder(colors.border, lineWidth: MosaicTokens.BorderWidth.hairline)
            )
            .opacity(isEnabled ? 1 : 0.5)
            .contentShape(Rectangle())
    }
}

private struct ProviderMessage: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String
    let showsProgress: Bool
    let onRetry: (() -> Void)?

    var body: some View {
        ProviderScroll {
            if showsProgress {
                ProgressView()
                    .tint(colorScheme.mosaic.textMuted)
            }
            ProviderBody(text: text)
                .accessibilityIdentifier("provider-message")
            if let onRetry {
                Button(L10n.commonTryAgain(), action: onRetry)
                    .buttonStyle(ProviderQuietStyle())
                    .accessibilityIdentifier("provider-retry")
            }
        }
    }
}

// MARK: Panels

private struct UnlockPanel: View {
    let model: ProviderModel

    var body: some View {
        ProviderScroll {
            ProviderTitle(text: L10n.unlockTitle())
                .accessibilityIdentifier("provider-unlock-title")
            ProviderBody(text: L10n.credentialVerifyTitle(), muted: true)
            if let message = model.errorText {
                ProviderError(text: message)
            }
            Button(L10n.unlockQuick()) { Task { await model.unlock() } }
                .buttonStyle(ProviderPrimaryStyle())
                .disabled(model.isWorking)
                .accessibilityIdentifier("provider-unlock")
        }
    }
}

/// A tap on a QuickType suggestion: names the saved site and asks before anything unlocks or fills.
private struct ConfirmFillPanel: View {
    let model: ProviderModel
    let site: String

    var body: some View {
        ProviderScroll {
            ProviderTitle(text: L10n.extConfirmFillTitle(site: site))
                .accessibilityIdentifier("provider-confirm-fill-title")
            ProviderBody(text: L10n.extConfirmFillBody(), muted: true)
            Button(L10n.extConfirmFillButton()) { Task { await model.confirmFill() } }
                .buttonStyle(ProviderPrimaryStyle())
                .accessibilityIdentifier("provider-confirm-fill")
        }
    }
}

/// A row that fills when tapped, or (without an action) a read-only row.
private struct LoginRow: View {
    @Environment(\.colorScheme) private var colorScheme
    let title: String
    let detail: String?
    let caption: String?
    let action: (() -> Void)?

    var body: some View {
        if let action {
            Button(action: action) { rowBody }
                .buttonStyle(.plain)
                .accessibilityHint(L10n.extFillHint())
        } else {
            rowBody
                .opacity(0.7)
                .accessibilityElement(children: .combine)
        }
    }

    private var rowBody: some View {
        let colors = colorScheme.mosaic
        return HStack(spacing: MosaicTokens.Space.s4) {
            VStack(alignment: .leading, spacing: MosaicTokens.Space.s1) {
                Text(title)
                    .font(Font.mosaic(MosaicTokens.FontSize.md, weight: .medium))
                    .foregroundStyle(colors.text)
                    .lineLimit(2)
                Text(detail ?? L10n.autofillNoUsername())
                    .font(Font.mosaic(MosaicTokens.FontSize.sm))
                    .foregroundStyle(colors.textMuted)
                    .lineLimit(1)
                if let caption {
                    Text(caption)
                        .font(Font.mosaic(MosaicTokens.FontSize.xs))
                        .foregroundStyle(colors.textMuted)
                }
            }
            Spacer(minLength: 0)
        }
        .padding(.vertical, MosaicTokens.Space.s4)
        .frame(maxWidth: .infinity, minHeight: MosaicTokens.Size.inputXl, alignment: .leading)
        .contentShape(Rectangle())
    }
}

private struct LoginsPanel: View {
    @Environment(\.colorScheme) private var colorScheme
    @Bindable var model: ProviderModel

    var body: some View {
        let colors = colorScheme.mosaic
        let site = model.list?.site ?? L10n.extThisSite()
        ProviderScroll {
            ProviderTitle(text: model.isOneTimeCode ? L10n.extOtpTitle() : L10n.extMatches(site: site))
            if let warning = model.list?.warning {
                ProviderWarning(text: warningText(warning))
            }
            if let code = model.list?.refusalCode {
                ProviderError(text: ErrorText.message(forCode: code))
            }
            if let message = model.errorText {
                ProviderError(text: message)
            }
            if model.visibleMatches.isEmpty {
                ProviderBody(text: L10n.extNoMatches(site: site))
                    .accessibilityAddTraits(.isHeader)
                ProviderBody(text: L10n.extNoMatchesHint(), muted: true)
            }
            VStack(spacing: 0) {
                ForEach(model.visibleMatches, id: \.key) { candidate in
                    LoginRow(
                        title: candidate.title, detail: candidate.username, caption: nil,
                        action: { Task { await model.fill(candidate.key) } })
                    Rectangle().fill(colors.divider).frame(height: MosaicTokens.BorderWidth.hairline)
                }
            }
            Button(model.showAll ? L10n.extHideAll() : L10n.extSearchAll()) { model.showAll.toggle() }
                .buttonStyle(ProviderQuietStyle())
                .accessibilityIdentifier("provider-search-all")
            if model.showAll {
                TextField(L10n.searchPlaceholder(), text: $model.query)
                    .font(Font.mosaic(MosaicTokens.FontSize.md))
                    .foregroundStyle(colors.text)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .padding(.horizontal, MosaicTokens.Space.s4)
                    .frame(minHeight: MosaicTokens.Size.touch)
                    .background(colors.surface, in: RoundedRectangle(cornerRadius: MosaicTokens.Radius.field))
                    .overlay(
                        RoundedRectangle(cornerRadius: MosaicTokens.Radius.field)
                            .strokeBorder(colors.borderStrong, lineWidth: MosaicTokens.BorderWidth.hairline)
                    )
                    .accessibilityLabel(L10n.searchPlaceholder())
                VStack(spacing: 0) {
                    ForEach(model.visibleOthers, id: \.itemId) { item in
                        LoginRow(
                            title: item.title, detail: item.subtitle, caption: L10n.extNotSavedFor(site: site),
                            action: nil)
                        Rectangle().fill(colors.divider).frame(height: MosaicTokens.BorderWidth.hairline)
                    }
                }
            }
        }
    }

    private func warningText(_ warning: OriginWarning) -> String {
        switch warning {
        case let .punycode(unicode):
            return L10n.extWarnPunycode(unicode: unicode)
        case let .lookalike(savedHost):
            return L10n.extWarnLookalike(site: savedHost)
        }
    }
}

private struct PasskeysPanel: View {
    @Environment(\.colorScheme) private var colorScheme
    @Bindable var model: ProviderModel

    var body: some View {
        let colors = colorScheme.mosaic
        ProviderScroll {
            ProviderTitle(text: L10n.extPasskeyTitle())
            if let message = model.errorText {
                ProviderError(text: message)
            }
            if model.passkeys.isEmpty {
                ProviderBody(text: L10n.errorPasskeyNotFound())
            }
            VStack(spacing: 0) {
                ForEach(model.passkeys, id: \.key) { passkey in
                    LoginRow(
                        title: passkey.rpId, detail: passkey.displayName.isEmpty ? passkey.userName : passkey.displayName,
                        caption: nil, action: { Task { await model.choose(passkey: passkey) } })
                    Rectangle().fill(colors.divider).frame(height: MosaicTokens.BorderWidth.hairline)
                }
            }
        }
    }
}

private struct ConfirmPanel: View {
    @Bindable var model: ProviderModel

    var body: some View {
        ProviderScroll {
            ProviderTitle(text: L10n.credentialCreateTitle())
            if case let .passkeyRegister(register) = model.request {
                ProviderBody(text: L10n.credentialCreateBody(site: register.rpId, user: register.userName))
            }
            if model.vaults.count > 1 {
                Picker(L10n.credentialCreateVault(), selection: $model.selectedVault) {
                    ForEach(model.vaults, id: \.vaultId) { vault in
                        Text(vault.name ?? L10n.credentialVaultDefault()).tag(Optional(vault.vaultId))
                    }
                }
                .pickerStyle(.menu)
            }
            if let message = model.errorText {
                ProviderError(text: message)
            }
            Button(L10n.credentialCreateConfirm()) { Task { await model.confirmRegistration() } }
                .buttonStyle(ProviderPrimaryStyle())
                .disabled(!model.isRegistrationConfirmable)
                .accessibilityIdentifier("provider-confirm")
        }
    }
}
