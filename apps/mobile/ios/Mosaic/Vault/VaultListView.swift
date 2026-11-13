import MosaicKit
import SwiftUI

/// One item: an initial in a tile, the title, a second line and a favorite mark.
struct ItemRow: View {
    @Environment(\.colorScheme) private var colorScheme
    let summary: ItemSummary

    var body: some View {
        let colors = colorScheme.mosaic
        let title = summary.title.isEmpty ? L10n.vaultUntitled() : summary.title
        let second = summary.subtitle ?? summary.primaryHost ?? FieldLabels.kind(summary.kind)
        HStack(spacing: MosaicTokens.Space.s4) {
            Text(String(title.prefix(1)).uppercased())
                .font(Font.mosaic(MosaicTokens.FontSize.md, weight: .semibold))
                .foregroundStyle(colors.text)
                .frame(width: MosaicTokens.Size.controlLg, height: MosaicTokens.Size.controlLg)
                .background(colors.bgSelected, in: RoundedRectangle(cornerRadius: MosaicTokens.Radius.field))
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: MosaicTokens.Space.s1) {
                Text(title)
                    .font(Font.mosaic(MosaicTokens.FontSize.md, weight: .medium))
                    .foregroundStyle(colors.text)
                    .fixedSize(horizontal: false, vertical: true)
                Text(second)
                    .font(Font.mosaic(MosaicTokens.FontSize.sm))
                    .foregroundStyle(colors.textMuted)
                    .lineLimit(1)
            }
            Spacer(minLength: MosaicTokens.Space.s3)
            if summary.favorite {
                Image(systemName: "star.fill")
                    .foregroundStyle(colors.warning)
                    .accessibilityLabel(L10n.vaultFavorite())
            }
        }
        .frame(minHeight: 48)
        .contentShape(Rectangle())
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isButton)
        .accessibilityIdentifier("item-row")
    }
}

/// The vault tab: the items, the sync state, pull to refresh, and the add menu.
struct VaultListView: View {
    @Environment(\.colorScheme) private var colorScheme
    let app: AppModel
    let vault: VaultModel
    let onOpen: (ItemSummary) -> Void
    let onAdd: (_ login: Bool) -> Void

    var body: some View {
        let colors = colorScheme.mosaic
        ZStack {
            colors.bg.ignoresSafeArea()
            List {
                Section {
                    header
                        .listRowBackground(Color.clear)
                        .listRowSeparator(.hidden)
                }
                if let items = vault.items {
                    if items.isEmpty {
                        VStack(alignment: .leading, spacing: MosaicTokens.Space.s3) {
                            SectionTitle(text: L10n.vaultEmptyTitle())
                            BodyText(text: L10n.vaultEmptyBody(), muted: true)
                        }
                        .padding(.top, MosaicTokens.Space.s7)
                        .listRowBackground(Color.clear)
                        .listRowSeparator(.hidden)
                    } else {
                        ForEach(items, id: \.itemId) { summary in
                            Button {
                                onOpen(summary)
                            } label: {
                                ItemRow(summary: summary)
                            }
                            .buttonStyle(.plain)
                            .listRowBackground(Color.clear)
                            .listRowSeparatorTint(colors.divider)
                        }
                    }
                }
            }
            .listStyle(.plain)
            .scrollContentBackground(.hidden)
            .refreshable { await vault.refresh() }
        }
        .navigationTitle(L10n.vaultTitle())
        .navigationBarTitleDisplayMode(.large)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Menu(L10n.vaultAdd()) {
                    Button(L10n.vaultAddLogin()) { onAdd(true) }
                        .accessibilityIdentifier("add-login")
                    Button(L10n.vaultAddNote()) { onAdd(false) }
                        .accessibilityIdentifier("add-note")
                }
                .accessibilityIdentifier("vault-add")
            }
        }
    }

    @ViewBuilder
    private var header: some View {
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s2) {
            if let items = vault.items {
                HintText(text: L10n.vaultCount(count: items.count))
                    .accessibilityIdentifier("vault-count")
            }
            let syncText = SyncLine.text(for: vault.overview)
            if !syncText.isEmpty {
                HintText(text: syncText)
                    .accessibilityIdentifier("sync-line")
            }
            if let pending = vault.overview?.pendingUploads, pending > 0 {
                HintText(text: L10n.syncPending(count: Int(pending)))
            }
            if let code = vault.failureCode {
                ErrorLine(text: ErrorText.message(forCode: code))
            }
        }
    }
}

/// The search tab: the core searches titles, user names, URLs and tags. The query stays on this screen.
struct SearchView: View {
    @Environment(\.colorScheme) private var colorScheme
    let vault: VaultModel
    let onOpen: (ItemSummary) -> Void
    @State private var query = ""
    @State private var results: [ItemSummary] = []
    @State private var failureCode: String?

    var body: some View {
        let colors = colorScheme.mosaic
        ZStack {
            colors.bg.ignoresSafeArea()
            List {
                Section {
                    VStack(alignment: .leading, spacing: MosaicTokens.Space.s2) {
                        let trimmed = query.trimmingCharacters(in: .whitespaces)
                        if let code = failureCode {
                            ErrorLine(text: ErrorText.message(forCode: code))
                        }
                        if trimmed.isEmpty {
                            HintText(text: L10n.searchHint())
                        } else if results.isEmpty && failureCode == nil {
                            HintText(text: L10n.searchNone(query: trimmed))
                        }
                    }
                    .listRowBackground(Color.clear)
                    .listRowSeparator(.hidden)
                }
                ForEach(results, id: \.itemId) { summary in
                    Button {
                        onOpen(summary)
                    } label: {
                        ItemRow(summary: summary)
                    }
                    .buttonStyle(.plain)
                    .listRowBackground(Color.clear)
                    .listRowSeparatorTint(colors.divider)
                }
            }
            .listStyle(.plain)
            .scrollContentBackground(.hidden)
        }
        .navigationTitle(L10n.navSearch())
        .navigationBarTitleDisplayMode(.large)
        .searchable(text: $query, prompt: L10n.searchPlaceholder())
        .autocorrectionDisabled()
        .textInputAutocapitalization(.never)
        .task(id: SearchKey(query: query, version: vault.version)) {
            await run()
        }
    }

    private func run() async {
        let trimmed = query.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else {
            results = []
            failureCode = nil
            return
        }
        try? await Task.sleep(for: .milliseconds(150))
        guard !Task.isCancelled else { return }
        do {
            results = try await vault.search(trimmed)
            failureCode = nil
        } catch {
            failureCode = ErrorText.code(of: error)
        }
    }
}

private struct SearchKey: Equatable {
    let query: String
    let version: Int
}
