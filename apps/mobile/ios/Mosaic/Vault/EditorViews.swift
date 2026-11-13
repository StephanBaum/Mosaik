import MosaicKit
import Observation
import SwiftUI

/// One website of the login editor.
struct UrlEntry: Identifiable, Equatable {
    let id = UUID()
    var url: String
    var matchEtld1: Bool
    var allowHttp: Bool
    var isApp: Bool
}

/// The login editor: title, user name, password with the generator, websites, notes and a one-time code.
/// Only the item types the FFI can save (logins and notes) have an editor.
@MainActor
@Observable
final class LoginEditorModel {
    let itemId: String?
    var title = ""
    var username = ""
    var favorite = false
    var notes = ""
    var urls: [UrlEntry] = []
    var showPassword = false
    var showGenerator = false
    private(set) var scanning = false
    private(set) var scanModel: ScanModel?
    private(set) var loaded: Bool
    private(set) var busy = false
    private(set) var errorCode: String?
    private(set) var hasOtp = false
    private(set) var otpRemoved = false
    private(set) var pendingId: String?
    private(set) var pendingName = ""
    var passwordClearToken = 0
    var passwordFillToken = 0
    var passwordFilled = false
    var otpClearToken = 0
    var otpFilled = false

    let password = SecretBuffer()
    let otpText = SecretBuffer()
    let generator = GeneratorModel()

    @ObservationIgnored private var vaultId: String?
    @ObservationIgnored private var hadNotes = false
    @ObservationIgnored private var saved = false
    @ObservationIgnored private let app: AppModel
    @ObservationIgnored private let vault: VaultModel

    init(app: AppModel, vault: VaultModel, vaultId: String?, itemId: String?) {
        self.app = app
        self.vault = vault
        self.vaultId = vaultId
        self.itemId = itemId
        self.loaded = itemId == nil
    }

    var isNew: Bool { itemId == nil }

    func load() async {
        if let itemId, let vaultId {
            do {
                let detail = try await vault.holder.call { core in try core.item(vaultId: vaultId, itemId: itemId) }
                title = detail.summary.title
                username = detail.fields.first(where: { $0.key == "login.username" })?.value ?? ""
                favorite = detail.summary.favorite
                let existing = detail.fields.first(where: { $0.key == "notes" })?.value ?? ""
                hadNotes = !existing.isEmpty
                notes = existing
                hasOtp = detail.otp != nil
                urls = detail.urls.map {
                    UrlEntry(url: $0.url, matchEtld1: $0.matchEtld1, allowHttp: $0.allowHttp, isApp: $0.isApp)
                }
                loaded = true
            } catch {
                errorCode = ErrorText.code(of: error)
            }
        } else if vaultId == nil {
            do {
                vaultId = try await vault.firstVaultId()
                if vaultId == nil {
                    errorCode = "vaultUnavailable"
                }
            } catch {
                errorCode = ErrorText.code(of: error)
            }
        }
    }

    // MARK: Generator

    /// Puts the generated password into the password field.
    func useGenerated() {
        guard var bytes = generator.take() else { return }
        password.replace(taking: &bytes)
        passwordFilled = !password.isEmpty
        passwordFillToken += 1
        showGenerator = false
    }

    // MARK: One-time code

    func addOtpFromText() async {
        guard !otpText.isEmpty else { return }
        let text = otpText.copyOut()
        busy = true
        defer { busy = false }
        do {
            let preview = try await vault.holder.call { core in try core.otpFromText(text: text) }
            takePreview(preview)
            otpText.wipe()
            otpFilled = false
            otpClearToken += 1
        } catch {
            errorCode = ErrorText.code(of: error)
        }
    }

    func startScanning() {
        errorCode = nil
        scanModel = ScanModel(
            target: .otp, holder: app.holder,
            onOtp: { [weak self] preview in
                self?.takePreview(preview)
                self?.stopScanning()
            })
        scanning = true
    }

    func stopScanning() {
        scanModel?.stop()
        scanModel = nil
        scanning = false
    }

    private func takePreview(_ preview: OtpPreview) {
        discardPending()
        pendingId = preview.pendingId
        pendingName = preview.issuer ?? preview.accountName ?? ""
        otpRemoved = false
        errorCode = nil
    }

    func removeOtp() {
        otpRemoved = true
    }

    /// Drops a pending setup that was never saved (the core keeps at most four, for ten minutes).
    func discardPending() {
        guard let id = pendingId else { return }
        pendingId = nil
        let holder = vault.holder
        Task { _ = try? await holder.call { core in core.otpPendingDiscard(pendingId: id) } }
    }

    /// The editor closes without saving.
    func leave() {
        stopScanning()
        generator.clear()
        password.wipe()
        otpText.wipe()
        if !saved {
            discardPending()
        }
    }

    // MARK: Save

    var canSave: Bool { loaded && !busy && vaultId != nil }

    func save() async -> ItemKey? {
        guard let vaultId else {
            errorCode = "vaultUnavailable"
            return nil
        }
        busy = true
        errorCode = nil
        defer { busy = false }
        let otp: OtpEdit
        if let pending = pendingId {
            otp = .setPending(id: pending)
        } else if otpRemoved {
            otp = .remove
        } else {
            otp = .keep
        }
        let drafts = urls.filter { $0.isApp || !$0.url.trimmingCharacters(in: .whitespaces).isEmpty }
            .map {
                UrlDraft(
                    url: $0.isApp ? $0.url : UrlNormalizer.normalize($0.url), matchEtld1: $0.matchEtld1,
                    allowHttp: $0.allowHttp)
            }
        let itemId = self.itemId
        let title = self.title.trimmingCharacters(in: .whitespaces)
        let username = self.username.trimmingCharacters(in: .whitespaces)
        let favorite = self.favorite
        let notesText = notes
        let hadNotes = self.hadNotes
        let passwordBuffer = password
        do {
            let key = try await vault.holder.call { core in
                var pw: Data? = passwordBuffer.isEmpty ? nil : passwordBuffer.copyOut()
                defer { pw?.wipe() }
                var noteBytes: Data? = notesText.isEmpty ? (hadNotes ? Data() : nil) : Data(notesText.utf8)
                defer { noteBytes?.wipe() }
                let key: ItemKey
                do {
                    let draft = LoginDraft(
                        vaultId: vaultId, itemId: itemId, title: title, username: username, password: pw,
                        urls: drafts, notes: noteBytes, otp: otp, favorite: favorite)
                    key = try core.saveLogin(draft: draft)
                }
                return key
            }
            saved = true
            pendingId = nil
            password.wipe()
            return key
        } catch {
            errorCode = ErrorText.code(of: error)
            return nil
        }
    }
}

/// The note editor: title, the note text and a favorite mark.
@MainActor
@Observable
final class NoteEditorModel {
    let itemId: String?
    var title = ""
    var notes = ""
    var favorite = false
    private(set) var loaded: Bool
    private(set) var busy = false
    private(set) var errorCode: String?

    @ObservationIgnored private var vaultId: String?
    @ObservationIgnored private var hadNotes = false
    @ObservationIgnored private let vault: VaultModel

    init(vault: VaultModel, vaultId: String?, itemId: String?) {
        self.vault = vault
        self.vaultId = vaultId
        self.itemId = itemId
        self.loaded = itemId == nil
    }

    var isNew: Bool { itemId == nil }
    var canSave: Bool { loaded && !busy && vaultId != nil }

    func load() async {
        if let itemId, let vaultId {
            do {
                let detail = try await vault.holder.call { core in try core.item(vaultId: vaultId, itemId: itemId) }
                title = detail.summary.title
                favorite = detail.summary.favorite
                let existing = detail.fields.first(where: { $0.key == "notes" })?.value ?? ""
                hadNotes = !existing.isEmpty
                notes = existing
                loaded = true
            } catch {
                errorCode = ErrorText.code(of: error)
            }
        } else if vaultId == nil {
            do {
                vaultId = try await vault.firstVaultId()
                if vaultId == nil {
                    errorCode = "vaultUnavailable"
                }
            } catch {
                errorCode = ErrorText.code(of: error)
            }
        }
    }

    func save() async -> ItemKey? {
        guard let vaultId else {
            errorCode = "vaultUnavailable"
            return nil
        }
        busy = true
        errorCode = nil
        defer { busy = false }
        let itemId = self.itemId
        let title = self.title.trimmingCharacters(in: .whitespaces)
        let favorite = self.favorite
        let notesText = notes
        let hadNotes = self.hadNotes
        do {
            return try await vault.holder.call { core in
                var noteBytes: Data? = notesText.isEmpty ? (hadNotes ? Data() : nil) : Data(notesText.utf8)
                defer { noteBytes?.wipe() }
                let key: ItemKey
                do {
                    let draft = NoteDraft(
                        vaultId: vaultId, itemId: itemId, title: title, notes: noteBytes, favorite: favorite)
                    key = try core.saveNote(draft: draft)
                }
                return key
            }
        } catch {
            errorCode = ErrorText.code(of: error)
            return nil
        }
    }
}

/// The password generator of the editor. The generated value stays in the core's `SecretValue` handle; the
/// preview text is dropped when the panel closes.
@MainActor
@Observable
final class GeneratorModel {
    var passphrase = false
    var length = 20.0
    var lowercase = true
    var uppercase = true
    var digits = true
    var symbols = true
    var excludeAmbiguous = false
    var words = 5.0
    var separator = "-"
    var capitalize = true
    var appendDigit = false
    private(set) var preview: String?
    private(set) var bits = 0
    private(set) var errorCode: String?
    @ObservationIgnored private var secret: SecretValue?

    var options: GeneratorOptions {
        GeneratorOptions(
            mode: passphrase ? .passphrase : .password, length: UInt32(length), lowercase: lowercase,
            uppercase: uppercase, digits: digits, symbols: symbols, excludeAmbiguous: excludeAmbiguous,
            words: UInt32(words), separator: separator, capitalize: capitalize, appendDigit: appendDigit)
    }

    func generate(holder: CoreHolder) async {
        let options = self.options
        do {
            let generated = try await holder.call { core in try core.generatePassword(options: options) }
            replace(generated.value)
            var bytes = generated.value.exposeBytes()
            preview = String(decoding: bytes, as: UTF8.self)
            bytes.wipe()
            bits = Int(generated.entropyBits.rounded())
            errorCode = nil
        } catch {
            errorCode = ErrorText.code(of: error)
        }
    }

    /// The generated bytes, for the password field. The handle is wiped.
    func take() -> Data? {
        guard let secret else { return nil }
        let bytes = secret.exposeBytes()
        clear()
        return bytes
    }

    func clear() {
        replace(nil)
        preview = nil
    }

    private func replace(_ next: SecretValue?) {
        secret?.wipe()
        secret = next
    }
}

// MARK: Views

/// Shared look of the editor screens.
private struct EditorScreen<Content: View>: View {
    @Environment(\.colorScheme) private var colorScheme
    private let content: Content

    init(@ViewBuilder content: () -> Content) {
        self.content = content()
    }

    var body: some View {
        ZStack {
            colorScheme.mosaic.bg.ignoresSafeArea()
            ScrollView {
                VStack(alignment: .leading, spacing: MosaicTokens.Space.s5) {
                    content
                }
                .padding(MosaicTokens.Space.s6)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .scrollDismissesKeyboard(.interactively)
        }
    }
}

/// A multi-line text box for notes.
private struct NotesField: View {
    @Environment(\.colorScheme) private var colorScheme
    @Binding var text: String
    let label: String
    let identifier: String

    var body: some View {
        let colors = colorScheme.mosaic
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s3) {
            FieldLabel(text: label)
            TextEditor(text: $text)
                .font(Font.mosaic(MosaicTokens.FontSize.md))
                .foregroundStyle(colors.text)
                .scrollContentBackground(.hidden)
                .frame(minHeight: 120)
                .mosaicFieldBox(colors: colors)
                .privacySensitive()
                .accessibilityLabel(label)
                .accessibilityIdentifier(identifier)
        }
    }
}

struct LoginEditorView: View {
    @Bindable var model: LoginEditorModel
    let app: AppModel
    let vault: VaultModel
    let onSaved: (ItemKey) -> Void

    var body: some View {
        EditorScreen {
            if !model.loaded {
                if let code = model.errorCode {
                    ErrorLine(text: ErrorText.message(forCode: code))
                } else {
                    HintText(text: L10n.commonWorking())
                }
            } else {
                form
            }
        }
        .navigationTitle(model.isNew ? L10n.editorNewLogin() : L10n.editorEditLogin())
        .navigationBarTitleDisplayMode(.inline)
        .task { await model.load() }
        .onDisappear { model.leave() }
        .onChange(of: app.isSceneActive) { _, active in
            if !active {
                model.generator.clear()
                model.showPassword = false
            }
        }
    }

    @ViewBuilder
    private var form: some View {
        PlainField(
            label: L10n.editorTitle(), placeholder: "", text: $model.title, identifier: "editor-title")
        PlainField(
            label: L10n.editorUsername(), placeholder: "", text: $model.username, identifier: "editor-username",
            keyboard: .emailAddress, contentType: .username)
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s3) {
            SecretField(
                label: L10n.editorPassword(), buffer: model.password, identifier: "editor-password",
                clearToken: model.passwordClearToken, fillToken: model.passwordFillToken,
                reveal: model.showPassword && app.capture.revealAllowed,
                onChange: { empty in model.passwordFilled = !empty })
            if let note = app.capture.hiddenNote {
                HintText(text: note)
            } else {
                Toggle(isOn: $model.showPassword) {
                    Text(L10n.itemSecretRevealAction(field: L10n.editorPassword()))
                        .font(Font.mosaic(MosaicTokens.FontSize.sm))
                }
                .accessibilityIdentifier("editor-password-reveal")
            }
            if !model.isNew {
                HintText(text: L10n.editorPasswordKeep())
            }
        }
        Button(L10n.editorGenerate()) { model.showGenerator.toggle() }
            .buttonStyle(SecondaryButtonStyle())
            .accessibilityIdentifier("editor-generate")
        if model.showGenerator {
            GeneratorPanel(model: model.generator, holder: vault.holder, onUse: { model.useGenerated() })
        }
        SectionTitle(text: L10n.editorWebsites())
        ForEach($model.urls) { $entry in
            HStack(spacing: MosaicTokens.Space.s3) {
                if entry.isApp {
                    BodyText(text: L10n.detailLinkedApp(name: UrlNormalizer.appName(entry.url)))
                        .frame(maxWidth: .infinity, alignment: .leading)
                } else {
                    PlainField(
                        label: L10n.editorWebsiteHint(), placeholder: L10n.editorWebsiteHint(), text: $entry.url,
                        identifier: "editor-website", keyboard: .URL, contentType: .URL)
                }
                Button(L10n.commonRemove()) {
                    let removed = entry.id
                    model.urls.removeAll { $0.id == removed }
                }
                .buttonStyle(QuietButtonStyle())
                .accessibilityLabel(
                    L10n.editorWebsiteRemove(number: String((model.urls.firstIndex(where: { $0.id == entry.id }) ?? 0) + 1)))
            }
        }
        Button(L10n.editorWebsiteAdd()) {
            model.urls.append(UrlEntry(url: "", matchEtld1: false, allowHttp: false, isApp: false))
        }
        .buttonStyle(SecondaryButtonStyle())
        .accessibilityIdentifier("editor-website-add")
        NotesField(text: $model.notes, label: L10n.editorNotes(), identifier: "editor-notes")
        otpSection
        OptionRow(text: L10n.editorFavorite(), isOn: $model.favorite, identifier: "editor-favorite")
        if let code = model.errorCode {
            ErrorLine(text: ErrorText.message(forCode: code))
        }
        if model.busy {
            HintText(text: L10n.editorSaving())
        }
        Button(L10n.commonSave()) {
            Task {
                if let key = await model.save() { onSaved(key) }
            }
        }
        .buttonStyle(PrimaryButtonStyle())
        .disabled(!model.canSave)
        .accessibilityIdentifier("editor-save")
    }

    @ViewBuilder
    private var otpSection: some View {
        SectionTitle(text: L10n.editorOtpTitle())
        if model.pendingId != nil {
            BodyText(text: L10n.editorOtpReady(name: model.pendingName))
        } else if model.hasOtp && !model.otpRemoved {
            BodyText(text: L10n.editorOtpHave())
            Button(L10n.editorOtpRemove()) { model.removeOtp() }
                .buttonStyle(SecondaryButtonStyle())
        } else if model.otpRemoved {
            BodyText(text: L10n.editorOtpRemoved())
        }
        if model.pendingId == nil {
            if model.scanning, let scan = model.scanModel {
                ScanBody(model: scan, sceneActive: app.isSceneActive)
                Button(L10n.commonCancel()) { model.stopScanning() }
                    .buttonStyle(SecondaryButtonStyle())
                    .accessibilityIdentifier("editor-scan-cancel")
            } else {
                SecretField(
                    label: L10n.editorOtpInput(), buffer: model.otpText, identifier: "editor-otp-input",
                    clearToken: model.otpClearToken, onChange: { empty in model.otpFilled = !empty },
                    onSubmit: { Task { await model.addOtpFromText() } })
                Button(L10n.editorOtpAdd()) { Task { await model.addOtpFromText() } }
                    .buttonStyle(SecondaryButtonStyle())
                    .disabled(!model.otpFilled || model.busy)
                    .accessibilityIdentifier("editor-otp-add")
                Button(L10n.scanOtpAction()) { model.startScanning() }
                    .buttonStyle(SecondaryButtonStyle())
                    .disabled(model.busy)
                    .accessibilityIdentifier("editor-otp-scan")
            }
        }
    }
}

struct NoteEditorView: View {
    @Bindable var model: NoteEditorModel
    let onSaved: (ItemKey) -> Void

    var body: some View {
        EditorScreen {
            if !model.loaded {
                if let code = model.errorCode {
                    ErrorLine(text: ErrorText.message(forCode: code))
                } else {
                    HintText(text: L10n.commonWorking())
                }
            } else {
                PlainField(
                    label: L10n.editorTitle(), placeholder: "", text: $model.title, identifier: "editor-title")
                NotesField(text: $model.notes, label: L10n.editorNotes(), identifier: "editor-notes")
                OptionRow(text: L10n.editorFavorite(), isOn: $model.favorite, identifier: "editor-favorite")
                if let code = model.errorCode {
                    ErrorLine(text: ErrorText.message(forCode: code))
                }
                if model.busy {
                    HintText(text: L10n.editorSaving())
                }
                Button(L10n.commonSave()) {
                    Task {
                        if let key = await model.save() { onSaved(key) }
                    }
                }
                .buttonStyle(PrimaryButtonStyle())
                .disabled(!model.canSave)
                .accessibilityIdentifier("editor-save")
            }
        }
        .navigationTitle(model.isNew ? L10n.editorNewNote() : L10n.editorEditNote())
        .navigationBarTitleDisplayMode(.inline)
        .task { await model.load() }
    }
}

/// The generator panel: mode, options, the result with its strength, and "use".
private struct GeneratorPanel: View {
    @Environment(\.colorScheme) private var colorScheme
    @Bindable var model: GeneratorModel
    let holder: CoreHolder
    let onUse: @MainActor () -> Void

    var body: some View {
        let colors = colorScheme.mosaic
        Panel {
            SectionTitle(text: L10n.generatorTitle())
            Picker(L10n.generatorTitle(), selection: $model.passphrase) {
                Text(L10n.generatorPassword()).tag(false)
                Text(L10n.generatorPassphrase()).tag(true)
            }
            .pickerStyle(.segmented)
            .accessibilityIdentifier("generator-mode")
            if model.passphrase {
                BodyText(text: L10n.generatorWordCount(count: String(Int(model.words))))
                Slider(value: $model.words, in: 3...10, step: 1)
                    .accessibilityLabel(L10n.generatorWordCount(count: String(Int(model.words))))
                PlainField(
                    label: L10n.generatorSeparator(), placeholder: "", text: $model.separator,
                    identifier: "generator-separator")
                OptionRow(text: L10n.generatorCapitalize(), isOn: $model.capitalize, identifier: "generator-capitalize")
                OptionRow(text: L10n.generatorAppendDigit(), isOn: $model.appendDigit, identifier: "generator-digit")
            } else {
                BodyText(text: L10n.generatorLength(length: String(Int(model.length))))
                Slider(value: $model.length, in: 8...64, step: 1)
                    .accessibilityLabel(L10n.generatorLength(length: String(Int(model.length))))
                OptionRow(text: L10n.generatorLowercase(), isOn: $model.lowercase, identifier: "generator-lower")
                OptionRow(text: L10n.generatorUppercase(), isOn: $model.uppercase, identifier: "generator-upper")
                OptionRow(text: L10n.generatorDigits(), isOn: $model.digits, identifier: "generator-digits")
                OptionRow(text: L10n.generatorSymbols(), isOn: $model.symbols, identifier: "generator-symbols")
                OptionRow(
                    text: L10n.generatorAmbiguous(), isOn: $model.excludeAmbiguous, identifier: "generator-ambiguous")
            }
            Button(L10n.generatorRun()) { Task { await model.generate(holder: holder) } }
                .buttonStyle(SecondaryButtonStyle())
                .accessibilityIdentifier("generator-run")
            if let code = model.errorCode {
                ErrorLine(text: ErrorText.message(forCode: code))
            }
            if let shown = model.preview {
                Text(shown)
                    .font(Font.mosaicMono(MosaicTokens.FontSize.md))
                    .foregroundStyle(colors.text)
                    .privacySensitive()
                    .accessibilityIdentifier("generator-preview")
                HintText(text: L10n.generatorStrength(bits: String(model.bits)))
                Button(L10n.generatorUse(), action: onUse)
                    .buttonStyle(PrimaryButtonStyle())
                    .accessibilityIdentifier("generator-use")
            }
        }
    }
}
