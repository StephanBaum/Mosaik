import MosaicKit
import SwiftUI
import UIKit
import UIKit.UIGestureRecognizerSubclass

/// A screen with the app background and the standard padding, scrolling when the content is taller than the
/// screen or the text is large (Dynamic Type).
struct MosaicScreen<Content: View>: View {
    @Environment(\.colorScheme) private var colorScheme
    private let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        let colors = colorScheme.mosaic
        ZStack {
            colors.bg.ignoresSafeArea()
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
}

struct ScreenTitle: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.xl, weight: .semibold))
            .foregroundStyle(colorScheme.mosaic.text)
            .accessibilityAddTraits(.isHeader)
    }
}

struct BodyText: View {
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

/// An error line (what happened and what to do), announced to VoiceOver when it appears.
struct ErrorLine: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.sm))
            .foregroundStyle(colorScheme.mosaic.danger)
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityIdentifier("error-line")
    }
}

/// A warning line (something to check, not a failure), in the warning colour of the design tokens.
struct WarningLine: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.sm))
            .foregroundStyle(colorScheme.mosaic.warning)
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityIdentifier("warning-line")
    }
}

/// The one primary action of a screen. The foil does not render natively, so this is the static fallback
/// of the design (tokens `PrimaryAction`).
struct PrimaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        PrimaryButtonBody(configuration: configuration)
    }

    private struct PrimaryButtonBody: View {
        @Environment(\.colorScheme) private var colorScheme
        @Environment(\.isEnabled) private var isEnabled
        let configuration: ButtonStyleConfiguration

        var body: some View {
            let colors = colorScheme.mosaic
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
                .overlay(
                    RoundedRectangle(cornerRadius: MosaicTokens.Radius.control)
                        .strokeBorder(colors.primaryActionRim, lineWidth: MosaicTokens.BorderWidth.hairline)
                )
                .opacity(isEnabled ? 1 : 0.5)
                .contentShape(Rectangle())
        }
    }
}

/// A quiet action next to the primary one.
struct SecondaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        SecondaryButtonBody(configuration: configuration)
    }

    private struct SecondaryButtonBody: View {
        @Environment(\.colorScheme) private var colorScheme
        @Environment(\.isEnabled) private var isEnabled
        let configuration: ButtonStyleConfiguration

        var body: some View {
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
}

/// The label above a field.
struct FieldLabel: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.sm, weight: .medium))
            .foregroundStyle(colorScheme.mosaic.textSecondary)
    }
}

extension View {
    /// The border and fill of a text field, with the 44 pt minimum target.
    func mosaicFieldBox(colors: MosaicColors) -> some View {
        padding(.horizontal, MosaicTokens.Space.s4)
            .frame(minHeight: MosaicTokens.Size.touch)
            .background(colors.surface, in: RoundedRectangle(cornerRadius: MosaicTokens.Radius.field))
            .overlay(
                RoundedRectangle(cornerRadius: MosaicTokens.Radius.field)
                    .strokeBorder(colors.borderStrong, lineWidth: MosaicTokens.BorderWidth.hairline)
            )
    }
}

/// A plain (not secret) text field: server, email, device name, one-time code.
struct PlainField: View {
    @Environment(\.colorScheme) private var colorScheme
    let label: String
    let placeholder: String
    @Binding var text: String
    var identifier: String
    var keyboard: UIKeyboardType = .default
    var contentType: UITextContentType?

    var body: some View {
        let colors = colorScheme.mosaic
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s3) {
            FieldLabel(text: label)
            TextField(placeholder, text: $text)
                .font(Font.mosaic(MosaicTokens.FontSize.md))
                .foregroundStyle(colors.text)
                .keyboardType(keyboard)
                .textContentType(contentType)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .mosaicFieldBox(colors: colors)
                .accessibilityLabel(label)
                .accessibilityIdentifier(identifier)
        }
    }
}

/// A secret text field on a `UITextField` with secure entry, not a SwiftUI `SecureField` (spec 3.5): the
/// bytes go into a `SecretBuffer`, and iCloud Keychain does not offer to save the master password.
struct SecretField: View {
    @Environment(\.colorScheme) private var colorScheme
    let label: String
    let buffer: SecretBuffer
    let identifier: String
    /// Bumping it empties the field.
    var clearToken: Int
    /// Bumping it fills the field with what the buffer holds (a generated password).
    var fillToken = 0
    /// The value is shown as typed (the Secret Key, never while the screen is captured).
    var reveal = false
    var onChange: @MainActor (Bool) -> Void = { _ in }
    var onSubmit: @MainActor () -> Void = {}

    var body: some View {
        let colors = colorScheme.mosaic
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s3) {
            FieldLabel(text: label)
            SecretTextField(
                buffer: buffer, accessibilityText: label, identifier: identifier, clearToken: clearToken,
                fillToken: fillToken, reveal: reveal, textColor: UIColor(colors.text), onChange: onChange, onSubmit: onSubmit
            )
            .frame(minHeight: MosaicTokens.Size.touch)
            .mosaicFieldBox(colors: colors)
            .privacySensitive()
        }
    }
}

struct SecretTextField: UIViewRepresentable {
    let buffer: SecretBuffer
    let accessibilityText: String
    let identifier: String
    let clearToken: Int
    let fillToken: Int
    let reveal: Bool
    let textColor: UIColor
    let onChange: @MainActor (Bool) -> Void
    let onSubmit: @MainActor () -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    func makeUIView(context: Context) -> UITextField {
        let field = UITextField()
        field.isSecureTextEntry = !reveal
        field.autocorrectionType = .no
        field.autocapitalizationType = .none
        field.spellCheckingType = .no
        field.smartQuotesType = .no
        field.smartDashesType = .no
        field.smartInsertDeleteType = .no
        field.returnKeyType = .go
        field.clearButtonMode = .never
        field.font = UIFontMetrics(forTextStyle: .body).scaledFont(
            for: MosaicFonts.monoFont(size: MosaicTokens.FontSize.md.size))
        field.adjustsFontForContentSizeCategory = true
        field.accessibilityLabel = accessibilityText
        field.accessibilityIdentifier = identifier
        field.delegate = context.coordinator
        field.addTarget(
            context.coordinator, action: #selector(Coordinator.editingChanged(_:)), for: .editingChanged)
        field.setContentHuggingPriority(.defaultLow, for: .horizontal)
        return field
    }

    func updateUIView(_ field: UITextField, context: Context) {
        context.coordinator.parent = self
        field.textColor = textColor
        if field.isSecureTextEntry == reveal {
            field.isSecureTextEntry = !reveal
        }
        if context.coordinator.lastClearToken != clearToken {
            context.coordinator.lastClearToken = clearToken
            field.text = ""
        }
        if context.coordinator.lastFillToken != fillToken {
            context.coordinator.lastFillToken = fillToken
            var bytes = buffer.copyOut()
            field.text = String(decoding: bytes, as: UTF8.self)
            bytes.wipe()
        }
    }

    @MainActor
    final class Coordinator: NSObject, UITextFieldDelegate {
        var parent: SecretTextField
        var lastClearToken: Int
        var lastFillToken: Int

        init(parent: SecretTextField) {
            self.parent = parent
            self.lastClearToken = parent.clearToken
            self.lastFillToken = parent.fillToken
        }

        @objc func editingChanged(_ field: UITextField) {
            parent.buffer.set(from: field.text ?? "")
            parent.onChange(parent.buffer.isEmpty)
        }

        func textFieldShouldReturn(_ field: UITextField) -> Bool {
            parent.onSubmit()
            return true
        }
    }
}

/// A checkbox row as a toggle switch with its label.
struct OptionRow: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String
    @Binding var isOn: Bool
    var identifier: String

    var body: some View {
        Toggle(isOn: $isOn) {
            Text(text)
                .font(Font.mosaic(MosaicTokens.FontSize.md))
                .foregroundStyle(colorScheme.mosaic.text)
                .fixedSize(horizontal: false, vertical: true)
        }
        .tint(colorScheme.mosaic.accent)
        .frame(minHeight: MosaicTokens.Size.touch)
        .accessibilityIdentifier(identifier)
    }
}

/// A group of related controls on a surface card (settings, the generator).
struct Panel<Content: View>: View {
    @Environment(\.colorScheme) private var colorScheme
    private let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        let colors = colorScheme.mosaic
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s4) {
            content
        }
        .padding(MosaicTokens.Space.s5)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(colors.surface, in: RoundedRectangle(cornerRadius: MosaicTokens.Radius.card))
        .overlay(
            RoundedRectangle(cornerRadius: MosaicTokens.Radius.card)
                .strokeBorder(colors.border, lineWidth: MosaicTokens.BorderWidth.hairline)
        )
    }
}

/// A section heading inside a screen.
struct SectionTitle: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.lg, weight: .semibold))
            .foregroundStyle(colorScheme.mosaic.text)
            .accessibilityAddTraits(.isHeader)
    }
}

/// A small muted line (a hint, a count, a sync state).
struct HintText: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.sm))
            .foregroundStyle(colorScheme.mosaic.textMuted)
            .fixedSize(horizontal: false, vertical: true)
    }
}

/// The least prominent action: a text button in the accent color with the 44 pt target.
struct QuietButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        QuietButtonBody(configuration: configuration)
    }

    private struct QuietButtonBody: View {
        @Environment(\.colorScheme) private var colorScheme
        @Environment(\.isEnabled) private var isEnabled
        let configuration: ButtonStyleConfiguration

        var body: some View {
            configuration.label
                .font(Font.mosaic(MosaicTokens.FontSize.sm, weight: .medium))
                .foregroundStyle(colorScheme.mosaic.text)
                .frame(minWidth: MosaicTokens.Size.touchMin, minHeight: MosaicTokens.Size.touchMin)
                .opacity(isEnabled ? (configuration.isPressed ? 0.6 : 1) : 0.5)
                .contentShape(Rectangle())
        }
    }
}

/// A transient message at the bottom of a screen (copied, revealed). Announced to VoiceOver.
struct NoticeBanner: View {
    @Environment(\.colorScheme) private var colorScheme
    let text: String

    var body: some View {
        let colors = colorScheme.mosaic
        Text(text)
            .font(Font.mosaic(MosaicTokens.FontSize.sm))
            .foregroundStyle(colors.text)
            .padding(MosaicTokens.Space.s4)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(colors.bgSelected, in: RoundedRectangle(cornerRadius: MosaicTokens.Radius.field))
            .overlay(
                RoundedRectangle(cornerRadius: MosaicTokens.Radius.field)
                    .strokeBorder(colors.border, lineWidth: MosaicTokens.BorderWidth.hairline)
            )
            .accessibilityIdentifier("notice")
            .onAppear {
                UIAccessibility.post(notification: .announcement, argument: text)
            }
            .onChange(of: text) { _, newText in
                UIAccessibility.post(notification: .announcement, argument: newText)
            }
    }
}

/// Tells the idle lock about every touch without taking part in it. A SwiftUI drag gesture on the root view
/// cancels the touches of the UIKit bars (back button, tab bar, toolbar menu); this recognizer fails at once,
/// does not cancel or delay anything, and sees every touch of the window.
struct TouchProbe: UIViewRepresentable {
    let onTouch: @MainActor () -> Void

    func makeUIView(context: Context) -> TouchProbeView {
        let view = TouchProbeView()
        view.onTouch = onTouch
        view.isUserInteractionEnabled = false
        view.isAccessibilityElement = false
        return view
    }

    func updateUIView(_ uiView: TouchProbeView, context: Context) {
        uiView.onTouch = onTouch
    }
}

final class TouchProbeView: UIView {
    var onTouch: @MainActor () -> Void = {}
    private let recognizer = TouchRecognizer(target: nil, action: nil)

    override func didMoveToWindow() {
        super.didMoveToWindow()
        recognizer.view?.removeGestureRecognizer(recognizer)
        recognizer.report = { [weak self] in self?.onTouch() }
        window?.addGestureRecognizer(recognizer)
    }

    override func removeFromSuperview() {
        recognizer.view?.removeGestureRecognizer(recognizer)
        super.removeFromSuperview()
    }
}

final class TouchRecognizer: UIGestureRecognizer {
    var report: @MainActor () -> Void = {}

    override init(target: Any?, action: Selector?) {
        super.init(target: target, action: action)
        cancelsTouchesInView = false
        delaysTouchesBegan = false
        delaysTouchesEnded = false
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent) {
        report()
        state = .failed
    }
}
